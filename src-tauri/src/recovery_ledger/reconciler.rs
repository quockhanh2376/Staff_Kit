//! Recovery Transaction Coordinator and Startup Reconciler.
//!
//! Orchestrates the 3-step staged recovery commit:
//! 1. `Prepared`: Stage transaction intent in `recovery.journal`.
//! 2. `DbCommitted`: Atomically commit SQLite transaction (burn code, set password, update epoch).
//! 3. `Finalized`: Append `CODE_BURNED` block to Ledger, sync to USB mirror if configured, delete journal.
//!
//! Exposes `reconcile_on_startup` to guarantee deterministic recovery if power cut occurs.

use super::block::LedgerEventType;
use super::journal::RecoveryJournal;
use super::ledger::RecoveryLedger;
use super::verification_gate::VerificationGate;
use rusqlite::Connection;
use std::path::Path;

pub struct RecoveryTransactionCoordinator;

impl RecoveryTransactionCoordinator {
    /// Reconcile in-flight journal on startup before serving any requests.
    pub fn reconcile_on_startup(
        conn: &Connection,
        journal: &RecoveryJournal,
        local_ledger_path: &Path,
        master_key: &[u8],
    ) -> Result<Option<String>, String> {
        let entry = match journal.read_active_journal()? {
            Some(e) => e,
            None => return Ok(None),
        };

        eprintln!(
            "[RECOVERY RECONCILER] Found in-flight recovery journal (tx_id: {}, state: {:?})",
            entry.tx_id, entry.state
        );

        // Check if database transaction committed
        let is_activation = entry.action.as_deref() == Some("batch_activation") || entry.code_id == 0;

        let is_db_committed: bool = if is_activation {
            conn.query_row(
                "SELECT COUNT(*) = 8 FROM app_recovery_codes WHERE account_id = ? AND batch_uuid = ?",
                rusqlite::params![entry.account_id, entry.code_fingerprint],
                |row| row.get(0),
            )
            .unwrap_or(false)
        } else {
            conn.query_row(
                "SELECT consumed_at IS NOT NULL FROM app_recovery_codes WHERE id = ?",
                [entry.code_id],
                |row| row.get(0),
            )
            .unwrap_or(false)
        };

        if !is_db_committed {
            // DB was never committed before the crash!
            // Discard stale journal cleanly to restore uncommitted state.
            journal.finalize()?;
            eprintln!("[RECOVERY RECONCILER] DB was uncommitted. Journal safely discarded.");
            return Ok(Some(format!(
                "Reconciled uncommitted recovery tx {}: aborted cleanly",
                entry.tx_id
            )));
        }

        // DB was committed! We must ensure Ledger block is committed forward.
        let mut ledger = RecoveryLedger::open_and_verify(local_ledger_path, master_key)?;

        if is_activation {
            let already_in_ledger = ledger.read_all_blocks()?.iter().any(|ev| {
                ev.account_id == entry.account_id
                    && ev.event_type == LedgerEventType::BatchActivated
                    && ev.payload == entry.code_fingerprint.as_bytes()
            });

            if !already_in_ledger {
                let payload = entry.code_fingerprint.as_bytes().to_vec();
                ledger.append_event(entry.account_id, LedgerEventType::BatchActivated, payload)?;
                eprintln!(
                    "[RECOVERY RECONCILER] Forward-committed missing Ledger BatchActivated event for {}",
                    entry.code_fingerprint
                );
                let _ = VerificationGate::sync_to_usb_mirror(conn, local_ledger_path);
            }
        } else if ledger.last_seq < entry.target_seq {
            // Missing block in ledger; append it now
            let payload = entry.code_fingerprint.as_bytes().to_vec();
            ledger.append_event(entry.account_id, LedgerEventType::CodeBurned, payload)?;
            eprintln!(
                "[RECOVERY RECONCILER] Forward-committed missing Ledger block at seq {}",
                entry.target_seq
            );

            // Sync to USB if configured
            let _ = VerificationGate::sync_to_usb_mirror(conn, local_ledger_path);
        }

        // Clean up journal
        journal.finalize()?;
        eprintln!(
            "[RECOVERY RECONCILER] In-flight recovery tx {} finalized successfully.",
            entry.tx_id
        );

        Ok(Some(format!(
            "Reconciled committed recovery tx {}: forward finalized",
            entry.tx_id
        )))
    }
}
