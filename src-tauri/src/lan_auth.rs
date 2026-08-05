//! SEC-002 LAN borrow server authentication — token core.
//!
//! Provides a backend-owned, memory-only authorization token for the LAN borrow
//! HTTP server. Completely separate from SEC-001's `SessionStore` — no shared
//! state, keys, or types.
//!
//! ## Design
//!
//! - One active 256-bit opaque token from `OsRng` at a time.
//! - Stored as raw `[u8; 32]` bytes in a `Mutex<Option<[u8; 32]>>`.
//! - No expiry, no HMAC, no embedded identity/claims/metadata.
//! - Verification: decode base64url → compare `[u8; 32]` using constant-time XOR.
//! - Regenerate replaces and invalidates the previous token.
//! - Revoke removes the active token (all subsequent verifies fail).
//! - App restart naturally clears it (process memory only).
//! - Never persisted to DB, config, filesystem, logs, URLs outside QR, or
//!   frontend storage.
//! - Token values are never exposed via `Debug`, `Display`, serialization, or
//!   error messages.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rand::rngs::OsRng;
use rand::RngCore;

// ── Constants ────────────────────────────────────────────────────────────────

/// Token entropy in bytes (256 bits).
const TOKEN_BYTES: usize = 32;

/// Encoded token length (base64url-no-pad of 32 bytes = 43 chars).
const TOKEN_ENCODED_LEN: usize = 43;

/// Replay-cache entry TTL.
const REPLAY_TTL: Duration = Duration::from_secs(5 * 60);

/// Sliding-window duration for rate limiting.
const RATE_WINDOW: Duration = Duration::from_secs(60);

// ── Public types ─────────────────────────────────────────────────────────────

/// In-memory store for the single active LAN borrow token.
///
/// Holds one active token at a time. `issue()` replaces any existing token.
/// `revoke()` clears it. `verify()` performs constant-time comparison.
/// All state is process-local; nothing is persisted.
pub struct LanTokenStore {
    active_token: Mutex<Option<[u8; TOKEN_BYTES]>>,
    replay_cache: Mutex<HashMap<String, Instant>>,
    rate_counters: Mutex<HashMap<RateKey, RateWindow>>,
}

/// Opaque marker proving the caller passed token verification.
/// Contains no identity, role, or metadata.
#[derive(Debug)]
pub struct LanTokenContext;

/// Rate-limit key: composite of peer IP and endpoint group.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RateKey {
    pub key: String,
}

/// Sliding-window counter for one rate-limit key.
struct RateWindow {
    entries: Vec<Instant>,
}

/// Error returned by LAN auth operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanAuthError {
    /// No token in the Authorization header or `?t=`/`#t=` parameter.
    Missing,
    /// Token string is malformed (wrong length, invalid base64url).
    Malformed,
    /// Token does not match the active token, or no active token exists.
    Invalid,
    /// Duplicate requestId within the replay window.
    #[allow(dead_code)]
    DuplicateRequest,
    /// Rate limit exceeded for this endpoint.
    RateLimited,
}

impl LanAuthError {
    /// Stable machine-readable error code for HTTP responses.
    pub fn code(&self) -> &'static str {
        match self {
            LanAuthError::Missing => "LAN_AUTH_REQUIRED",
            LanAuthError::Malformed => "LAN_AUTH_REQUIRED",
            LanAuthError::Invalid => "LAN_AUTH_REQUIRED",
            LanAuthError::DuplicateRequest => "LAN_DUPLICATE_REQUEST",
            LanAuthError::RateLimited => "LAN_RATE_LIMITED",
        }
    }
}

impl std::fmt::Display for LanAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for LanAuthError {}

// ── LanTokenStore implementation ────────────────────────────────────────────

impl Default for LanTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

impl LanTokenStore {
    /// Create an empty store (no active token).
    pub fn new() -> Self {
        Self {
            active_token: Mutex::new(None),
            replay_cache: Mutex::new(HashMap::new()),
            rate_counters: Mutex::new(HashMap::new()),
        }
    }

    /// Generate a new 256-bit opaque token from `OsRng`.
    /// Replaces and invalidates any previously issued token.
    /// Returns the base64url-no-pad encoded string for QR embedding.
    pub fn issue(&self) -> String {
        let mut bytes = [0u8; TOKEN_BYTES];
        OsRng.fill_bytes(&mut bytes);
        if let Ok(mut guard) = self.active_token.lock() {
            *guard = Some(bytes);
        }
        encode_url_safe_no_pad(&bytes)
    }

    /// Remove the active token. All subsequent `verify` calls fail.
    pub fn revoke(&self) {
        if let Ok(mut guard) = self.active_token.lock() {
            *guard = None;
        }
    }

    /// Return whether an active token is currently issued.
    pub fn is_ready(&self) -> bool {
        self.active_token
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    /// Verify a token string against the active token.
    ///
    /// Decodes the base64url string to 32 bytes, then compares using
    /// constant-time XOR. Returns `Ok(LanTokenContext)` on match.
    ///
    /// Errors:
    /// - `Missing` if the input is empty.
    /// - `Malformed` if the input is not valid base64url or wrong length.
    /// - `Invalid` if no active token exists or the bytes don't match.
    pub fn verify(&self, encoded: &str) -> Result<LanTokenContext, LanAuthError> {
        if encoded.is_empty() {
            return Err(LanAuthError::Missing);
        }
        if encoded.len() != TOKEN_ENCODED_LEN {
            return Err(LanAuthError::Malformed);
        }
        let decoded = decode_url_safe_no_pad(encoded)?;
        if decoded.len() != TOKEN_BYTES {
            return Err(LanAuthError::Malformed);
        }
        let mut candidate = [0u8; TOKEN_BYTES];
        candidate.copy_from_slice(&decoded);

        let guard = self
            .active_token
            .lock()
            .map_err(|_| LanAuthError::Invalid)?;
        match *guard {
            None => Err(LanAuthError::Invalid),
            Some(ref active) => {
                if constant_time_eq(&candidate, active) {
                    Ok(LanTokenContext)
                } else {
                    Err(LanAuthError::Invalid)
                }
            }
        }
    }

    /// Check and record a `requestId` for replay protection.
    ///
    /// Atomically checks whether `request_id` has been seen within the TTL
    /// window and inserts it if not. Two concurrent calls with the same
    /// `request_id` are guaranteed to result in exactly one `Ok(())` and
    /// one `Err(DuplicateRequest)` — the `HashMap::entry` API provides
    /// atomic lookup-and-insert semantics under the mutex lock.
    ///
    /// **Phase-B wiring contract:** call this only after request syntax,
    /// token, employee, asset, and business validation pass, but before
    /// DB mutation — so a validation failure does not consume the requestId.
    ///
    /// Returns `Ok(())` on first sighting within the TTL window.
    /// Returns `DuplicateRequest` if the same `requestId` was seen recently.
    pub fn check_and_record_request(&self, request_id: &str) -> Result<(), LanAuthError> {
        if request_id.is_empty() {
            return Err(LanAuthError::Malformed);
        }
        let now = Instant::now();
        let mut cache = self
            .replay_cache
            .lock()
            .map_err(|_| LanAuthError::Invalid)?;

        // Prune expired entries to keep memory bounded.
        cache.retain(|_, ts| now.duration_since(*ts) < REPLAY_TTL);

        // Atomic lookup-and-insert: entry() checks existence and reserves the
        // slot in one operation under the lock. If the key already exists
        // (whether live or expired-but-not-yet-pruned), we reject. If the
        // key is absent, we insert and accept. No other thread can observe
        // a state between "checked" and "inserted."
        match cache.entry(request_id.to_string()) {
            std::collections::hash_map::Entry::Occupied(_) => Err(LanAuthError::DuplicateRequest),
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(now);
                Ok(())
            }
        }
    }

    /// Check rate limit for a composite key (peer IP + endpoint group).
    ///
    /// Returns `Ok(())` if under the limit, `RateLimited` if exceeded.
    /// Uses a compact keyed sliding-window: one `RateWindow` per key,
    /// holding `Instant` stamps pruned to the 60-second window.
    pub fn check_rate_limit(&self, key: &str, max_per_window: u32) -> Result<(), LanAuthError> {
        let now = Instant::now();
        let rk = RateKey {
            key: key.to_string(),
        };
        let mut counters = self
            .rate_counters
            .lock()
            .map_err(|_| LanAuthError::Invalid)?;

        // Get or create the window for this endpoint.
        let window = counters.entry(rk).or_insert_with(|| RateWindow {
            entries: Vec::new(),
        });

        // Prune expired entries within this window.
        window
            .entries
            .retain(|ts| now.duration_since(*ts) < RATE_WINDOW);

        // Check limit.
        if window.entries.len() >= max_per_window as usize {
            return Err(LanAuthError::RateLimited);
        }

        // Record this request.
        window.entries.push(now);
        Ok(())
    }
}

// ── Constant-time comparison ────────────────────────────────────────────────

/// Compare two byte slices of equal length using a XOR accumulator.
///
/// This function runs in time proportional to the slice length and does not
/// short-circuit on the first differing byte. The length check at the top is
/// a single branch that leaks only whether the lengths match — for fixed
/// 32-byte tokens this is always true for valid inputs.
///
/// **Not a substitute for a formally verified constant-time implementation**,
/// but sufficient defense-in-depth for a LAN bearer token.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ── Base64-URL encoding/decoding (no external dependency) ────────────────────

/// Encode 32 bytes as 43-char base64url (no padding).
fn encode_url_safe_no_pad(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(TOKEN_ENCODED_LEN);
    let mut chunks = bytes.chunks_exact(3);
    for chunk in &mut chunks {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        out.push(TABLE[(n & 0x3F) as usize] as char);
    }
    let rem = chunks.remainder();
    match rem.len() {
        1 => {
            let n = (rem[0] as u32) << 16;
            out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
            out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        }
        2 => {
            let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
            out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
            out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
            out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        }
        _ => {}
    }
    out
}

/// Decode a base64url-no-pad string into raw bytes.
///
/// Returns `Err(LanAuthError::Malformed)` for invalid characters or unexpected
/// length. Does not allocate on the hot path for valid 43-char inputs (returns
/// a fixed-size buffer via `Vec`).
fn decode_url_safe_no_pad(encoded: &str) -> Result<Vec<u8>, LanAuthError> {
    const DECODE_TABLE: [i8; 128] = {
        let mut t = [-1i8; 128];
        let mut i = 0u8;
        while i < 26 {
            t[(b'A' + i) as usize] = i as i8;
            i += 1;
        }
        let mut i = 0u8;
        while i < 26 {
            t[(b'a' + i) as usize] = (26 + i) as i8;
            i += 1;
        }
        let mut i = 0u8;
        while i < 10 {
            t[(b'0' + i) as usize] = (52 + i) as i8;
            i += 1;
        }
        t[b'-' as usize] = 62;
        t[b'_' as usize] = 63;
        t
    };

    let bytes = encoded.as_bytes();
    // Reject any byte outside the URL-safe alphabet.
    for &b in bytes {
        if b >= 128 || DECODE_TABLE[b as usize] < 0 {
            return Err(LanAuthError::Malformed);
        }
    }

    let mut out = Vec::with_capacity(bytes.len() * 3 / 4 + 1);
    let mut i = 0;
    while i + 4 <= bytes.len() {
        let v0 = DECODE_TABLE[bytes[i] as usize] as u32;
        let v1 = DECODE_TABLE[bytes[i + 1] as usize] as u32;
        let v2 = DECODE_TABLE[bytes[i + 2] as usize] as u32;
        let v3 = DECODE_TABLE[bytes[i + 3] as usize] as u32;
        let n = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
        out.push((n >> 16) as u8);
        out.push((n >> 8) as u8);
        out.push(n as u8);
        i += 4;
    }
    let rem = &bytes[i..];
    match rem.len() {
        2 => {
            let v0 = DECODE_TABLE[rem[0] as usize] as u32;
            let v1 = DECODE_TABLE[rem[1] as usize] as u32;
            let n = (v0 << 18) | (v1 << 12);
            out.push((n >> 16) as u8);
        }
        3 => {
            let v0 = DECODE_TABLE[rem[0] as usize] as u32;
            let v1 = DECODE_TABLE[rem[1] as usize] as u32;
            let v2 = DECODE_TABLE[rem[2] as usize] as u32;
            let n = (v0 << 18) | (v1 << 12) | (v2 << 6);
            out.push((n >> 16) as u8);
            out.push((n >> 8) as u8);
        }
        0 => {}
        _ => return Err(LanAuthError::Malformed),
    }
    Ok(out)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Token generation ─────────────────────────────────────────────────────

    #[test]
    fn token_is_32_bytes_256_bits() {
        let store = LanTokenStore::new();
        let encoded = store.issue();
        let decoded = decode_url_safe_no_pad(&encoded).expect("valid base64url");
        assert_eq!(decoded.len(), TOKEN_BYTES, "decoded token must be 32 bytes");
        assert_eq!(TOKEN_BYTES * 8, 256, "256 bits");
    }

    #[test]
    fn encoded_token_is_url_safe_and_correct_length() {
        let store = LanTokenStore::new();
        let encoded = store.issue();
        assert_eq!(encoded.len(), TOKEN_ENCODED_LEN, "43 base64url chars");
        for b in encoded.bytes() {
            assert!(
                b.is_ascii_alphanumeric() || b == b'-' || b == b'_',
                "token contains non-URL-safe char"
            );
        }
        assert!(!encoded.contains('='), "no padding");
        assert!(!encoded.contains('+'), "no standard base64 chars");
        assert!(!encoded.contains('/'), "no standard base64 chars");
    }

    #[test]
    fn encoded_token_contains_no_embedded_metadata() {
        // The token is pure random bytes encoded as base64url.
        // Verify it doesn't contain any recognizable structure.
        let store = LanTokenStore::new();
        let encoded = store.issue();
        // Should not contain JSON, timestamps, role strings, etc.
        assert!(!encoded.contains("admin"));
        assert!(!encoded.contains("user"));
        assert!(!encoded.contains("role"));
        assert!(!encoded.contains("exp"));
        assert!(!encoded.contains("iat"));
        assert!(!encoded.contains("{"));
        assert!(!encoded.contains("}"));
        assert!(!encoded.contains("."));
    }

    #[test]
    fn generated_tokens_are_unique_across_repeated_generation() {
        let store = LanTokenStore::new();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..256 {
            let token = store.issue();
            assert!(seen.insert(token), "duplicate token generated");
        }
        assert_eq!(seen.len(), 256);
    }

    // ── Token verification ───────────────────────────────────────────────────

    #[test]
    fn valid_token_accepted() {
        let store = LanTokenStore::new();
        let token = store.issue();
        assert!(store.verify(&token).is_ok());
    }

    #[test]
    fn empty_token_rejected_as_missing() {
        let store = LanTokenStore::new();
        store.issue();
        let err = store.verify("").unwrap_err();
        assert_eq!(err, LanAuthError::Missing);
    }

    #[test]
    fn malformed_token_rejected() {
        let store = LanTokenStore::new();
        store.issue();
        // Wrong length (too short).
        assert_eq!(store.verify("abc").unwrap_err(), LanAuthError::Malformed);
        // Wrong length (too long).
        let long = "A".repeat(100);
        assert_eq!(store.verify(&long).unwrap_err(), LanAuthError::Malformed);
        // Invalid base64url character.
        let bad = "A".repeat(42) + "+";
        assert_eq!(store.verify(&bad).unwrap_err(), LanAuthError::Malformed);
    }

    #[test]
    fn mismatched_token_rejected() {
        let store = LanTokenStore::new();
        store.issue();
        // A valid-format but wrong token.
        let wrong = encode_url_safe_no_pad(&[0xFFu8; TOKEN_BYTES]);
        assert_eq!(store.verify(&wrong).unwrap_err(), LanAuthError::Invalid);
    }

    #[test]
    fn mismatch_at_first_byte_rejected() {
        let store = LanTokenStore::new();
        let token = store.issue();
        let mut decoded = decode_url_safe_no_pad(&token).expect("decode");
        decoded[0] ^= 0x01;
        let tampered = encode_url_safe_no_pad(&decoded);
        assert_eq!(store.verify(&tampered).unwrap_err(), LanAuthError::Invalid);
    }

    #[test]
    fn mismatch_at_middle_byte_rejected() {
        let store = LanTokenStore::new();
        let token = store.issue();
        let mut decoded = decode_url_safe_no_pad(&token).expect("decode");
        decoded[TOKEN_BYTES / 2] ^= 0x01;
        let tampered = encode_url_safe_no_pad(&decoded);
        assert_eq!(store.verify(&tampered).unwrap_err(), LanAuthError::Invalid);
    }

    #[test]
    fn mismatch_at_last_byte_rejected() {
        let store = LanTokenStore::new();
        let token = store.issue();
        let mut decoded = decode_url_safe_no_pad(&token).expect("decode");
        decoded[TOKEN_BYTES - 1] ^= 0x01;
        let tampered = encode_url_safe_no_pad(&decoded);
        assert_eq!(store.verify(&tampered).unwrap_err(), LanAuthError::Invalid);
    }

    // ── Regeneration and revocation ──────────────────────────────────────────

    #[test]
    fn regeneration_invalidates_old_token() {
        let store = LanTokenStore::new();
        let old = store.issue();
        assert!(store.verify(&old).is_ok());

        let new = store.issue();
        assert!(store.verify(&new).is_ok());
        assert_eq!(
            store.verify(&old).unwrap_err(),
            LanAuthError::Invalid,
            "old token must be invalid after regeneration"
        );
        assert_ne!(old, new, "new token must differ from old");
    }

    #[test]
    fn revoke_invalidates_token() {
        let store = LanTokenStore::new();
        let token = store.issue();
        assert!(store.verify(&token).is_ok());

        store.revoke();

        assert_eq!(
            store.verify(&token).unwrap_err(),
            LanAuthError::Invalid,
            "token must be invalid after revoke"
        );
    }

    #[test]
    fn revoke_on_empty_store_is_noop() {
        let store = LanTokenStore::new();
        store.revoke(); // should not panic
        assert_eq!(
            store.verify("x".repeat(43).as_str()).unwrap_err(),
            LanAuthError::Invalid
        );
    }

    #[test]
    fn readiness_tracks_issue_and_revoke_without_exposing_token() {
        let store = LanTokenStore::new();
        assert!(!store.is_ready());
        let token = store.issue();
        assert!(store.is_ready());
        assert!(store.verify(&token).is_ok());
        store.revoke();
        assert!(!store.is_ready());
    }

    #[test]
    fn verify_on_empty_store_returns_invalid() {
        let store = LanTokenStore::new();
        let fake = encode_url_safe_no_pad(&[0u8; TOKEN_BYTES]);
        assert_eq!(store.verify(&fake).unwrap_err(), LanAuthError::Invalid);
    }

    // ── Token hygiene: no exposure ───────────────────────────────────────────

    #[test]
    fn token_value_absent_from_lan_auth_error_display() {
        let store = LanTokenStore::new();
        let token = store.issue();
        let err = store.verify("wrong-format").unwrap_err();
        let display = format!("{err}");
        let debug = format!("{err:?}");
        assert!(!display.contains(&token), "token leaked in Display");
        assert!(!debug.contains(&token), "token leaked in Debug");
    }

    #[test]
    fn constant_time_eq_rejects_different_lengths() {
        assert!(!constant_time_eq(&[1, 2, 3], &[1, 2]));
        assert!(!constant_time_eq(&[1], &[1, 2, 3]));
    }

    #[test]
    fn constant_time_eq_accepts_identical_slices() {
        assert!(constant_time_eq(&[0u8; 32], &[0u8; 32]));
        assert!(constant_time_eq(&[0xFFu8; 32], &[0xFFu8; 32]));
    }

    #[test]
    fn constant_time_eq_rejects_single_bit_difference() {
        let a = [0u8; 32];
        let mut b = [0u8; 32];
        b[15] ^= 0x01;
        assert!(!constant_time_eq(&a, &b));
    }

    // ── Replay cache (Phase A unit tests — no wiring) ────────────────────────

    #[test]
    fn replay_cache_accepts_first_sighting() {
        let store = LanTokenStore::new();
        assert!(store.check_and_record_request("req-001").is_ok());
    }

    #[test]
    fn replay_cache_rejects_duplicate() {
        let store = LanTokenStore::new();
        store.check_and_record_request("req-001").unwrap();
        assert_eq!(
            store.check_and_record_request("req-001").unwrap_err(),
            LanAuthError::DuplicateRequest
        );
    }

    #[test]
    fn replay_cache_rejects_empty_request_id() {
        let store = LanTokenStore::new();
        assert_eq!(
            store.check_and_record_request("").unwrap_err(),
            LanAuthError::Malformed
        );
    }

    // ── Rate limiter (Phase A unit tests — no wiring) ───────────────────────

    #[test]
    fn rate_limiter_allows_under_limit() {
        let store = LanTokenStore::new();
        for _ in 0..5 {
            assert!(store.check_rate_limit("127.0.0.1:test-endpoint", 5).is_ok());
        }
    }

    #[test]
    fn rate_limiter_rejects_over_limit() {
        let store = LanTokenStore::new();
        for _ in 0..3 {
            store
                .check_rate_limit("127.0.0.1:test-endpoint", 3)
                .unwrap();
        }
        assert_eq!(
            store
                .check_rate_limit("127.0.0.1:test-endpoint", 3)
                .unwrap_err(),
            LanAuthError::RateLimited
        );
    }

    #[test]
    fn rate_limiter_independent_per_endpoint() {
        let store = LanTokenStore::new();
        for _ in 0..3 {
            store.check_rate_limit("127.0.0.1:search", 3).unwrap();
        }
        // Different endpoint should still be allowed.
        assert!(store.check_rate_limit("127.0.0.1:submit", 3).is_ok());
    }

    // ── Concurrent replay atomicity ──────────────────────────────────────────

    #[test]
    fn concurrent_same_request_id_exactly_one_succeeds() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let store = Arc::new(LanTokenStore::new());
        let barrier = Arc::new(Barrier::new(2));
        let request_id = "concurrent-req-001";

        let store_a = Arc::clone(&store);
        let barrier_a = Arc::clone(&barrier);
        let handle_a = thread::spawn(move || {
            barrier_a.wait();
            store_a.check_and_record_request(request_id)
        });

        let store_b = Arc::clone(&store);
        let barrier_b = Arc::clone(&barrier);
        let handle_b = thread::spawn(move || {
            barrier_b.wait();
            store_b.check_and_record_request(request_id)
        });

        let result_a = handle_a.join().expect("thread A must not panic");
        let result_b = handle_b.join().expect("thread B must not panic");

        // Exactly one Ok, exactly one DuplicateRequest.
        let successes = [&result_a, &result_b].iter().filter(|r| r.is_ok()).count();
        let duplicates = [&result_a, &result_b]
            .iter()
            .filter(|r| matches!(r, Err(LanAuthError::DuplicateRequest)))
            .count();

        assert_eq!(successes, 1, "exactly one thread must succeed");
        assert_eq!(
            duplicates, 1,
            "exactly one thread must be rejected as replay"
        );

        // Store is not poisoned — subsequent calls still work.
        assert!(store.check_and_record_request("after-concurrent").is_ok());
    }
}
