//! Recovery Revocation Ledger file management and verification engine.
//!
//! Enforces:
//! - Append-only integrity with strict sequence checks (`seq_n = seq_{n-1} + 1`).
//! - SHA-256 chain linking (`prev_hash_n = SHA-256(Block_{n-1})`).
//! - HMAC authentication on every block.
//! - Durable file flush (`sync_all()`).

use super::block::{LedgerBlock, LedgerEventType, LedgerHeader};
use super::key_derivation::derive_ledger_key;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerVerificationResult {
    /// Ledger is intact, valid, and contains `count` verified blocks.
    Valid {
        header: LedgerHeader,
        last_seq: u64,
        block_count: usize,
    },
    /// Ledger file does not exist.
    NotFound,
    /// Ledger file is corrupt, truncated, or tampered.
    Corrupted(String),
}

pub struct RecoveryLedger {
    pub path: PathBuf,
    pub header: LedgerHeader,
    pub ledger_key: [u8; 32],
    pub last_seq: u64,
    pub last_block_hash: [u8; 32],
    pub block_count: usize,
}

impl RecoveryLedger {
    /// Initialize a new ledger file on disk, or open and verify an existing one.
    pub fn open_or_create(path: &Path, master_key: &[u8]) -> Result<Self, String> {
        if path.exists() {
            Self::open_and_verify(path, master_key)
        } else {
            Self::create_new(path, master_key)
        }
    }

    /// Create a brand new ledger file with fresh UUID and header.
    pub fn create_new(path: &Path, master_key: &[u8]) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("failed to create ledger dir: {e}"))?;
        }

        // Generate fresh 16-byte UUID
        let mut uuid = [0u8; 16];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut uuid);
        let now_unix = chrono::Utc::now().timestamp();
        let header = LedgerHeader::new(uuid, now_unix);

        let ledger_key = derive_ledger_key(master_key, &header.ledger_uuid);
        let header_digest = header.digest();

        let serialized_header = header.serialize();
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(|e| format!("failed to create ledger file: {e}"))?;

        file.write_all(&serialized_header)
            .map_err(|e| format!("failed to write ledger header: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("failed to sync ledger header: {e}"))?;

        Ok(Self {
            path: path.to_path_buf(),
            header,
            ledger_key,
            last_seq: 0,
            last_block_hash: header_digest,
            block_count: 0,
        })
    }

    /// Open and verify every block and HMAC in the ledger file.
    pub fn open_and_verify(path: &Path, master_key: &[u8]) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("failed to open ledger: {e}"))?;
        let mut reader = BufReader::new(file);

        let header = LedgerHeader::deserialize(&mut reader)
            .map_err(|e| format!("corrupt ledger header: {e}"))?;

        let ledger_key = derive_ledger_key(master_key, &header.ledger_uuid);
        let mut last_seq = 0u64;
        let mut expected_prev_hash = header.digest();
        let mut block_count = 0usize;

        loop {
            // Check if EOF reached cleanly
            let mut peek = [0u8; 1];
            match reader.read(&mut peek) {
                Ok(0) => break, // EOF reached cleanly
                Ok(_) => {
                    // Rewind 1 byte and read full block
                    reader
                        .seek(SeekFrom::Current(-1))
                        .map_err(|e| format!("seek error: {e}"))?;
                }
                Err(e) => return Err(format!("read error during ledger verification: {e}")),
            }

            let block = LedgerBlock::deserialize(&mut reader)
                .map_err(|e| format!("corrupt ledger block at seq {}: {e}", last_seq + 1))?;

            // 1. Sequence check: strictly monotonic
            if block.seq != last_seq + 1 {
                return Err(format!(
                    "ledger sequence violation: expected {}, found {}",
                    last_seq + 1,
                    block.seq
                ));
            }

            // 2. Hash link check: matches previous block hash (or header hash for block 1)
            if block.prev_hash != expected_prev_hash {
                return Err(format!(
                    "ledger hash chain broken at seq {}: prev_hash mismatch",
                    block.seq
                ));
            }

            // 3. Signature verification: HMAC must match
            if !block.verify_signature(&ledger_key) {
                return Err(format!(
                    "ledger block HMAC signature invalid at seq {}",
                    block.seq
                ));
            }

            expected_prev_hash = block.block_hash();
            last_seq = block.seq;
            block_count += 1;
        }

        Ok(Self {
            path: path.to_path_buf(),
            header,
            ledger_key,
            last_seq,
            last_block_hash: expected_prev_hash,
            block_count,
        })
    }

    /// Append a new authenticated block to the ledger and durably flush to disk.
    pub fn append_event(
        &mut self,
        account_id: i64,
        event_type: LedgerEventType,
        payload: Vec<u8>,
    ) -> Result<LedgerBlock, String> {
        let now_unix = chrono::Utc::now().timestamp();
        let next_seq = self.last_seq + 1;

        let block = LedgerBlock::new(
            next_seq,
            self.last_block_hash,
            now_unix,
            account_id,
            event_type,
            payload,
            &self.ledger_key,
        );

        let serialized_block = block.serialize();

        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("failed to open ledger for appending: {e}"))?;

        file.write_all(&serialized_block)
            .map_err(|e| format!("failed to write block {next_seq} to ledger: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("failed to flush block {next_seq} to disk: {e}"))?;

        self.last_block_hash = block.block_hash();
        self.last_seq = next_seq;
        self.block_count += 1;

        Ok(block)
    }

    /// Read all blocks for analysis or fingerprint verification.
    pub fn read_all_blocks(&self) -> Result<Vec<LedgerBlock>, String> {
        let file = File::open(&self.path).map_err(|e| format!("failed to open ledger: {e}"))?;
        let mut reader = BufReader::new(file);

        let _header = LedgerHeader::deserialize(&mut reader)
            .map_err(|e| format!("corrupt ledger header: {e}"))?;

        let mut blocks = Vec::with_capacity(self.block_count);
        loop {
            let mut peek = [0u8; 1];
            match reader.read(&mut peek) {
                Ok(0) => break,
                Ok(_) => {
                    reader
                        .seek(SeekFrom::Current(-1))
                        .map_err(|e| format!("seek error: {e}"))?;
                }
                Err(e) => return Err(format!("read error: {e}")),
            }

            let block = LedgerBlock::deserialize(&mut reader)
                .map_err(|e| format!("error reading block: {e}"))?;
            blocks.push(block);
        }

        Ok(blocks)
    }
}
