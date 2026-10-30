//! Recovery Code generation, Crockford Base32 formatting, and two-phase activation.
//!
//! Enforces:
//! - 8 cryptographically secure codes per Super Admin using Crockford Base32.
//! - Independent Argon2id salts for each code hash.
//! - In-memory transient storage for pending batches (15-minute TTL).
//! - Two-phase Generate -> Confirm Activation flow.
//! - Atomic revocation of existing codes and legacy codes upon activation.
//! - Integration with Recovery Revocation Ledger and Recovery Transaction Journal.

use super::block::LedgerEventType;
use super::ledger::RecoveryLedger;
use super::verification_gate::VerificationGate;
use rand::rngs::OsRng;
use rand::Rng;
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
pub const CODES_PER_BATCH: usize = 8;
pub const CODE_LEN: usize = 10;
pub const PENDING_BATCH_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone)]
pub struct PendingBatch {
    pub batch_id: String,
    pub account_id: i64,
    pub username: String,
    pub plaintext_codes: Vec<String>,
    pub code_hashes: Vec<String>,
    pub created_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRecoveryCodesResult {
    pub batch_id: String,
    pub codes: Vec<String>,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCodeStatus {
    pub total_codes: usize,
    pub remaining_codes: usize,
    pub has_active_batch: bool,
    pub legacy_code_configured: bool,
    pub is_default_legacy_code: bool,
}

pub struct RecoveryCodeManager {
    pending_batches: Mutex<HashMap<i64, PendingBatch>>,
}

impl Default for RecoveryCodeManager {
    fn default() -> Self {
        Self {
            pending_batches: Mutex::new(HashMap::new()),
        }
    }
}

impl RecoveryCodeManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate a 10-char Crockford Base32 code formatted as `XXXXX-XXXXX`.
    pub fn generate_single_code() -> String {
        let mut rng = OsRng;
        let mut raw = String::with_capacity(CODE_LEN);
        for _ in 0..CODE_LEN {
            let idx = rng.gen_range(0..CROCKFORD_ALPHABET.len());
            raw.push(CROCKFORD_ALPHABET[idx] as char);
        }
        format!("{}-{}", &raw[..5], &raw[5..])
    }

    /// Normalize user input: strip hyphens, spaces, to uppercase.
    pub fn normalize_code(input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric())
            .map(|c| c.to_ascii_uppercase())
            .collect()
    }

    /// Phase 1: Generate 8 fresh recovery codes in RAM with 15-min TTL.
    /// Does NOT touch or modify active database codes.
    pub fn generate_codes(
        &self,
        account_id: i64,
        username: &str,
    ) -> Result<GenerateRecoveryCodesResult, String> {
        let mut codes = Vec::with_capacity(CODES_PER_BATCH);
        let mut hashes = Vec::with_capacity(CODES_PER_BATCH);

        for _ in 0..CODES_PER_BATCH {
            let plaintext = Self::generate_single_code();
            let normalized = Self::normalize_code(&plaintext);
            let hash = crate::db::auth::hash_password(&normalized)?;
            codes.push(plaintext);
            hashes.push(hash);
        }

        let batch_id = format!(
            "rc_batch_{}_{}",
            account_id,
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );

        let pending = PendingBatch {
            batch_id: batch_id.clone(),
            account_id,
            username: username.to_string(),
            plaintext_codes: codes.clone(),
            code_hashes: hashes,
            created_at: Instant::now(),
        };

        let mut lock = self.pending_batches.lock().unwrap_or_else(|p| p.into_inner());
        lock.insert(account_id, pending);

        Ok(GenerateRecoveryCodesResult {
            batch_id,
            codes,
            expires_in_seconds: PENDING_BATCH_TTL.as_secs(),
        })
    }

    /// Explicit cancellation of pending generation.
    pub fn cancel_generation(&self, account_id: i64) {
        let mut lock = self.pending_batches.lock().unwrap_or_else(|p| p.into_inner());
        lock.remove(&account_id);
    }

    /// Phase 2: Confirm Activation.
    /// Atomically replaces database codes and appends `BATCH_ACTIVATED` to the ledger.
    pub fn activate_codes(
        &self,
        conn: &mut Connection,
        account_id: i64,
        batch_id: &str,
        local_ledger_path: &Path,
        master_key: &[u8],
    ) -> Result<(), String> {
        // 1. Retrieve and validate pending batch
        let pending = {
            let mut lock = self.pending_batches.lock().unwrap_or_else(|p| p.into_inner());
            let batch = lock.get(&account_id).cloned().ok_or_else(|| {
                "No pending recovery code batch found. Please generate new codes.".to_string()
            })?;

            if batch.batch_id != batch_id {
                return Err("Pending batch ID mismatch.".to_string());
            }

            if batch.created_at.elapsed() > PENDING_BATCH_TTL {
                lock.remove(&account_id);
                return Err("Pending recovery code batch has expired. Please regenerate.".to_string());
            }

            // Remove from memory
            lock.remove(&account_id);
            batch
        };

        // 2. Open / verify ledger
        let mut ledger = RecoveryLedger::open_or_create(local_ledger_path, master_key)?;

        // 3. Stage journal entry before DB commit (Phase A/C1 crash resilience)
        let journal_dir = local_ledger_path.parent().unwrap_or_else(|| Path::new("."));
        let journal = crate::recovery_ledger::journal::RecoveryJournal::new(journal_dir.to_path_buf());
        let journal_entry = crate::recovery_ledger::journal::RecoveryJournalEntry::new_batch_activation_prepared(
            format!("act_{}", pending.batch_id),
            account_id,
            pending.username.clone(),
            pending.batch_id.clone(),
            ledger.last_seq + 1,
        );
        journal.stage_prepared(&journal_entry)?;

        // 4. Atomic Database Transaction
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| format!("failed to start transaction: {e}"))?;

        // Delete existing codes in app_recovery_codes for this account
        tx.execute(
            "DELETE FROM app_recovery_codes WHERE account_id = ?",
            params![account_id],
        )
        .map_err(|e| format!("failed to revoke previous recovery codes: {e}"))?;

        // Revoke legacy recovery code hash
        tx.execute(
            "UPDATE app_local_accounts SET recovery_code_hash = NULL WHERE id = ?",
            params![account_id],
        )
        .map_err(|e| format!("failed to clear legacy recovery code hash: {e}"))?;

        // Insert the 8 new code hashes
        for (i, hash) in pending.code_hashes.iter().enumerate() {
            tx.execute(
                r#"
                INSERT INTO app_recovery_codes (account_id, batch_uuid, code_hash, code_index, consumed_at)
                VALUES (?, ?, ?, ?, NULL)
                "#,
                params![account_id, &pending.batch_id, hash, (i + 1) as i64],
            )
            .map_err(|e| format!("failed to insert recovery code {i}: {e}"))?;
        }

        // Commit SQLite transaction
        tx.commit()
            .map_err(|e| format!("failed to commit recovery codes transaction: {e}"))?;

        // Mark journal DB committed
        let _ = journal.mark_db_committed(journal_entry);

        // 5. Append BATCH_ACTIVATED to Ledger
        let payload = pending.batch_id.as_bytes().to_vec();
        ledger
            .append_event(account_id, LedgerEventType::BatchActivated, payload)
            .map_err(|e| format!("failed to log batch activation in ledger: {e}"))?;

        // Sync to USB mirror if configured
        let _ = VerificationGate::sync_to_usb_mirror(conn, local_ledger_path);

        // Finalize journal (clean removal)
        let _ = journal.finalize();

        Ok(())
    }

    /// Explicit Immediate Revocation: burns all active codes without replacing them.
    pub fn revoke_codes(
        &self,
        conn: &mut Connection,
        account_id: i64,
        local_ledger_path: &Path,
        master_key: &[u8],
    ) -> Result<(), String> {
        self.cancel_generation(account_id);

        let mut ledger = RecoveryLedger::open_or_create(local_ledger_path, master_key)?;

        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| format!("failed to start transaction: {e}"))?;

        tx.execute(
            "DELETE FROM app_recovery_codes WHERE account_id = ?",
            params![account_id],
        )
        .map_err(|e| format!("failed to delete recovery codes: {e}"))?;

        tx.execute(
            "UPDATE app_local_accounts SET recovery_code_hash = NULL WHERE id = ?",
            params![account_id],
        )
        .map_err(|e| format!("failed to clear legacy recovery code hash: {e}"))?;

        tx.commit()
            .map_err(|e| format!("failed to commit revocation transaction: {e}"))?;

        let payload = b"EXPLICIT_REVOCATION".to_vec();
        ledger
            .append_event(account_id, LedgerEventType::BatchRevoked, payload)
            .map_err(|e| format!("failed to append revocation block to ledger: {e}"))?;

        let _ = VerificationGate::sync_to_usb_mirror(conn, local_ledger_path);

        Ok(())
    }

    /// Query recovery code status for an account.
    pub fn query_status(conn: &Connection, account_id: i64) -> Result<RecoveryCodeStatus, String> {
        let total_codes: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM app_recovery_codes WHERE account_id = ?",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let remaining_codes: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM app_recovery_codes WHERE account_id = ? AND consumed_at IS NULL",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let legacy_hash: Option<String> = conn
            .query_row(
                "SELECT recovery_code_hash FROM app_local_accounts WHERE id = ?",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        let legacy_code_configured = legacy_hash.is_some();
        let is_default_legacy_code = match legacy_hash {
            Some(ref hash) => crate::db::auth::verify_password(
                crate::db::DEFAULT_LOCAL_ACCOUNT_RECOVERY_CODE,
                hash,
            )
            .unwrap_or(false),
            None => false,
        };

        Ok(RecoveryCodeStatus {
            total_codes,
            remaining_codes,
            has_active_batch: total_codes > 0 && remaining_codes > 0,
            legacy_code_configured,
            is_default_legacy_code,
        })
    }
}
