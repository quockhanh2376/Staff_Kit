//! Comprehensive tests for Recovery Revocation Ledger, Block format,
//! Journal state machine, and crash/fault-injection matrices.

use super::block::{LedgerBlock, LedgerEventType};
use super::journal::{JournalState, RecoveryJournal, RecoveryJournalEntry};
use super::key_derivation::derive_ledger_key;
use super::ledger::RecoveryLedger;
use super::reconciler::RecoveryTransactionCoordinator;
use super::verification_gate::{RecoveryGateStatus, VerificationGate, LEDGER_FILENAME};
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

const TEST_MASTER_KEY: &[u8] = b"SK-AES256-unit-test-master-key";

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let unique = format!("{}_{}_{}", prefix, std::process::id(), chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).expect("create test temp dir");
        Self { path }
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn create_temp_dir(prefix: &str) -> (TempTestDir, PathBuf) {
    let t = TempTestDir::new(prefix);
    let p = t.path.clone();
    (t, p)
}

fn init_mock_sqlite_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE app_settings (setting_key TEXT PRIMARY KEY, setting_value TEXT, updated_at TEXT);
        CREATE TABLE app_local_accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            token_epoch INTEGER NOT NULL DEFAULT 1,
            force_password_reset INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE app_recovery_codes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account_id INTEGER NOT NULL,
            code_hash TEXT NOT NULL,
            code_index INTEGER NOT NULL,
            consumed_at TEXT DEFAULT NULL
        );
        INSERT INTO app_local_accounts (id, username, password_hash) VALUES (1, 'adman', 'hash_pass');
        INSERT INTO app_recovery_codes (id, account_id, code_hash, code_index) VALUES (101, 1, 'code_hash_1', 1);
        "#,
    )
    .unwrap();
    conn
}

#[test]
fn test_led_a01_key_derivation_determinism_and_salt_isolation() {
    let uuid1 = [1u8; 16];
    let uuid2 = [2u8; 16];

    let key1 = derive_ledger_key(TEST_MASTER_KEY, &uuid1);
    let key1_repeat = derive_ledger_key(TEST_MASTER_KEY, &uuid1);
    assert_eq!(key1, key1_repeat, "Key derivation must be deterministic");

    let key2 = derive_ledger_key(TEST_MASTER_KEY, &uuid2);
    assert_ne!(key1, key2, "Different UUIDs must derive different keys");

    let key_diff_master = derive_ledger_key(b"other-master-key", &uuid1);
    assert_ne!(key1, key_diff_master, "Different master keys must derive different keys");
}

#[test]
fn test_led_a02_block_tampering_and_hmac_invalidation() {
    let ledger_key = [7u8; 32];
    let prev_hash = [0u8; 32];
    let block = LedgerBlock::new(
        1,
        prev_hash,
        1700000000,
        1,
        LedgerEventType::CodeBurned,
        b"fingerprint-12345".to_vec(),
        &ledger_key,
    );

    assert!(block.verify_signature(&ledger_key), "Signature should be valid");

    // Tamper seq
    let mut tampered_seq = block.clone();
    tampered_seq.seq = 2;
    assert!(!tampered_seq.verify_signature(&ledger_key), "Seq tampering must invalidate HMAC");

    // Tamper payload
    let mut tampered_payload = block.clone();
    tampered_payload.payload = b"fingerprint-TAMPERED".to_vec();
    assert!(!tampered_payload.verify_signature(&ledger_key), "Payload tampering must invalidate HMAC");

    // Wrong key
    let wrong_key = [8u8; 32];
    assert!(!block.verify_signature(&wrong_key), "Verification with wrong key must fail");
}

#[test]
fn test_led_a03_ledger_append_and_chain_integrity() {
    let (_dir, path) = create_temp_dir("led_a03");
    let ledger_path = path.join(LEDGER_FILENAME);

    let mut ledger = RecoveryLedger::create_new(&ledger_path, TEST_MASTER_KEY).unwrap();
    assert_eq!(ledger.last_seq, 0);
    assert_eq!(ledger.block_count, 0);

    // Append 3 blocks
    ledger.append_event(1, LedgerEventType::BatchActivated, b"batch-uuid-1".to_vec()).unwrap();
    ledger.append_event(1, LedgerEventType::CodeBurned, b"fp-code-1".to_vec()).unwrap();
    ledger.append_event(1, LedgerEventType::CodeBurned, b"fp-code-2".to_vec()).unwrap();

    assert_eq!(ledger.last_seq, 3);
    assert_eq!(ledger.block_count, 3);

    // Re-open and verify
    let reloaded = RecoveryLedger::open_and_verify(&ledger_path, TEST_MASTER_KEY).unwrap();
    assert_eq!(reloaded.last_seq, 3);
    assert_eq!(reloaded.block_count, 3);

    // Tamper middle block on disk and assert verification error
    let mut bytes = fs::read(&ledger_path).unwrap();
    // Corrupt one byte near the middle
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0xFF;
    fs::write(&ledger_path, bytes).unwrap();

    let tampered_result = RecoveryLedger::open_and_verify(&ledger_path, TEST_MASTER_KEY);
    assert!(tampered_result.is_err(), "Corrupted ledger on disk must fail verification");
}

#[test]
fn test_led_a04_journal_staged_atomic_write_and_cleanup() {
    let (_dir, path) = create_temp_dir("led_a04");
    let journal = RecoveryJournal::new(path);

    let entry = RecoveryJournalEntry::new_prepared(
        "tx-12345".to_string(),
        1,
        "adman".to_string(),
        101,
        "fp-123".to_string(),
        1,
    );

    // Staged write
    journal.stage_prepared(&entry).unwrap();
    assert!(journal.journal_path().exists());
    assert!(!journal.tmp_journal_path().exists(), "Tmp file should be renamed");

    // Read active
    let active = journal.read_active_journal().unwrap().unwrap();
    assert_eq!(active.tx_id, "tx-12345");
    assert_eq!(active.state, JournalState::Prepared);

    // Finalize
    journal.finalize().unwrap();
    assert!(!journal.journal_path().exists());
    assert!(journal.read_active_journal().unwrap().is_none());
}

#[test]
fn test_led_a05_crash_boundary_b_reconciler_uncommitted_db() {
    let (_dir, path) = create_temp_dir("led_a05");
    let journal = RecoveryJournal::new(path.clone());
    let ledger_path = path.join(LEDGER_FILENAME);
    let _ledger = RecoveryLedger::create_new(&ledger_path, TEST_MASTER_KEY).unwrap();

    let conn = init_mock_sqlite_db();

    // Stage prepared journal for code 101, but in DB code 101 is NOT consumed (consumed_at IS NULL)
    let entry = RecoveryJournalEntry::new_prepared(
        "tx-crash-before-db".to_string(),
        1,
        "adman".to_string(),
        101,
        "fp-123".to_string(),
        1,
    );
    journal.stage_prepared(&entry).unwrap();

    // Reconciler runs on startup
    let res = RecoveryTransactionCoordinator::reconcile_on_startup(
        &conn,
        &journal,
        &ledger_path,
        TEST_MASTER_KEY,
    )
    .unwrap();

    assert!(res.unwrap().contains("aborted cleanly"));
    // Journal must be removed
    assert!(!journal.journal_path().exists());
}

#[test]
fn test_led_a06_crash_boundary_d_reconciler_committed_db_forward_commits_ledger() {
    let (_dir, path) = create_temp_dir("led_a06");
    let journal = RecoveryJournal::new(path.clone());
    let ledger_path = path.join(LEDGER_FILENAME);
    let mut ledger = RecoveryLedger::create_new(&ledger_path, TEST_MASTER_KEY).unwrap();
    assert_eq!(ledger.last_seq, 0);

    let conn = init_mock_sqlite_db();
    // Simulate DB committed: mark code 101 consumed_at = NOW
    conn.execute(
        "UPDATE app_recovery_codes SET consumed_at = datetime('now') WHERE id = 101",
        [],
    )
    .unwrap();

    // Stage prepared journal targeting seq 1
    let entry = RecoveryJournalEntry::new_prepared(
        "tx-crash-after-db".to_string(),
        1,
        "adman".to_string(),
        101,
        "fp-123".to_string(),
        1,
    );
    journal.stage_prepared(&entry).unwrap();

    // Reconciler runs on startup
    let res = RecoveryTransactionCoordinator::reconcile_on_startup(
        &conn,
        &journal,
        &ledger_path,
        TEST_MASTER_KEY,
    )
    .unwrap();

    assert!(res.unwrap().contains("forward finalized"));
    assert!(!journal.journal_path().exists());

    // Verify ledger has seq 1 block appended
    ledger = RecoveryLedger::open_and_verify(&ledger_path, TEST_MASTER_KEY).unwrap();
    assert_eq!(ledger.last_seq, 1);
    assert_eq!(ledger.block_count, 1);
}

#[test]
fn test_led_a10_verification_gate_usb_offline_fail_closed() {
    let (_dir, path) = create_temp_dir("led_a10");
    let ledger_path = path.join(LEDGER_FILENAME);
    let _ledger = RecoveryLedger::create_new(&ledger_path, TEST_MASTER_KEY).unwrap();

    let conn = init_mock_sqlite_db();
    // Set USB backup enabled = 1 and non-existent USB path
    conn.execute(
        "INSERT INTO app_settings (setting_key, setting_value) VALUES ('backup_usb_enabled_v1', '1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO app_settings (setting_key, setting_value) VALUES ('backup_usb_path_v1', 'Z:/MissingDrive')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO app_settings (setting_key, setting_value) VALUES ('backup_usb_device_id_v1', 'VOL_TEST_USB')",
        [],
    )
    .unwrap();

    let status = VerificationGate::evaluate(&conn, &ledger_path, TEST_MASTER_KEY);
    match status {
        RecoveryGateStatus::Disabled { reason, is_usb_offline } => {
            assert!(is_usb_offline, "Missing USB must be flagged as USB offline");
            assert!(reason.contains("disconnected or unavailable"));
        }
        _ => panic!("Expected gate to be Disabled when USB is enabled but disconnected"),
    }
}

#[test]
fn test_led_a11_verification_gate_local_only_when_usb_disabled() {
    let (_dir, path) = create_temp_dir("led_a11");
    let ledger_path = path.join(LEDGER_FILENAME);
    let _ledger = RecoveryLedger::create_new(&ledger_path, TEST_MASTER_KEY).unwrap();

    let conn = init_mock_sqlite_db();
    // USB backup enabled = 0
    conn.execute(
        "INSERT INTO app_settings (setting_key, setting_value) VALUES ('backup_usb_enabled_v1', '0')",
        [],
    )
    .unwrap();

    let status = VerificationGate::evaluate(&conn, &ledger_path, TEST_MASTER_KEY);
    match status {
        RecoveryGateStatus::PermittedLocalOnly { local_seq } => {
            assert_eq!(local_seq, 0);
        }
        _ => panic!("Expected gate to be PermittedLocalOnly when USB is disabled"),
    }
}
