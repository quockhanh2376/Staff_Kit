//! Separate authenticated, tamper-evident journal for critical Security and DB Restore events.
//!
//! Enforces:
//! - Complete decoupling from the Recovery Ledger (`ledger.rev`) so recovery guarantees are not compromised.
//! - SHA-256 hash chaining across entries (`prev_hash_n = SHA-256(Block_{n-1})`).
//! - HMAC-SHA256 authentication derived from master key + unique journal UUID.
//! - Durable disk flush on every entry (`file.sync_all()`).
//! - Out-of-DB persistence: Stored in an external file (`audit_critical.ledger`)
//!   that is NEVER overwritten during SQLite database restores.

use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const AUDIT_JOURNAL_FILENAME: &str = "audit_critical.ledger";
pub const AUDIT_JOURNAL_MAGIC: &[u8; 4] = b"SKAJ";
pub const AUDIT_JOURNAL_VERSION: u16 = 1;
pub const AUDIT_JOURNAL_KDF_INFO: &[u8] = b"StaffKit-Audit-Journal-HMAC-v1";

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

    let mut inner = Sha256::new();
    inner.update(&i_key_pad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(&o_key_pad);
    outer.update(&inner_hash);

    let mut result = [0u8; 32];
    result.copy_from_slice(&outer.finalize());
    result
}

/// Derive a 32-byte AuditJournal key using HKDF-SHA256.
pub fn derive_audit_journal_key(master_key: &[u8], journal_uuid: &[u8]) -> [u8; 32] {
    let prk = hmac_sha256(journal_uuid, master_key);
    let mut expand_input = Vec::with_capacity(AUDIT_JOURNAL_KDF_INFO.len() + 1);
    expand_input.extend_from_slice(AUDIT_JOURNAL_KDF_INFO);
    expand_input.push(0x01);
    hmac_sha256(&prk, &expand_input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditJournalHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub uuid: [u8; 16],
    pub created_at: i64,
}

impl AuditJournalHeader {
    pub fn new(uuid: [u8; 16], created_at: i64) -> Self {
        Self {
            magic: *AUDIT_JOURNAL_MAGIC,
            version: AUDIT_JOURNAL_VERSION,
            uuid,
            created_at,
        }
    }

    pub fn serialize(&self) -> [u8; 30] {
        let mut buf = [0u8; 30];
        buf[0..4].copy_from_slice(&self.magic);
        buf[4..6].copy_from_slice(&self.version.to_le_bytes());
        buf[6..22].copy_from_slice(&self.uuid);
        buf[22..30].copy_from_slice(&self.created_at.to_le_bytes());
        buf
    }

    pub fn deserialize(buf: &[u8; 30]) -> Result<Self, String> {
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&buf[0..4]);
        if &magic != AUDIT_JOURNAL_MAGIC {
            return Err("invalid audit journal magic bytes".to_string());
        }

        let version = u16::from_le_bytes([buf[4], buf[5]]);
        if version != AUDIT_JOURNAL_VERSION {
            return Err(format!("unsupported audit journal version: {version}"));
        }

        let mut uuid = [0u8; 16];
        uuid.copy_from_slice(&buf[6..22]);

        let mut created_bytes = [0u8; 8];
        created_bytes.copy_from_slice(&buf[22..30]);
        let created_at = i64::from_le_bytes(created_bytes);

        Ok(Self {
            magic,
            version,
            uuid,
            created_at,
        })
    }

    pub fn digest(&self) -> [u8; 32] {
        let serialized = self.serialize();
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        let mut out = [0u8; 32];
        out.copy_from_slice(&hasher.finalize());
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditJournalBlock {
    pub seq: u64,
    pub timestamp_unix: i64,
    pub event_type: String,
    pub actor: Option<String>,
    pub target: String,
    pub details_json: String,
    pub prev_block_hash: [u8; 32],
    pub hmac: [u8; 32],
}

impl AuditJournalBlock {
    pub fn serialize_payload(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.seq.to_le_bytes());
        buf.extend_from_slice(&self.timestamp_unix.to_le_bytes());

        let event_bytes = self.event_type.as_bytes();
        buf.extend_from_slice(&(event_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(event_bytes);

        match &self.actor {
            Some(a) => {
                buf.push(1);
                let a_bytes = a.as_bytes();
                buf.extend_from_slice(&(a_bytes.len() as u16).to_le_bytes());
                buf.extend_from_slice(a_bytes);
            }
            None => {
                buf.push(0);
            }
        }

        let target_bytes = self.target.as_bytes();
        buf.extend_from_slice(&(target_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(target_bytes);

        let details_bytes = self.details_json.as_bytes();
        buf.extend_from_slice(&(details_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(details_bytes);

        buf.extend_from_slice(&self.prev_block_hash);
        buf
    }

    pub fn deserialize(buf: &[u8], key: &[u8; 32]) -> Result<Self, String> {
        if buf.len() < 32 {
            return Err("audit block buffer too small for HMAC".to_string());
        }
        let payload_len = buf.len() - 32;
        let payload = &buf[..payload_len];
        let mut expected_hmac = [0u8; 32];
        expected_hmac.copy_from_slice(&buf[payload_len..]);

        let actual_hmac = hmac_sha256(key, payload);
        if actual_hmac != expected_hmac {
            return Err("audit journal block HMAC mismatch: block has been tampered with".to_string());
        }

        let mut offset = 0;
        if payload.len() < 16 {
            return Err("malformed audit block: header truncated".to_string());
        }

        let seq = u64::from_le_bytes(payload[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let timestamp_unix = i64::from_le_bytes(payload[offset..offset + 8].try_into().unwrap());
        offset += 8;

        if payload.len() < offset + 2 {
            return Err("malformed audit block: event_type length truncated".to_string());
        }
        let event_len = u16::from_le_bytes(payload[offset..offset + 2].try_into().unwrap()) as usize;
        offset += 2;
        if payload.len() < offset + event_len {
            return Err("malformed audit block: event_type truncated".to_string());
        }
        let event_type = String::from_utf8(payload[offset..offset + event_len].to_vec())
            .map_err(|e| format!("invalid event_type UTF-8: {e}"))?;
        offset += event_len;

        if payload.len() < offset + 1 {
            return Err("malformed audit block: actor flag truncated".to_string());
        }
        let has_actor = payload[offset] == 1;
        offset += 1;
        let actor = if has_actor {
            if payload.len() < offset + 2 {
                return Err("malformed audit block: actor length truncated".to_string());
            }
            let a_len = u16::from_le_bytes(payload[offset..offset + 2].try_into().unwrap()) as usize;
            offset += 2;
            if payload.len() < offset + a_len {
                return Err("malformed audit block: actor truncated".to_string());
            }
            let a_str = String::from_utf8(payload[offset..offset + a_len].to_vec())
                .map_err(|e| format!("invalid actor UTF-8: {e}"))?;
            offset += a_len;
            Some(a_str)
        } else {
            None
        };

        if payload.len() < offset + 2 {
            return Err("malformed audit block: target length truncated".to_string());
        }
        let target_len = u16::from_le_bytes(payload[offset..offset + 2].try_into().unwrap()) as usize;
        offset += 2;
        if payload.len() < offset + target_len {
            return Err("malformed audit block: target truncated".to_string());
        }
        let target = String::from_utf8(payload[offset..offset + target_len].to_vec())
            .map_err(|e| format!("invalid target UTF-8: {e}"))?;
        offset += target_len;

        if payload.len() < offset + 4 {
            return Err("malformed audit block: details_json length truncated".to_string());
        }
        let details_len = u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if payload.len() < offset + details_len {
            return Err("malformed audit block: details_json truncated".to_string());
        }
        let details_json = String::from_utf8(payload[offset..offset + details_len].to_vec())
            .map_err(|e| format!("invalid details_json UTF-8: {e}"))?;
        offset += details_len;

        if payload.len() < offset + 32 {
            return Err("malformed audit block: prev_block_hash truncated".to_string());
        }
        let mut prev_block_hash = [0u8; 32];
        prev_block_hash.copy_from_slice(&payload[offset..offset + 32]);

        Ok(Self {
            seq,
            timestamp_unix,
            event_type,
            actor,
            target,
            details_json,
            prev_block_hash,
            hmac: expected_hmac,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditJournalSummary {
    pub total_blocks: usize,
    pub last_seq: u64,
    pub last_block_hash: [u8; 32],
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct AuditJournal {
    pub path: PathBuf,
    pub header: AuditJournalHeader,
    pub journal_key: [u8; 32],
    pub last_seq: u64,
    pub last_block_hash: [u8; 32],
    pub block_count: usize,
}

impl AuditJournal {
    /// Open existing journal or create new one.
    pub fn open_or_create(path: &Path, master_key: &[u8]) -> Result<Self, String> {
        if path.exists() {
            Self::open_and_verify(path, master_key)
        } else {
            Self::create_new(path, master_key)
        }
    }

    /// Create brand new journal file with fresh UUID and header.
    pub fn create_new(path: &Path, master_key: &[u8]) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("failed to create audit journal dir: {e}"))?;
        }

        let mut uuid = [0u8; 16];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut uuid);
        let now_unix = chrono::Utc::now().timestamp();
        let header = AuditJournalHeader::new(uuid, now_unix);

        let journal_key = derive_audit_journal_key(master_key, &header.uuid);
        let header_digest = header.digest();

        let serialized_header = header.serialize();
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(|e| format!("failed to create audit journal file: {e}"))?;

        file.write_all(&serialized_header)
            .map_err(|e| format!("failed to write audit journal header: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("failed to sync audit journal header: {e}"))?;

        Ok(Self {
            path: path.to_path_buf(),
            header,
            journal_key,
            last_seq: 0,
            last_block_hash: header_digest,
            block_count: 0,
        })
    }

    /// Open and verify all blocks in an existing audit journal.
    pub fn open_and_verify(path: &Path, master_key: &[u8]) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("failed to open audit journal: {e}"))?;
        let file_len = file
            .metadata()
            .map_err(|e| format!("failed to read audit journal metadata: {e}"))?
            .len();

        if file_len < 30 {
            return Err("audit journal file is too small to contain header".to_string());
        }

        let mut reader = BufReader::new(file);
        let mut header_buf = [0u8; 30];
        reader
            .read_exact(&mut header_buf)
            .map_err(|e| format!("failed to read audit journal header: {e}"))?;

        let header = AuditJournalHeader::deserialize(&header_buf)?;
        let journal_key = derive_audit_journal_key(master_key, &header.uuid);

        let mut expected_prev_hash = header.digest();
        let mut expected_seq = 1u64;
        let mut block_count = 0usize;

        while (reader.stream_position().map_err(|e| e.to_string())?) < file_len {
            let mut len_buf = [0u8; 4];
            if let Err(e) = reader.read_exact(&mut len_buf) {
                return Err(format!("failed to read block length at block {expected_seq}: {e}"));
            }
            let block_len = u32::from_le_bytes(len_buf) as usize;
            if block_len < 32 || block_len > 10 * 1024 * 1024 {
                return Err(format!("invalid block length ({block_len}) at seq {expected_seq}"));
            }

            let mut block_buf = vec![0u8; block_len];
            reader
                .read_exact(&mut block_buf)
                .map_err(|e| format!("failed to read block data at block {expected_seq}: {e}"))?;

            let block = AuditJournalBlock::deserialize(&block_buf, &journal_key)?;

            if block.seq != expected_seq {
                return Err(format!(
                    "audit sequence mismatch: expected {expected_seq}, found {}",
                    block.seq
                ));
            }

            if block.prev_block_hash != expected_prev_hash {
                return Err(format!(
                    "audit previous block hash mismatch at seq {expected_seq}"
                ));
            }

            let mut hasher = Sha256::new();
            hasher.update(&block_buf);
            expected_prev_hash.copy_from_slice(&hasher.finalize());

            expected_seq += 1;
            block_count += 1;
        }

        Ok(Self {
            path: path.to_path_buf(),
            header,
            journal_key,
            last_seq: expected_seq - 1,
            last_block_hash: expected_prev_hash,
            block_count,
        })
    }

    /// Append a critical audit event to the journal.
    pub fn append_event(
        &mut self,
        event_type: &str,
        actor: Option<&str>,
        target: &str,
        details_json: &str,
    ) -> Result<u64, String> {
        let seq = self.last_seq + 1;
        let timestamp_unix = chrono::Utc::now().timestamp();

        let block = AuditJournalBlock {
            seq,
            timestamp_unix,
            event_type: event_type.to_string(),
            actor: actor.map(|s| s.to_string()),
            target: target.to_string(),
            details_json: details_json.to_string(),
            prev_block_hash: self.last_block_hash,
            hmac: [0u8; 32],
        };

        let payload = block.serialize_payload();
        let hmac = hmac_sha256(&self.journal_key, &payload);

        let mut block_bytes = Vec::with_capacity(payload.len() + 32);
        block_bytes.extend_from_slice(&payload);
        block_bytes.extend_from_slice(&hmac);

        let block_len = block_bytes.len() as u32;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("failed to open audit journal for append: {e}"))?;

        file.write_all(&block_len.to_le_bytes())
            .map_err(|e| format!("failed to write audit block length: {e}"))?;
        file.write_all(&block_bytes)
            .map_err(|e| format!("failed to write audit block data: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("failed to sync audit block to disk: {e}"))?;

        let mut hasher = Sha256::new();
        hasher.update(&block_bytes);
        self.last_block_hash.copy_from_slice(&hasher.finalize());
        self.last_seq = seq;
        self.block_count += 1;

        Ok(seq)
    }

    /// Read all validated audit blocks.
    pub fn read_all_blocks(&self) -> Result<Vec<AuditJournalBlock>, String> {
        let file = File::open(&self.path).map_err(|e| format!("failed to open audit journal: {e}"))?;
        let file_len = file
            .metadata()
            .map_err(|e| format!("failed to read metadata: {e}"))?
            .len();

        let mut reader = BufReader::new(file);
        reader
            .seek(SeekFrom::Start(30))
            .map_err(|e| format!("failed to seek past header: {e}"))?;

        let mut blocks = Vec::with_capacity(self.block_count);

        while (reader.stream_position().map_err(|e| e.to_string())?) < file_len {
            let mut len_buf = [0u8; 4];
            reader
                .read_exact(&mut len_buf)
                .map_err(|e| format!("failed to read length: {e}"))?;
            let block_len = u32::from_le_bytes(len_buf) as usize;
            let mut block_buf = vec![0u8; block_len];
            reader
                .read_exact(&mut block_buf)
                .map_err(|e| format!("failed to read data: {e}"))?;

            let block = AuditJournalBlock::deserialize(&block_buf, &self.journal_key)?;
            blocks.push(block);
        }

        Ok(blocks)
    }
}

/// Helper function to record a critical audit event into the external journal.
pub fn record_critical_audit_event(
    journal_path: &Path,
    master_key: &[u8],
    event_type: &str,
    actor: Option<&str>,
    target: &str,
    details_json: &str,
) -> Result<u64, String> {
    let mut journal = AuditJournal::open_or_create(journal_path, master_key)?;
    journal.append_event(event_type, actor, target, details_json)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("staffkit_audit_test_{}_{}", name, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
            fs::create_dir_all(&path).expect("create test temp dir");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_audit_journal_create_append_and_verify() {
        let td = TestDir::new("create_append");
        let journal_path = td.path.join(AUDIT_JOURNAL_FILENAME);
        let master_key = b"test_master_key_32_bytes_super!";

        let mut journal = AuditJournal::open_or_create(&journal_path, master_key).expect("create journal");
        assert_eq!(journal.block_count, 0);
        assert_eq!(journal.last_seq, 0);

        let seq1 = journal.append_event("security.database.restore_initiated", Some("adman"), "database", r#"{"source":"test.bak"}"#).expect("append 1");
        assert_eq!(seq1, 1);

        let seq2 = journal.append_event("security.database.restore_completed", Some("adman"), "database", r#"{"status":"SUCCESS"}"#).expect("append 2");
        assert_eq!(seq2, 2);

        // Re-open and verify
        let verified = AuditJournal::open_and_verify(&journal_path, master_key).expect("verify");
        assert_eq!(verified.block_count, 2);
        assert_eq!(verified.last_seq, 2);

        let blocks = verified.read_all_blocks().expect("read all");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].event_type, "security.database.restore_initiated");
        assert_eq!(blocks[0].actor.as_deref(), Some("adman"));
        assert_eq!(blocks[1].event_type, "security.database.restore_completed");
    }

    #[test]
    fn test_audit_journal_tamper_detection() {
        let td = TestDir::new("tamper");
        let journal_path = td.path.join(AUDIT_JOURNAL_FILENAME);
        let master_key = b"test_master_key_32_bytes_super!";

        let mut journal = AuditJournal::open_or_create(&journal_path, master_key).expect("create");
        journal.append_event("security.account.role_changed", Some("adman"), "account_2", r#"{"new_role":"admin"}"#).expect("append");

        // Tamper with the file by flipping a byte in the block
        let mut file_bytes = fs::read(&journal_path).expect("read");
        let len = file_bytes.len();
        file_bytes[len - 40] ^= 0xFF; // flip payload byte
        fs::write(&journal_path, &file_bytes).expect("write tampered");

        let res = AuditJournal::open_and_verify(&journal_path, master_key);
        assert!(res.is_err(), "tampered journal must fail verification");
        assert!(res.unwrap_err().contains("HMAC mismatch"));
    }

    #[test]
    fn test_audit_journal_wrong_key_fails() {
        let td = TestDir::new("wrong_key");
        let journal_path = td.path.join(AUDIT_JOURNAL_FILENAME);
        let master_key = b"test_master_key_32_bytes_super!";
        let wrong_key = b"wrong_key_wrong_key_wrong_key_!";

        let mut journal = AuditJournal::open_or_create(&journal_path, master_key).expect("create");
        journal.append_event("security.database.restore_initiated", Some("adman"), "database", "{}").expect("append");

        let res = AuditJournal::open_and_verify(&journal_path, wrong_key);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("HMAC mismatch"));
    }

    #[test]
    fn test_audit_journal_survives_simulated_db_restore() {
        let td = TestDir::new("restore_survivability");
        let db_path = td.path.join("staffkit.db");
        let journal_path = td.path.join(AUDIT_JOURNAL_FILENAME);
        let master_key = b"test_master_key_32_bytes_super!";

        // 1. Initial database and pre-restore audit events
        fs::write(&db_path, b"initial_db_v1").expect("write db v1");
        record_critical_audit_event(&journal_path, master_key, "security.auth.login_success", Some("adman"), "session", "{}").expect("log login");
        record_critical_audit_event(&journal_path, master_key, "security.account.admin_assisted_password_reset", Some("adman"), "user_1", "{}").expect("log reset");

        // 2. Simulate restoring an old backup by overwriting staffkit.db
        let backup_v0 = b"old_backup_v0";
        fs::write(&db_path, backup_v0).expect("restore old db");

        // 3. Post-restore events appended to journal
        record_critical_audit_event(&journal_path, master_key, "security.database.restore_completed", Some("adman"), "database", r#"{"source":"v0.bak"}"#).expect("log restore");

        // 4. Verify journal still has all 3 events intact across the restore
        let verified = AuditJournal::open_and_verify(&journal_path, master_key).expect("verify");
        assert_eq!(verified.block_count, 3);
        let blocks = verified.read_all_blocks().expect("read");
        assert_eq!(blocks[0].event_type, "security.auth.login_success");
        assert_eq!(blocks[1].event_type, "security.account.admin_assisted_password_reset");
        assert_eq!(blocks[2].event_type, "security.database.restore_completed");
    }
}
