//! Cryptographic key derivation for the Recovery Revocation Ledger.
//!
//! Derives an authenticated `LedgerKey` (32 bytes) from the database master key
//! using standard HKDF-SHA256 with explicit domain separation.
//!
//! The derived key is kept strictly in-memory and is never serialized or written to disk.

use sha2::{Digest, Sha256};

pub const LEDGER_KDF_INFO: &[u8] = b"StaffKit-Recovery-Ledger-HMAC-v1";
pub const LEDGER_KEY_LEN: usize = 32;

/// Derive a 32-byte LedgerKey using standard HKDF-SHA256 (RFC 5869).
///
/// `master_key`: The SQLCipher database encryption key bytes or string.
/// `ledger_uuid`: Salt for HKDF (the unique identifier for the ledger instance).
pub fn derive_ledger_key(master_key: &[u8], ledger_uuid: &[u8]) -> [u8; LEDGER_KEY_LEN] {
    // HKDF-Extract(salt = ledger_uuid, IKM = master_key) -> PRK (32 bytes)
    let prk = hmac_sha256(ledger_uuid, master_key);

    // HKDF-Expand(PRK = prk, info = LEDGER_KDF_INFO, L = 32) -> OKM (32 bytes)
    // T(1) = HMAC-SHA256(PRK, info || 0x01)
    let mut expand_input = Vec::with_capacity(LEDGER_KDF_INFO.len() + 1);
    expand_input.extend_from_slice(LEDGER_KDF_INFO);
    expand_input.push(0x01);

    hmac_sha256(&prk, &expand_input)
}

/// Compute HMAC-SHA256 according to RFC 2104.
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;

    let mut key_block = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let mut hasher = Sha256::new();
        hasher.update(key);
        let digest = hasher.finalize();
        key_block[..32].copy_from_slice(&digest);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut o_key_pad = [0u8; BLOCK_SIZE];
    let mut i_key_pad = [0u8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        o_key_pad[i] = key_block[i] ^ 0x5c;
        i_key_pad[i] = key_block[i] ^ 0x36;
    }

    // Inner hash: H(i_key_pad || data)
    let mut inner_hasher = Sha256::new();
    inner_hasher.update(&i_key_pad);
    inner_hasher.update(data);
    let inner_hash = inner_hasher.finalize();

    // Outer hash: H(o_key_pad || inner_hash)
    let mut outer_hasher = Sha256::new();
    outer_hasher.update(&o_key_pad);
    outer_hasher.update(&inner_hash);
    let outer_hash = outer_hasher.finalize();

    let mut result = [0u8; 32];
    result.copy_from_slice(&outer_hash);
    result
}

/// Compute SHA-256 digest of data.
pub fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sha256_rfc4231_test_case_2() {
        // Test with key = "Jefe" and data = "what do ya want for nothing?"
        let key = b"Jefe";
        let data = b"what do ya want for nothing?";
        let mac = hmac_sha256(key, data);
        let expected_hex = "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843";
        assert_eq!(hex::encode(mac), expected_hex);
    }

    #[test]
    fn test_derive_ledger_key_deterministic_and_separated() {
        let master = b"SK-AES256-test-key-material";
        let uuid1 = b"ledger-uuid-1111-2222";
        let uuid2 = b"ledger-uuid-3333-4444";

        let key1 = derive_ledger_key(master, uuid1);
        let key1_repeat = derive_ledger_key(master, uuid1);
        assert_eq!(key1, key1_repeat, "HKDF derivation must be deterministic");

        let key2 = derive_ledger_key(master, uuid2);
        assert_ne!(key1, key2, "Different ledger UUIDs must yield different keys");

        let different_master = b"SK-AES256-DIFFERENT-master";
        let key3 = derive_ledger_key(different_master, uuid1);
        assert_ne!(key1, key3, "Different master keys must yield different keys");
    }

    // Helper for hex representation in test
    mod hex {
        pub fn encode(data: [u8; 32]) -> String {
            data.iter().map(|b| format!("{:02x}", b)).collect()
        }
    }
}
