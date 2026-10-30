//! Recovery Transaction Journal state machine and crash-safety manager.
//!
//! State flow:
//! 1. `Prepared`: Journal written staging recovery transaction details.
//! 2. `DbCommitted`: Database transaction committed (code consumed, password changed).
//! 3. `Finalized`: Ledger block appended, journal deleted cleanly.
//!
//! If power loss or abrupt exit occurs at any boundary:
//! - Startup reconciler inspects `recovery.journal`.
//! - Discards stale uncommitted journal, OR completes forward deterministically.

use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub const RECOVERY_JOURNAL_FILENAME: &str = "recovery.journal";
pub const RECOVERY_JOURNAL_TMP_FILENAME: &str = "recovery.journal.tmp";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JournalState {
    Prepared,
    DbCommitted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryJournalEntry {
    pub tx_id: String,
    pub account_id: i64,
    pub username: String,
    pub code_id: i64,
    pub code_fingerprint: String,
    pub target_seq: u64,
    pub state: JournalState,
    pub created_at_unix: i64,
    #[serde(default)]
    pub action: Option<String>,
}

impl RecoveryJournalEntry {
    pub fn new_prepared(
        tx_id: String,
        account_id: i64,
        username: String,
        code_id: i64,
        code_fingerprint: String,
        target_seq: u64,
    ) -> Self {
        Self {
            tx_id,
            account_id,
            username,
            code_id,
            code_fingerprint,
            target_seq,
            state: JournalState::Prepared,
            created_at_unix: chrono::Utc::now().timestamp(),
            action: None,
        }
    }

    pub fn new_batch_activation_prepared(
        tx_id: String,
        account_id: i64,
        username: String,
        batch_id: String,
        target_seq: u64,
    ) -> Self {
        Self {
            tx_id,
            account_id,
            username,
            code_id: 0,
            code_fingerprint: batch_id,
            target_seq,
            state: JournalState::Prepared,
            created_at_unix: chrono::Utc::now().timestamp(),
            action: Some("batch_activation".to_string()),
        }
    }
}

pub struct RecoveryJournal {
    pub journal_dir: PathBuf,
}

impl RecoveryJournal {
    pub fn new(journal_dir: PathBuf) -> Self {
        Self { journal_dir }
    }

    pub fn journal_path(&self) -> PathBuf {
        self.journal_dir.join(RECOVERY_JOURNAL_FILENAME)
    }

    pub fn tmp_journal_path(&self) -> PathBuf {
        self.journal_dir.join(RECOVERY_JOURNAL_TMP_FILENAME)
    }

    /// Step 1: Write staged journal file atomically using .tmp and rename.
    pub fn stage_prepared(&self, entry: &RecoveryJournalEntry) -> Result<(), String> {
        fs::create_dir_all(&self.journal_dir)
            .map_err(|e| format!("failed to create journal dir: {e}"))?;

        let json = serde_json::to_vec_pretty(entry)
            .map_err(|e| format!("failed to serialize journal: {e}"))?;

        let tmp_path = self.tmp_journal_path();
        let target_path = self.journal_path();

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp_path)
            .map_err(|e| format!("failed to open tmp journal: {e}"))?;

        file.write_all(&json)
            .map_err(|e| format!("failed to write tmp journal: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("failed to sync tmp journal: {e}"))?;

        // Atomic rename (Windows ReplaceFile / MoveFileEx semantics)
        fs::rename(&tmp_path, &target_path)
            .map_err(|e| format!("failed to atomically commit journal file: {e}"))?;

        Ok(())
    }

    /// Step 2 (Optional in-file update): Mark journal as DB committed.
    pub fn mark_db_committed(&self, mut entry: RecoveryJournalEntry) -> Result<(), String> {
        entry.state = JournalState::DbCommitted;
        self.stage_prepared(&entry)
    }

    /// Read active journal if present.
    pub fn read_active_journal(&self) -> Result<Option<RecoveryJournalEntry>, String> {
        let path = self.journal_path();
        if !path.exists() {
            // Also cleanup dangling .tmp if present from crash before rename
            let tmp_path = self.tmp_journal_path();
            if tmp_path.exists() {
                let _ = fs::remove_file(tmp_path);
            }
            return Ok(None);
        }

        let content = fs::read(&path)
            .map_err(|e| format!("failed to read journal file: {e}"))?;
        let entry: RecoveryJournalEntry = serde_json::from_slice(&content)
            .map_err(|e| format!("corrupt recovery journal file: {e}"))?;

        Ok(Some(entry))
    }

    /// Step 3: Remove journal upon successful finalization.
    pub fn finalize(&self) -> Result<(), String> {
        let path = self.journal_path();
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("failed to delete journal on finalize: {e}"))?;
        }
        let tmp = self.tmp_journal_path();
        if tmp.exists() {
            let _ = fs::remove_file(tmp);
        }
        Ok(())
    }
}
