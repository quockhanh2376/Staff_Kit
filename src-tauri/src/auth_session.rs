//! Backend-owned session authorization for Tauri commands (SEC-001).
//!
//! Sessions live entirely in process memory (`SessionStore`). A login mints an
//! opaque 128-bit token that the frontend holds only in runtime memory; the
//! token is never persisted to the database, filesystem, `localStorage`,
//! config files, URLs, logs, or error messages. Every guarded command resolves
//! the supplied token into a backend-created [`SessionContext`] and rejects
//! requests that fail authentication or authorization.
//!
//! Design rules enforced here:
//! - Tokens are unforgeable random values; the frontend cannot mint or guess them.
//! - `account_id`, `account_key`, and `role` are captured at login time and
//!   frozen in the entry. Frontend-supplied identity values are never trusted.
//! - Locks are held only for the cheap HashMap lookup; all database, filesystem,
//!   async, or slow I/O happens after the lock is dropped.
//! - No `unsafe` code. Poisoned mutexes are recovered rather than panicking.
//! - No database schema change.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rand::rngs::OsRng;
use rand::RngCore;

// ── Expiry configuration ─────────────────────────────────────────────────────

/// Idle timeout: a session unused for this long is expired.
pub const SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(8 * 60 * 60); // 8 hours

/// Absolute session lifetime, measured from issuance. Refreshing `last_seen_at`
/// never extends this cap.
pub const SESSION_ABSOLUTE_LIFETIME: Duration = Duration::from_secs(12 * 60 * 60); // 12 hours

// ── Stable, machine-readable auth error codes ────────────────────────────────

/// No session token was supplied, or the token is unknown to this process.
pub const AUTH_REQUIRED: &str = "AUTH_REQUIRED";

/// The session exists but has expired (idle or absolute).
pub const AUTH_SESSION_EXPIRED: &str = "AUTH_SESSION_EXPIRED";

/// The session is valid but lacks the role required for the command.
pub const AUTH_FORBIDDEN: &str = "AUTH_FORBIDDEN";

/// The actor attempted a self-referential operation that is not permitted, e.g.
/// deleting the account currently signed in. The session is left intact.
pub const AUTH_CANNOT_DELETE_SELF: &str = "AUTH_CANNOT_DELETE_SELF";

/// Error returned by guards. The string form is the stable code above, which
/// the frontend matches centrally. No session token or account secret is ever
/// placed in the error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthError(pub &'static str);

impl AuthError {
    pub fn code(&self) -> &'static str {
        self.0
    }
}

/// Allow `AuthError` to flow through `?` in `Result<_, String>` command bodies.
/// The conversion yields only the stable code string (never a token or secret).
impl From<AuthError> for String {
    fn from(err: AuthError) -> Self {
        err.0.to_string()
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for AuthError {}

// ── Role ─────────────────────────────────────────────────────────────────────

/// Account roles matching the strings stored in `app_local_accounts.role`:
/// `"super_admin"`, `"admin"`, `"user"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Admin,
    SuperAdmin,
}

impl Role {
    /// Parse a role from the raw DB string. Unknown values fall back to the
    /// least-privileged role (`User`) so a corrupt role field can never grant
    /// elevated access.
    pub fn from_db_str(raw: &str) -> Self {
        match raw {
            "super_admin" => Role::SuperAdmin,
            "admin" => Role::Admin,
            _ => Role::User,
        }
    }

    /// `true` if this role satisfies the `authenticated` guard (any login).
    pub fn at_least_user(self) -> bool {
        matches!(self, Role::User | Role::Admin | Role::SuperAdmin)
    }

    /// `true` if this role satisfies the `admin` guard.
    pub fn at_least_admin(self) -> bool {
        matches!(self, Role::Admin | Role::SuperAdmin)
    }

    /// `true` if this role satisfies the `super_admin` guard.
    pub fn is_super_admin(self) -> bool {
        matches!(self, Role::SuperAdmin)
    }
}

// ── Clock abstraction (for deterministic expiry tests, no sleeping) ──────────

/// Injectable monotonic clock so expiry tests run instantly.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

/// Production clock backed by `Instant::now()`.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Shared, controllable clock for tests. The same handle can be handed to both
/// the `SessionStore` and the test harness, so `advance` is observed by the
/// store immediately.
pub struct TestClock {
    start: Arc<Mutex<Instant>>,
}

impl TestClock {
    pub fn new() -> Self {
        Self {
            start: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Clone the shared handle so both the store and the test share state.
    pub fn handle(&self) -> Self {
        Self {
            start: Arc::clone(&self.start),
        }
    }

    /// Advance the virtual clock by `delta`. Saturating — never underflows.
    pub fn advance(&self, delta: Duration) {
        if let Ok(mut guard) = self.start.lock() {
            *guard = guard.checked_add(delta).unwrap_or(*guard);
        }
    }
}

impl Default for TestClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for TestClock {
    fn now(&self) -> Instant {
        self.start
            .lock()
            .map(|guard| *guard)
            .unwrap_or_else(|poisoned| *poisoned.into_inner())
    }
}

// ── Opaque token (no Debug/Display/Serialize leakage) ────────────────────────

/// Opaque session token. The inner value is intentionally never printed,
/// serialized, or compared by value in user-facing code. The `Debug`
/// implementation is redacted so tokens cannot leak via `dbg!()`/`format!`.
#[derive(Clone)]
struct OpaqueToken(String);

impl OpaqueToken {
    fn new(value: String) -> Self {
        Self(value)
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for OpaqueToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OpaqueToken(<redacted>)")
    }
}

impl PartialEq for OpaqueToken {
    fn eq(&self, other: &Self) -> bool {
        // Constant-time-ish comparison to avoid timing side channels on length-equal
        // tokens. This is defense-in-depth; the map key equality is the real gate.
        if self.0.len() != other.0.len() {
            return false;
        }
        let mut diff: u8 = 0;
        for (a, b) in self.0.bytes().zip(other.0.bytes()) {
            diff |= a ^ b;
        }
        diff == 0
    }
}

impl Eq for OpaqueToken {}

impl std::hash::Hash for OpaqueToken {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

// ── Session record ───────────────────────────────────────────────────────────

/// Internal session record. `account_id`, `account_key`, and `role` are frozen
/// at issue time and never refreshed from the database thereafter; a role change
/// requires explicit invalidation (handled in Phase D).
#[derive(Clone)]
struct SessionEntry {
    account_id: i64,
    account_key: String,
    role: Role,
    issued_at: Instant,
    last_seen_at: Instant,
    absolute_expires_at: Instant,
}

/// Safe authorization identity handed to command bodies. Contains only the
/// fields commands need to attribute work — never the token or raw entry.
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub account_id: i64,
    pub account_key: String,
    pub role: Role,
}

// ── SessionStore ─────────────────────────────────────────────────────────────

/// In-memory session store. Held in `tauri::State` and never persisted.
pub struct SessionStore {
    entries: Mutex<HashMap<OpaqueToken, SessionEntry>>,
    clock: Box<dyn Clock>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            clock: Box::new(SystemClock),
        }
    }
}

impl SessionStore {
    /// Production constructor with the real monotonic clock.
    pub fn new() -> Self {
        Self::default()
    }

    /// Test constructor with a controllable clock.
    pub fn with_clock(clock: impl Clock + 'static) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            clock: Box::new(clock),
        }
    }

    /// Generate a fresh 128-bit opaque token, URL-safe base64 (no padding).
    /// Returns `(token_string, issued_at, absolute_expires_at)`.
    fn mint_token(&self) -> (String, Instant, Instant) {
        let mut bytes = [0u8; 16]; // 128 bits
        OsRng.fill_bytes(&mut bytes);
        let token = encode_url_safe_no_pad(&bytes);
        let issued_at = self.clock.now();
        let absolute_expires_at = issued_at + SESSION_ABSOLUTE_LIFETIME;
        (token, issued_at, absolute_expires_at)
    }

    /// Mint a session for `account_id`/`account_key`/`role`. The returned
    /// string is the only value the frontend ever sees; it must be held in
    /// runtime memory only.
    pub fn issue_session(&self, account_id: i64, account_key: &str, role: Role) -> String {
        let (token, issued_at, absolute_expires_at) = self.mint_token();
        let now = self.clock.now();
        let entry = SessionEntry {
            account_id,
            account_key: account_key.to_string(),
            role,
            issued_at,
            last_seen_at: now,
            absolute_expires_at,
        };
        let token_key = OpaqueToken::new(token.clone());
        let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        entries.insert(token_key, entry);
        token
    }

    /// Resolve a token to a [`SessionContext`], refreshing `last_seen_at` on
    /// success. Expired entries are removed. The lock is held only for the
    /// lookup + clone + removal — no I/O is performed under the lock.
    pub fn resolve_session(&self, token: &str) -> Result<SessionContext, AuthError> {
        let now = self.clock.now();
        let token_key = OpaqueToken::new(token.to_string());
        // Bounded critical section: lookup, expiry check, refresh, clone, drop.
        // A poisoned lock is recovered (not propagated) so a prior panic can never
        // wedge the entire auth subsystem; the recovered map serves requests.
        let outcome = {
            let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
            // Clone the entry out of the map first so the immutable borrow ends
            // before we mutate the map to refresh `last_seen_at`.
            let entry = match entries.remove(&token_key) {
                Some(entry) => entry,
                None => return Err(AuthError(AUTH_REQUIRED)),
            };
            let idle_expired = now.duration_since(entry.last_seen_at) >= SESSION_IDLE_TIMEOUT;
            let absolute_expired = now >= entry.absolute_expires_at;
            if absolute_expired || idle_expired {
                // Already removed above; nothing to reinsert.
                return Err(AuthError(AUTH_SESSION_EXPIRED));
            }
            // Refresh idle time without extending absolute expiry, then reinsert.
            let mut refreshed = entry.clone();
            refreshed.last_seen_at = now;
            entries.insert(token_key, refreshed);
            // Clone only the safe identity out of the critical section.
            SessionContext {
                account_id: entry.account_id,
                account_key: entry.account_key,
                role: entry.role,
            }
        };
        Ok(outcome)
    }

    /// Revoke a single token (logout).
    pub fn invalidate_token(&self, token: &str) {
        let token_key = OpaqueToken::new(token.to_string());
        let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        entries.remove(&token_key);
    }

    /// Revoke every session belonging to `account_id` (password change/reset,
    /// account deletion/disable, role change).
    pub fn invalidate_account(&self, account_id: i64) {
        let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        entries.retain(|_, entry| entry.account_id != account_id);
    }

    /// Revoke every session (logout-everyone / reset_all_data / DB restore).
    pub fn invalidate_all(&self) {
        let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        entries.clear();
    }

    /// Drop expired entries. Called opportunistically by resolve and at known
    /// lifecycle points; not required for correctness (resolve prunes lazily).
    pub fn prune_expired(&self) {
        let now = self.clock.now();
        let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        entries.retain(|_, entry| {
            let idle_expired = now.duration_since(entry.last_seen_at) >= SESSION_IDLE_TIMEOUT;
            let absolute_expired = now >= entry.absolute_expires_at;
            !(idle_expired || absolute_expired)
        });
    }

    /// Number of live (not pruned) sessions. Test-only convenience.
    #[allow(dead_code)]
    pub fn active_session_count(&self) -> usize {
        self.prune_expired();
        let entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        entries.len()
    }
}

// ── Guards ───────────────────────────────────────────────────────────────────

/// Idempotent logout helper: invalidate `token` if present. Used by the
/// `logout_local_account` command and is safe to call with an already-absent or
/// expired token (no-op). Exposed for direct use by command bodies and tests.
pub fn logout_via_store(store: &SessionStore, token: &str) {
    store.invalidate_token(token);
}

/// Require any authenticated session.
pub fn require_authenticated(
    store: &SessionStore,
    token: &str,
) -> Result<SessionContext, AuthError> {
    let ctx = store.resolve_session(token)?;
    if ctx.role.at_least_user() {
        Ok(ctx)
    } else {
        Err(AuthError(AUTH_FORBIDDEN))
    }
}

/// Require an `admin` or `super_admin` session.
pub fn require_admin(store: &SessionStore, token: &str) -> Result<SessionContext, AuthError> {
    let ctx = store.resolve_session(token)?;
    if ctx.role.at_least_admin() {
        Ok(ctx)
    } else {
        Err(AuthError(AUTH_FORBIDDEN))
    }
}

/// Require a `super_admin` session.
pub fn require_super_admin(store: &SessionStore, token: &str) -> Result<SessionContext, AuthError> {
    let ctx = store.resolve_session(token)?;
    if ctx.role.is_super_admin() {
        Ok(ctx)
    } else {
        Err(AuthError(AUTH_FORBIDDEN))
    }
}

// ── URL-safe base64 encoder (no external dependency) ──────────────────────────

/// Encode 16 bytes as 22-char URL-safe base64 without padding. Suitable for IPC
/// transport as a Tauri command argument (no `+`, `/`, or `=`).
fn encode_url_safe_no_pad(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    debug_assert_eq!(TABLE.len(), 64);
    let mut out = String::with_capacity(((bytes.len() + 2) / 3) * 4);
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

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn store() -> SessionStore {
        SessionStore::with_clock(TestClock::new())
    }

    fn issue(store: &SessionStore, role: Role) -> String {
        store.issue_session(1, "adman", role)
    }

    // ── Token generation ──────────────────────────────────────────────────────

    #[test]
    fn generated_tokens_are_unique_across_a_meaningful_sample() {
        let store = store();
        let mut seen = HashSet::new();
        for _ in 0..4096 {
            let token = issue(&store, Role::User);
            assert!(seen.insert(token), "duplicate token generated");
        }
        assert_eq!(seen.len(), 4096);
    }

    #[test]
    fn token_entropy_and_format_expectations() {
        let store = store();
        for _ in 0..256 {
            let token = issue(&store, Role::User);
            // 16 bytes -> 22 base64-url chars without padding; all chars from the
            // URL-safe alphabet (letters, digits, '-', '_').
            assert_eq!(token.len(), 22, "token must be 22 URL-safe base64 chars");
            assert!(
                token
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
                "token contains a non-URL-safe character: {token}"
            );
            assert!(!token.contains('='));
            assert!(!token.contains('+'));
            assert!(!token.contains('/'));
        }
    }

    #[test]
    fn url_safe_encoder_matches_expected_alphabet() {
        // 16 bytes of known value to assert the encoder output shape.
        let bytes = [0u8; 16];
        let encoded = encode_url_safe_no_pad(&bytes);
        assert_eq!(encoded.len(), 22);
        // 16 zero bytes -> 'A' * 22 with the standard base64 alphabet.
        assert!(encoded.chars().all(|c| c == 'A'));
    }

    // ── Resolve / refresh ─────────────────────────────────────────────────────

    #[test]
    fn valid_session_resolves_to_backend_created_context() {
        let store = store();
        let token = store.issue_session(42, "alice", Role::Admin);

        let ctx = store
            .resolve_session(&token)
            .expect("valid session resolves");
        assert_eq!(ctx.account_id, 42);
        assert_eq!(ctx.account_key, "alice");
        assert_eq!(ctx.role, Role::Admin);
    }

    #[test]
    fn successful_access_refreshes_idle_time_without_extending_absolute_expiry() {
        let clock = TestClock::new();
        let store = SessionStore::with_clock(clock.handle());
        let token = store.issue_session(1, "adman", Role::User);

        // Advance close to the idle limit (but under it), then access.
        clock.advance(SESSION_IDLE_TIMEOUT - Duration::from_secs(1));
        store
            .resolve_session(&token)
            .expect("session still valid just before idle timeout");

        // Advance another short delta — idle would have expired without the refresh.
        clock.advance(Duration::from_secs(2));
        store
            .resolve_session(&token)
            .expect("refreshed idle time kept the session alive");

        // Absolute expiry is fixed at issuance regardless of refreshes.
        clock.advance(SESSION_ABSOLUTE_LIFETIME);
        assert_eq!(
            store.resolve_session(&token).unwrap_err().code(),
            AUTH_SESSION_EXPIRED,
            "absolute expiry is not extended by refresh"
        );
    }

    // ── Expiry ────────────────────────────────────────────────────────────────

    #[test]
    fn idle_expiry_removes_session() {
        let clock = TestClock::new();
        let store = SessionStore::with_clock(clock.handle());
        let token = store.issue_session(1, "adman", Role::User);

        clock.advance(SESSION_IDLE_TIMEOUT);
        assert_eq!(
            store.resolve_session(&token).unwrap_err().code(),
            AUTH_SESSION_EXPIRED
        );
        // Entry was pruned during resolve.
        assert_eq!(store.active_session_count(), 0);
    }

    #[test]
    fn absolute_expiry_takes_precedence_and_is_not_extended() {
        let clock = TestClock::new();
        let store = SessionStore::with_clock(clock.handle());
        let token = store.issue_session(1, "adman", Role::User);

        // Keep accessing so idle never expires, advancing up to (but not past)
        // the absolute cap. Each access refreshes last_seen_at.
        for _ in 0..23 {
            clock.advance(Duration::from_secs(30 * 60)); // 30 min each, ~11.5h total
            store
                .resolve_session(&token)
                .expect("session valid while under absolute cap");
        }
        // Cross the absolute cap. The session must expire even though idle was
        // being refreshed every step — the absolute deadline never moves.
        clock.advance(Duration::from_secs(30 * 60)); // total now 12h == absolute cap
        assert_eq!(
            store.resolve_session(&token).unwrap_err().code(),
            AUTH_SESSION_EXPIRED,
            "absolute expiry is not extended by refresh"
        );
    }

    // ── Unknown / expired error codes ─────────────────────────────────────────

    #[test]
    fn unknown_token_returns_auth_required() {
        let store = store();
        assert_eq!(
            store
                .resolve_session("not-a-real-token")
                .unwrap_err()
                .code(),
            AUTH_REQUIRED
        );
    }

    #[test]
    fn empty_token_returns_auth_required() {
        let store = store();
        assert_eq!(store.resolve_session("").unwrap_err().code(), AUTH_REQUIRED);
    }

    #[test]
    fn expired_token_returns_auth_session_expired_not_auth_required() {
        let clock = TestClock::new();
        let store = SessionStore::with_clock(clock.handle());
        let token = store.issue_session(1, "adman", Role::User);
        clock.advance(SESSION_IDLE_TIMEOUT);
        let err = store.resolve_session(&token).unwrap_err();
        assert_eq!(err.code(), AUTH_SESSION_EXPIRED);
        assert_ne!(err.code(), AUTH_REQUIRED);
    }

    // ── Approved expiry contract ───────────────────────────────────────────────
    //
    // 1. resolve_session on an existing-but-expired entry: remove it and return
    //    AUTH_SESSION_EXPIRED for that call.
    // 2. A later call with the now-removed token returns AUTH_REQUIRED.
    // 3. If prune_expired removed the token first, AUTH_REQUIRED is acceptable;
    //    no tombstone cache is added.

    #[test]
    fn expired_session_is_removed_and_subsequent_call_is_auth_required() {
        let clock = TestClock::new();
        let store = SessionStore::with_clock(clock.handle());
        let token = store.issue_session(1, "adman", Role::User);

        // Contract point 1: first expired resolve returns AUTH_SESSION_EXPIRED
        // and removes the entry.
        clock.advance(SESSION_IDLE_TIMEOUT);
        assert_eq!(
            store.resolve_session(&token).unwrap_err().code(),
            AUTH_SESSION_EXPIRED,
            "expired resolve returns AUTH_SESSION_EXPIRED"
        );
        // Contract point 2: same token is now gone -> AUTH_REQUIRED.
        assert_eq!(
            store.resolve_session(&token).unwrap_err().code(),
            AUTH_REQUIRED,
            "removed token is unknown on the next call"
        );
        assert_eq!(
            store.active_session_count(),
            0,
            "expired entry was not reinserted"
        );
    }

    #[test]
    fn prune_expired_then_resolve_returns_auth_required() {
        // Contract point 3: if prune_expired removed the token before resolve,
        // resolve returns AUTH_REQUIRED (no tombstone).
        let clock = TestClock::new();
        let store = SessionStore::with_clock(clock.handle());
        let token = store.issue_session(1, "adman", Role::User);
        clock.advance(SESSION_IDLE_TIMEOUT + Duration::from_secs(1));

        store.prune_expired();
        assert_eq!(
            store.resolve_session(&token).unwrap_err().code(),
            AUTH_REQUIRED,
            "pruned token resolves as AUTH_REQUIRED (no tombstone)"
        );
    }

    #[test]
    fn successful_resolve_refreshes_only_last_seen_and_not_absolute_expiry() {
        // The internal entry is not public, so we verify behavior indirectly:
        // repeatedly refreshing well within the idle window must keep the
        // session alive until the absolute cap is reached, and the absolute cap
        // must equal the original issuance cap (not move forward on each refresh).
        let clock = TestClock::new();
        let store = SessionStore::with_clock(clock.handle());
        let token = store.issue_session(1, "adman", Role::User);

        // Refresh several times; each must update last_seen_at only.
        for _ in 0..5 {
            clock.advance(Duration::from_secs(60));
            store
                .resolve_session(&token)
                .expect("session stays valid on refresh");
        }

        // If absolute_expires_at had moved forward with last_seen_at, the session
        // would survive past 12h. It must NOT.
        clock.advance(SESSION_ABSOLUTE_LIFETIME);
        assert_eq!(
            store.resolve_session(&token).unwrap_err().code(),
            AUTH_SESSION_EXPIRED,
            "absolute_expires_at never changed despite refreshes"
        );
    }

    #[test]
    fn expired_session_is_never_reinserted() {
        let clock = TestClock::new();
        let store = SessionStore::with_clock(clock.handle());
        let token = store.issue_session(1, "adman", Role::User);
        clock.advance(SESSION_IDLE_TIMEOUT);

        // First resolve: expired -> removed, AUTH_SESSION_EXPIRED.
        assert_eq!(
            store.resolve_session(&token).unwrap_err().code(),
            AUTH_SESSION_EXPIRED
        );
        // The expired entry must not have been reinserted on the expired path.
        assert_eq!(store.active_session_count(), 0);
        // A second resolve on the same (removed) token: AUTH_REQUIRED, never
        // AUTH_SESSION_EXPIRED (would imply reinsertion + re-expiry).
        assert_eq!(
            store.resolve_session(&token).unwrap_err().code(),
            AUTH_REQUIRED
        );
        assert_eq!(store.active_session_count(), 0);
    }

    #[test]
    fn guard_failure_for_insufficient_role_keeps_session_valid() {
        // A FORBIDDEN guard result must not invalidate the underlying session;
        // the same token must continue to resolve for lower-sensitivity calls.
        let store = store();
        let token = issue(&store, Role::User);

        // Insufficient-role failures.
        assert_eq!(
            require_admin(&store, &token).unwrap_err().code(),
            AUTH_FORBIDDEN
        );
        assert_eq!(
            require_super_admin(&store, &token).unwrap_err().code(),
            AUTH_FORBIDDEN
        );

        // The session is still valid for the authenticated guard and resolve.
        require_authenticated(&store, &token).expect("session still valid after FORBIDDEN");
        store
            .resolve_session(&token)
            .expect("resolve still valid after FORBIDDEN");
    }

    // ── Guards / roles ────────────────────────────────────────────────────────

    #[test]
    fn normal_user_passes_authenticated_guard_only() {
        let store = store();
        let token = issue(&store, Role::User);

        require_authenticated(&store, &token).expect("user passes authenticated");
        assert_eq!(
            require_admin(&store, &token).unwrap_err().code(),
            AUTH_FORBIDDEN
        );
        assert_eq!(
            require_super_admin(&store, &token).unwrap_err().code(),
            AUTH_FORBIDDEN
        );
    }

    #[test]
    fn admin_passes_authenticated_and_admin_guards() {
        let store = store();
        let token = issue(&store, Role::Admin);

        require_authenticated(&store, &token).expect("admin passes authenticated");
        require_admin(&store, &token).expect("admin passes admin");
        assert_eq!(
            require_super_admin(&store, &token).unwrap_err().code(),
            AUTH_FORBIDDEN
        );
    }

    #[test]
    fn super_admin_passes_all_guards() {
        let store = store();
        let token = issue(&store, Role::SuperAdmin);

        require_authenticated(&store, &token).expect("super_admin passes authenticated");
        require_admin(&store, &token).expect("super_admin passes admin");
        require_super_admin(&store, &token).expect("super_admin passes super_admin");
    }

    #[test]
    fn guard_on_unknown_token_returns_auth_required() {
        let store = store();
        assert_eq!(
            require_super_admin(&store, "ghost").unwrap_err().code(),
            AUTH_REQUIRED
        );
    }

    #[test]
    fn role_from_db_str_falls_back_to_user_for_unknown_values() {
        assert_eq!(Role::from_db_str("super_admin"), Role::SuperAdmin);
        assert_eq!(Role::from_db_str("admin"), Role::Admin);
        assert_eq!(Role::from_db_str("user"), Role::User);
        assert_eq!(Role::from_db_str(""), Role::User);
        assert_eq!(
            Role::from_db_str("root"),
            Role::User,
            "unknown never elevates"
        );
        assert_eq!(
            Role::from_db_str("ADMIN"),
            Role::User,
            "matching is case-sensitive"
        );
    }

    // ── Invalidation ──────────────────────────────────────────────────────────

    #[test]
    fn invalidate_token_logs_out_single_session() {
        let store = store();
        let a = issue(&store, Role::User);
        let b = issue(&store, Role::User);

        store.invalidate_token(&a);
        assert_eq!(store.resolve_session(&a).unwrap_err().code(), AUTH_REQUIRED);
        store.resolve_session(&b).expect("other session unaffected");
    }

    #[test]
    fn invalidate_account_revokes_all_sessions_for_that_account() {
        let store = store();
        let a = store.issue_session(7, "alice", Role::Admin);
        let b = store.issue_session(7, "alice", Role::Admin);
        let c = store.issue_session(9, "bob", Role::User);

        store.invalidate_account(7);
        assert_eq!(store.resolve_session(&a).unwrap_err().code(), AUTH_REQUIRED);
        assert_eq!(store.resolve_session(&b).unwrap_err().code(), AUTH_REQUIRED);
        store.resolve_session(&c).expect("other account unaffected");
    }

    #[test]
    fn invalidate_all_clears_every_session() {
        let store = store();
        let a = issue(&store, Role::User);
        let b = issue(&store, Role::SuperAdmin);

        store.invalidate_all();
        assert_eq!(store.resolve_session(&a).unwrap_err().code(), AUTH_REQUIRED);
        assert_eq!(store.resolve_session(&b).unwrap_err().code(), AUTH_REQUIRED);
        assert_eq!(store.active_session_count(), 0);
    }

    #[test]
    fn prune_expired_drops_only_expired_entries() {
        let clock = TestClock::new();
        let store = SessionStore::with_clock(clock.handle());
        // 'old' is issued first, then left idle past the idle timeout.
        let old = store.issue_session(1, "alice", Role::User);
        clock.advance(SESSION_IDLE_TIMEOUT + Duration::from_secs(1)); // 'old' now idle-expired
                                                                      // 'fresh' is issued AFTER the advance, so its last_seen_at is the current now.
        let fresh = store.issue_session(2, "bob", Role::User);

        // prune_expired drops the idle-expired 'old' but keeps 'fresh'.
        store.prune_expired();

        // 'old' was removed from the map, so resolve reports it as unknown.
        assert_eq!(
            store.resolve_session(&old).unwrap_err().code(),
            AUTH_REQUIRED,
            "pruned entry is unknown to resolve"
        );
        // 'fresh' survives because it was just issued.
        store.resolve_session(&fresh).expect("fresh survived prune");
        // And after the dust settles, exactly one live session remains.
        assert_eq!(store.active_session_count(), 1);
    }

    // ── Concurrency / safety ──────────────────────────────────────────────────

    #[test]
    fn poisoned_lock_does_not_panic_and_continues_serving_recovered_state() {
        // Share one store across the poisoning thread and the recovery check so
        // the poisoned Mutex is the same one the resolve call must recover.
        let store = std::sync::Arc::new(store());
        // Seed an entry first so recovery has something to serve.
        let token = store.issue_session(1, "alice", Role::Admin);

        let poison_store = std::sync::Arc::clone(&store);
        let handle = std::thread::spawn(move || {
            // Hold the lock and panic while holding it -> poisons the Mutex.
            let _guard = poison_store.entries.lock().expect("acquire to poison");
            panic!("simulated poisoning under the lock");
        });
        handle.join().expect_err("poisoning thread must panic");

        // The public API must NOT panic on the poisoned lock. With our
        // `unwrap_or_else(|p| p.into_inner())` recovery it continues to serve
        // the previously-inserted entry.
        let ctx = store
            .resolve_session(&token)
            .expect("recovered poisoned lock serves cached session");
        assert_eq!(ctx.account_id, 1);
        assert_eq!(ctx.role, Role::Admin);
    }

    #[test]
    fn concurrent_issue_and_resolve_are_safe() {
        let store = std::sync::Arc::new(store());
        let mut handles = Vec::new();
        for _ in 0..8 {
            let s = std::sync::Arc::clone(&store);
            handles.push(std::thread::spawn(move || {
                let token = s.issue_session(1, "alice", Role::User);
                require_authenticated(&s, &token).expect("concurrent resolve works");
            }));
        }
        for h in handles {
            h.join().expect("thread completed without panic");
        }
        assert_eq!(store.active_session_count(), 8);
    }

    // ── Secret hygiene ─────────────────────────────────────────────────────────

    #[test]
    fn token_value_is_absent_from_formatted_errors_and_debug() {
        let store = store();
        let token = issue(&store, Role::User);

        // Resolve then format the resulting context/error; no token leaks.
        let ctx = store.resolve_session(&token).expect("valid");
        let ctx_debug = format!("{ctx:?}");
        assert!(!ctx_debug.contains(&token), "context Debug leaks token");

        // Auth error carries only the stable code, never the token.
        let err = store.resolve_session("bogus").unwrap_err();
        let err_display = format!("{err}");
        let err_debug = format!("{err:?}");
        assert_eq!(err_display, AUTH_REQUIRED);
        assert!(err_debug.contains(AUTH_REQUIRED));
        assert!(!err_debug.contains("bogus"));
    }

    #[test]
    fn opaque_token_debug_is_redacted() {
        let tok = OpaqueToken::new("super-secret-value".to_string());
        let dbg = format!("{tok:?}");
        assert!(dbg.contains("redacted"));
        assert!(!dbg.contains("super-secret-value"));
    }
}
