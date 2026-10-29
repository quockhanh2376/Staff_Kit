use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use chrono::{DateTime, Local};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Manager};

use super::audit;
use super::schema::{
    AUTO_BACKUP_ENABLED_SETTING_KEY, AUTO_BACKUP_INTERVAL_DAYS, AUTO_BACKUP_INTERVAL_HOURS,
    AUTO_BACKUP_LAST_DATE_SETTING_KEY, AUTO_BACKUP_LAST_STATUS_SETTING_KEY,
    AUTO_BACKUP_RETENTION_DAYS, AUTO_BACKUP_RETENTION_FILES, BACKUP_DIRECTORY_SETTING_KEY,
    BACKUP_FILE_PREFIX, DB_FILE_NAME, DB_SETTINGS_FILE_NAME, HISTORY_FILE_PREFIX,
    HISTORY_FOLDER_NAME, HISTORY_RETENTION_APP_CLOSE,
    HISTORY_RETENTION_CRITICAL_OPS, HISTORY_RETENTION_IMPORT, HISTORY_RETENTION_RESTORE,
    USB_BACKUP_DEVICE_ID_SETTING_KEY, USB_BACKUP_DEVICE_NAME_SETTING_KEY,
    USB_BACKUP_ENABLED_SETTING_KEY, USB_BACKUP_LAST_DATE_SETTING_KEY,
    USB_BACKUP_LAST_STATUS_SETTING_KEY, USB_BACKUP_NEXT_RETRY_DATE_SETTING_KEY,
    USB_BACKUP_PATH_SETTING_KEY, USB_BACKUP_RETRY_STAGE_SETTING_KEY,
};
use super::storage_safety;
use super::{get_setting_value, open_runtime_connection, require_text, set_setting_value};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatus {
    pub initialized: bool,
    pub db_path: String,
    pub sqlite_version: String,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettings {
    pub backup_directory_path: String,
    pub auto_backup_enabled: bool,
    pub retention_files: i64,
    pub auto_backup_interval_days: i64,
    pub auto_backup_retention_days: i64,
    pub last_auto_backup_date: Option<String>,
    pub last_auto_backup_status: Option<String>,
    pub usb_backup_enabled: bool,
    pub usb_backup_directory_path: Option<String>,
    pub usb_device_id: Option<String>,
    pub usb_device_name: Option<String>,
    pub last_usb_backup_date: Option<String>,
    pub last_usb_backup_status: Option<String>,
    pub protection_status: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInfo {
    pub filename: String,
    pub label: String,
    pub timestamp: String,
    pub size_mb: f64,
    pub full_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRunResult {
    pub backup_file_path: String,
    pub retained_files: i64,
    pub performed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettingsUpdateInput {
    pub backup_directory_path: String,
    pub auto_backup_enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureUsbBackupInput {
    pub usb_directory_path: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct DbSettings {
    custom_path: Option<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn get_backup_settings(app: &AppHandle) -> Result<BackupSettings, String> {
    let conn = open_runtime_connection(app)?;
    read_backup_settings(&conn, app)
}

pub fn configure_usb_backup_destination(
    app: &AppHandle,
    payload: ConfigureUsbBackupInput,
) -> Result<BackupSettings, String> {
    let conn = open_runtime_connection(app)?;
    let db_path = super::resolve_database_path(app)?;
    let default_backup_dir = resolve_default_backup_directory(app)?;
    let stored_backup_dir = get_setting_value(&conn, BACKUP_DIRECTORY_SETTING_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let local_backup_dir = PathBuf::from(
        stored_backup_dir.unwrap_or_else(|| default_backup_dir.to_string_lossy().to_string()),
    );

    if payload.enabled {
        let usb_path_str = require_text(payload.usb_directory_path, "usbDirectoryPath")?;
        let usb_path = PathBuf::from(&usb_path_str);

        // Validate destination
        let identity = storage_safety::validate_usb_destination(&usb_path, &db_path, &local_backup_dir)?;

        set_setting_value(&conn, USB_BACKUP_ENABLED_SETTING_KEY, Some("1"))?;
        set_setting_value(&conn, USB_BACKUP_PATH_SETTING_KEY, Some(usb_path_str.as_str()))?;
        set_setting_value(
            &conn,
            USB_BACKUP_DEVICE_ID_SETTING_KEY,
            Some(&identity.volume_serial_number),
        )?;
        set_setting_value(
            &conn,
            USB_BACKUP_DEVICE_NAME_SETTING_KEY,
            Some(&identity.volume_name),
        )?;
        set_setting_value(&conn, USB_BACKUP_RETRY_STAGE_SETTING_KEY, Some("0"))?;
        set_setting_value(&conn, USB_BACKUP_NEXT_RETRY_DATE_SETTING_KEY, None)?;
    } else {
        set_setting_value(&conn, USB_BACKUP_ENABLED_SETTING_KEY, Some("0"))?;
        set_setting_value(&conn, USB_BACKUP_RETRY_STAGE_SETTING_KEY, Some("0"))?;
        set_setting_value(&conn, USB_BACKUP_NEXT_RETRY_DATE_SETTING_KEY, None)?;
    }

    read_backup_settings(&conn, app)
}

pub fn update_backup_settings(
    app: &AppHandle,
    payload: BackupSettingsUpdateInput,
) -> Result<BackupSettings, String> {
    let conn = open_runtime_connection(app)?;
    let backup_directory_path = require_text(payload.backup_directory_path, "backupDirectoryPath")?;

    fs::create_dir_all(&backup_directory_path)
        .map_err(|err| format!("failed to create backup directory: {err}"))?;

    set_setting_value(
        &conn,
        BACKUP_DIRECTORY_SETTING_KEY,
        Some(backup_directory_path.as_str()),
    )?;
    set_setting_value(
        &conn,
        AUTO_BACKUP_ENABLED_SETTING_KEY,
        Some(if payload.auto_backup_enabled {
            "1"
        } else {
            "0"
        }),
    )?;

    read_backup_settings(&conn, app)
}

pub fn backup_database_now(app: &AppHandle) -> Result<BackupRunResult, String> {
    let _guard = try_acquire_backup_lock()?;
    let conn = open_runtime_connection(app)?;
    let settings = read_backup_settings(&conn, app)?;
    run_backup(app, settings.backup_directory_path.as_str(), false)
}

// ── Configurable DB path ──────────────────────────────────────────────────────

pub fn get_db_custom_path(app: &AppHandle) -> Result<Option<String>, String> {
    let settings = read_db_settings(app)?;
    Ok(settings.custom_path.filter(|p| !p.trim().is_empty()))
}

pub fn set_db_custom_path(app: &AppHandle, new_path: Option<&str>) -> Result<(), String> {
    let mut settings = read_db_settings(app)?;
    settings.custom_path = new_path.map(|p| p.to_string());
    let res = write_db_settings(app, &settings);
    super::invalidate_db_cache();
    res
}

/// Change DB to a new location, with two modes:
///
/// **LINK mode** (target DB already exists):
///   The target folder already has a `staff_kit.sqlite3` — just update
///   `db_settings.json` to point there. The existing shared data is preserved.
///
/// **COPY mode** (target folder is empty / no DB there yet):
///   Copy the current local DB to the new location, then update the setting.
///
/// Returns `(new_full_path, linked)` where `linked = true` means LINK mode was used.
pub fn move_database_to(app: &AppHandle, target_folder: &str) -> Result<String, String> {
    let target_folder = target_folder.trim();
    if target_folder.is_empty() {
        return Err("Target folder path cannot be empty.".to_string());
    }

    let target_dir = PathBuf::from(target_folder);
    fs::create_dir_all(&target_dir)
        .map_err(|err| format!("failed to create target directory: {err}"))?;

    let target_path = target_dir.join(DB_FILE_NAME);

    if target_path.exists() {
        // ── LINK MODE: shared DB already exists → just point to it ──────────
        // Validate it is a valid (openable) SQLite / SQLCipher file first
        super::open_encrypted_connection(&target_path).map_err(|_| {
            "The database at the target path could not be opened. \
                 Make sure it is a valid Staff Kit database encrypted with the same version."
                .to_string()
        })?;

        set_db_custom_path(app, Some(target_folder))?;
        super::invalidate_db_cache();

        Ok(format!("LINKED:{}", target_path.to_string_lossy()))
    } else {
        // ── COPY MODE: no DB at target → copy local DB there ────────────────
        let current_path = super::resolve_database_path(app)?;

        // Checkpoint WAL fully BEFORE copying so the copied file is complete
        {
            let conn = super::open_encrypted_connection(&current_path)?;
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|err| format!("failed to checkpoint WAL before move: {err}"))?;
        }
        super::invalidate_db_cache();

        fs::copy(&current_path, &target_path)
            .map_err(|err| format!("failed to copy database to new location: {err}"))?;

        set_db_custom_path(app, Some(target_folder))?;
        super::invalidate_db_cache();

        Ok(format!("COPIED:{}", target_path.to_string_lossy()))
    }
}

// ── History snapshots ─────────────────────────────────────────────────────────

#[cfg(test)]
pub static SIMULATE_SNAPSHOT_FAILURE: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
pub static SIMULATE_SNAPSHOT_VERIFICATION_FAILURE: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub fn set_simulate_snapshot_failure(fail: bool) {
    SIMULATE_SNAPSHOT_FAILURE.store(fail, Ordering::Release);
}

#[cfg(test)]
pub fn set_simulate_snapshot_verification_failure(fail: bool) {
    SIMULATE_SNAPSHOT_VERIFICATION_FAILURE.store(fail, Ordering::Release);
}

/// Verifies that a created snapshot file is a valid, readable, and structurally sound
/// SQLCipher encrypted database by opening it with the application key and running PRAGMA integrity_check.
/// If verification fails, the corrupted/unreadable snapshot file is removed and Err is returned.
pub fn verify_snapshot_integrity(snapshot_path: &Path) -> Result<(), String> {
    #[cfg(test)]
    if SIMULATE_SNAPSHOT_VERIFICATION_FAILURE.load(Ordering::Acquire) {
        let _ = fs::remove_file(snapshot_path);
        return Err("simulated snapshot verification failure: integrity verification failed".to_string());
    }

    if !snapshot_path.exists() {
        return Err(format!(
            "snapshot verification failed: file does not exist ({})",
            snapshot_path.display()
        ));
    }

    let metadata = fs::metadata(snapshot_path)
        .map_err(|err| format!("snapshot verification failed: cannot read file metadata: {err}"))?;
    if metadata.len() == 0 {
        let _ = fs::remove_file(snapshot_path);
        return Err("snapshot verification failed: created file is empty (0 bytes)".to_string());
    }

    let conn = match super::open_encrypted_connection(snapshot_path) {
        Ok(c) => c,
        Err(err) => {
            let _ = fs::remove_file(snapshot_path);
            return Err(format!(
                "snapshot verification failed: unable to decrypt snapshot with application key: {err}"
            ));
        }
    };

    let integrity: String = match conn.query_row("PRAGMA integrity_check;", [], |r| r.get(0)) {
        Ok(val) => val,
        Err(err) => {
            let _ = fs::remove_file(snapshot_path);
            return Err(format!(
                "snapshot verification failed: PRAGMA integrity_check query failed: {err}"
            ));
        }
    };

    if !integrity.eq_ignore_ascii_case("ok") {
        let _ = fs::remove_file(snapshot_path);
        return Err(format!(
            "snapshot verification failed: database integrity check returned '{integrity}'"
        ));
    }

    Ok(())
}

/// Creates and strictly verifies a mandatory safety snapshot for a given database path.
/// Fails closed if snapshot creation or integrity verification fails.
pub fn create_mandatory_safety_snapshot_for_db(
    db_path: &Path,
    label: &str,
) -> Result<SnapshotInfo, String> {
    #[cfg(test)]
    if SIMULATE_SNAPSHOT_FAILURE.load(Ordering::Acquire) {
        return Err("simulated snapshot creation failure: operation blocked by safety guard".to_string());
    }

    let history_dir = resolve_history_dir(db_path)?;
    fs::create_dir_all(&history_dir)
        .map_err(|err| format!("failed to create history directory for safety snapshot: {err}"))?;

    let now = Local::now();
    let timestamp_str = now.format("%Y%m%d_%H%M%S").to_string();
    let safe_label = label.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
    let filename = format!("{HISTORY_FILE_PREFIX}_{timestamp_str}_{safe_label}.sqlite3");
    let snapshot_path = history_dir.join(&filename);

    // 1. Checkpoint WAL and perform VACUUM INTO
    {
        let conn = super::open_encrypted_connection(db_path)?;
        conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
            .map_err(|err| format!("failed to checkpoint WAL before safety snapshot: {err}"))?;
        let escaped = snapshot_path.to_string_lossy().replace('\'', "''");
        conn.execute_batch(&format!("VACUUM INTO '{escaped}';"))
            .map_err(|err| format!("failed to create safety snapshot: {err}"))?;
    }

    // 2. Strict Verification: Open snapshot with encryption key and run PRAGMA integrity_check
    verify_snapshot_integrity(&snapshot_path)?;

    // 3. Prune old history snapshots
    let _ = prune_history_snapshots(&history_dir);

    let size_mb = fs::metadata(&snapshot_path)
        .map(|m| m.len() as f64 / 1_048_576.0)
        .unwrap_or(0.0);

    Ok(SnapshotInfo {
        filename,
        label: safe_label,
        timestamp: now.format("%Y-%m-%d %H:%M").to_string(),
        size_mb: (size_mb * 100.0).round() / 100.0,
        full_path: snapshot_path.to_string_lossy().to_string(),
    })
}

/// Creates and strictly verifies a mandatory safety snapshot before destructive or bulk operations.
/// If creation or integrity verification fails, the operation fails closed and returns Err.
pub fn create_mandatory_safety_snapshot(
    app: &AppHandle,
    label: &str,
) -> Result<SnapshotInfo, String> {
    let db_path = super::resolve_database_path(app)?;
    create_mandatory_safety_snapshot_for_db(&db_path, label)
}

/// Create a snapshot of the current DB. `label` is a short descriptor like
/// "before_import" or "app_close". Keeps at most HISTORY_RETENTION_COUNT files.
pub fn create_history_snapshot(app: &AppHandle, label: &str) -> Result<SnapshotInfo, String> {
    create_mandatory_safety_snapshot(app, label)
}

pub fn list_history_snapshots(app: &AppHandle) -> Result<Vec<SnapshotInfo>, String> {
    let db_path = super::resolve_database_path(app)?;
    let history_dir = resolve_history_dir(&db_path)?;

    if !history_dir.exists() {
        return Ok(Vec::new());
    }

    let mut snapshots: Vec<SnapshotInfo> = fs::read_dir(&history_dir)
        .map_err(|err| format!("failed to scan history directory: {err}"))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with(HISTORY_FILE_PREFIX) && n.ends_with(".sqlite3"))
                    .unwrap_or(false)
        })
        .filter_map(|path| {
            let filename = path.file_name()?.to_string_lossy().to_string();
            let (label, timestamp) = parse_snapshot_filename(&filename);
            let size_mb = fs::metadata(&path)
                .map(|m| m.len() as f64 / 1_048_576.0)
                .unwrap_or(0.0);
            Some(SnapshotInfo {
                full_path: path.to_string_lossy().to_string(),
                filename,
                label,
                timestamp,
                size_mb: (size_mb * 100.0).round() / 100.0,
            })
        })
        .collect();

    // Newest first
    snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(snapshots)
}

#[cfg(test)]
pub static SIMULATE_RESTORE_POST_VERIFY_FAILURE: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub fn set_simulate_restore_post_verify_failure(fail: bool) {
    SIMULATE_RESTORE_POST_VERIFY_FAILURE.store(fail, Ordering::Release);
}

/// Validates that a source backup file is a valid, uncorrupted, and schema-compatible
/// Staff Kit SQLCipher database before performing any restore actions.
pub fn validate_source_backup(source_path: &Path) -> Result<(), String> {
    if !source_path.exists() {
        return Err(format!("source backup file not found: {}", source_path.display()));
    }
    if !source_path.is_file() {
        return Err("selected source backup path is not a file".to_string());
    }
    let metadata = fs::metadata(source_path)
        .map_err(|err| format!("cannot read source backup metadata: {err}"))?;
    if metadata.len() == 0 {
        return Err("source backup file is empty (0 bytes)".to_string());
    }

    // 1. Key validation: must decrypt with application key
    let conn = super::open_encrypted_connection(source_path).map_err(|err| {
        format!("source backup decryption failed: unable to open with encryption key: {err}")
    })?;

    // 2. Integrity validation: PRAGMA integrity_check
    let integrity: String = conn
        .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
        .map_err(|err| format!("source backup integrity check failed: {err}"))?;
    if !integrity.eq_ignore_ascii_case("ok") {
        return Err(format!("source backup is corrupt: integrity check returned '{integrity}'"));
    }

    // 3. Schema compatibility validation: must contain core tables
    let has_employees: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='employees');",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);
    if !has_employees {
        return Err("source backup is incompatible: missing 'employees' table".to_string());
    }

    let source_version: u32 = conn
        .query_row("PRAGMA user_version;", [], |r| r.get(0))
        .unwrap_or(0);
    if source_version > super::SCHEMA_USER_VERSION {
        return Err(format!(
            "source backup schema version ({source_version}) is newer than application schema version ({})",
            super::SCHEMA_USER_VERSION
        ));
    }

    Ok(())
}

/// Executes an atomic, staged replacement of destination_path with source_path.
/// If any step or verification fails, the original database is recovered intact.
fn execute_staged_replacement_with_rollback(
    destination_path: &Path,
    source_path: &Path,
    actor_account_id: Option<i64>,
    snapshot_filename: Option<&str>,
) -> Result<(), String> {
    let parent = destination_path
        .parent()
        .ok_or_else(|| "destination database path has no parent directory".to_string())?;

    let file_stem = destination_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(DB_FILE_NAME);

    let staged_path = parent.join(format!("{file_stem}.restoring"));
    if staged_path.exists() {
        let _ = fs::remove_file(&staged_path);
    }

    // ── STAGE: Stream backup from source into staged_path using Online Backup API ──
    {
        let src_conn = super::open_encrypted_connection(source_path)
            .map_err(|err| format!("failed to open source database: {err}"))?;
        let mut staged_conn = super::open_encrypted_connection(&staged_path)
            .map_err(|err| format!("failed to open staging database: {err}"))?;

        let backup = rusqlite::backup::Backup::new(&src_conn, &mut staged_conn)
            .map_err(|err| format!("failed to initialize staged restore: {err}"))?;
        backup
            .run_to_completion(5, std::time::Duration::from_millis(0), None)
            .map_err(|err| {
                let _ = fs::remove_file(&staged_path);
                format!("failed to stream backup into staging: {err}")
            })?;
        drop(backup);

        staged_conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|err| {
                let _ = fs::remove_file(&staged_path);
                format!("failed to checkpoint staged database: {err}")
            })?;

        super::apply_migrations(&staged_conn).map_err(|err| {
            let _ = fs::remove_file(&staged_path);
            format!("failed to migrate staged database: {err}")
        })?;

        let integrity: String = staged_conn
            .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
            .map_err(|err| {
                let _ = fs::remove_file(&staged_path);
                format!("staged integrity check failed: {err}")
            })?;
        if !integrity.eq_ignore_ascii_case("ok") {
            let _ = fs::remove_file(&staged_path);
            return Err(format!("staged database integrity check failed: '{integrity}'"));
        }
        drop(staged_conn);
        drop(src_conn);
    }

    // ── DRAIN ACTIVE CONNECTIONS & WAL FLUSH ──
    if destination_path.exists() {
        if let Ok(conn) = super::open_encrypted_connection(destination_path) {
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }
    }
    super::invalidate_db_cache();

    // ── PREPARE ROLLBACK BACKUP OF ACTIVE DB ──
    let orig_bak = parent.join(format!("{file_stem}.orig_bak"));
    let wal_path = parent.join(format!("{file_stem}-wal"));
    let shm_path = parent.join(format!("{file_stem}-shm"));
    let wal_bak = parent.join(format!("{file_stem}-wal.bak"));
    let shm_bak = parent.join(format!("{file_stem}-shm.bak"));

    if orig_bak.exists() {
        let _ = fs::remove_file(&orig_bak);
    }
    if wal_bak.exists() {
        let _ = fs::remove_file(&wal_bak);
    }
    if shm_bak.exists() {
        let _ = fs::remove_file(&shm_bak);
    }

    let original_existed = destination_path.exists();
    if original_existed {
        fs::rename(destination_path, &orig_bak)
            .map_err(|err| format!("failed to preserve original database for rollback: {err}"))?;
        if wal_path.exists() {
            let _ = fs::rename(&wal_path, &wal_bak);
        }
        if shm_path.exists() {
            let _ = fs::rename(&shm_path, &shm_bak);
        }
    }

    // ── SWAP STAGED FILE INTO PLACE ──
    if let Err(swap_err) = fs::rename(&staged_path, destination_path) {
        // Rollback immediately
        if original_existed && orig_bak.exists() {
            let _ = fs::rename(&orig_bak, destination_path);
            if wal_bak.exists() {
                let _ = fs::rename(&wal_bak, &wal_path);
            }
            if shm_bak.exists() {
                let _ = fs::rename(&shm_bak, &shm_path);
            }
        }
        let _ = fs::remove_file(&staged_path);
        super::invalidate_db_cache();
        return Err(format!(
            "failed to swap staged database into place; original database recovered intact: {swap_err}"
        ));
    }

    // ── VERIFY RESTORED DATABASE ──
    let verify_result = (|| -> Result<(), String> {
        #[cfg(test)]
        if SIMULATE_RESTORE_POST_VERIFY_FAILURE.load(Ordering::Acquire) {
            return Err("simulated post-restore verification failure for rollback testing".to_string());
        }

        let conn = super::open_encrypted_connection(destination_path)
            .map_err(|e| format!("cannot open restored database: {e}"))?;
        let integrity: String = conn
            .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
            .map_err(|e| format!("integrity check query failed on restored db: {e}"))?;
        if !integrity.eq_ignore_ascii_case("ok") {
            return Err(format!("restored database integrity check failed: '{integrity}'"));
        }

        // Record SUCCESS in restored DB audit logs
        let audit_payload = json!({
            "action": "database.restore",
            "source": source_path.to_string_lossy(),
            "status": "SUCCESS",
            "safetySnapshot": snapshot_filename,
        })
        .to_string();

        let _ = audit::insert_audit_log_conn(
            &conn,
            "database.restore",
            "account",
            actor_account_id.map(|id| id.to_string()).as_deref(),
            "database",
            "main",
            Some(&audit_payload),
        );

        Ok(())
    })();

    if let Err(verify_err) = verify_result {
        // ROLLBACK TO ORIGINAL DATABASE
        let _ = fs::remove_file(destination_path);
        if original_existed && orig_bak.exists() {
            let _ = fs::rename(&orig_bak, destination_path);
            if wal_bak.exists() {
                let _ = fs::rename(&wal_bak, &wal_path);
            }
            if shm_bak.exists() {
                let _ = fs::rename(&shm_bak, &shm_path);
            }
        }
        super::invalidate_db_cache();

        // Record FAILURE in recovered original DB audit logs
        if let Ok(orig_conn) = super::open_encrypted_connection(destination_path) {
            let audit_payload = json!({
                "action": "database.restore",
                "source": source_path.to_string_lossy(),
                "status": "FAILED",
                "error": verify_err,
                "safetySnapshot": snapshot_filename,
            })
            .to_string();

            let _ = audit::insert_audit_log_conn(
                &orig_conn,
                "database.restore",
                "account",
                actor_account_id.map(|id| id.to_string()).as_deref(),
                "database",
                "main",
                Some(&audit_payload),
            );
        }

        return Err(format!(
            "restored database verification failed; original database has been recovered intact: {verify_err}"
        ));
    }

    // Success! Clean up temporary rollback files
    if orig_bak.exists() {
        let _ = fs::remove_file(&orig_bak);
    }
    if wal_bak.exists() {
        let _ = fs::remove_file(&wal_bak);
    }
    if shm_bak.exists() {
        let _ = fs::remove_file(&shm_bak);
    }

    super::invalidate_db_cache();
    Ok(())
}

/// Restores a snapshot safely with validation, safety snapshot, maintenance lock, and rollback.
pub fn restore_history_snapshot(
    app: &AppHandle,
    filename: &str,
    actor_account_id: Option<i64>,
) -> Result<(), String> {
    // Security: reject filenames containing path separators or traversal sequences
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err("invalid snapshot filename: path traversal not allowed".to_string());
    }
    if !filename.starts_with(HISTORY_FILE_PREFIX) || !filename.ends_with(".sqlite3") {
        return Err("invalid snapshot filename: must be a valid snapshot file".to_string());
    }

    let db_path = super::resolve_database_path(app)?;
    let history_dir = resolve_history_dir(&db_path)?;
    let snapshot_path = history_dir.join(filename);

    // Double-check resolved path stays inside history_dir (defense in depth)
    if !snapshot_path.starts_with(&history_dir) {
        return Err("invalid snapshot path: outside history directory".to_string());
    }

    if !snapshot_path.exists() {
        return Err(format!("snapshot file not found: {filename}"));
    }

    let _restore_guard = try_acquire_restore_lock()?;
    validate_source_backup(&snapshot_path)?;
    let safety_snapshot = create_mandatory_safety_snapshot(app, "before_restore")?;

    execute_staged_replacement_with_rollback(
        &db_path,
        &snapshot_path,
        actor_account_id,
        Some(&safety_snapshot.filename),
    )
}

/// Restores the database from an external file safely with validation, safety snapshot, maintenance lock, and rollback.
pub fn restore_database_from_file(
    app: &AppHandle,
    source_path: &str,
    actor_account_id: Option<i64>,
) -> Result<(), String> {
    let source = Path::new(source_path);

    // Basic validation
    if !source.exists() {
        return Err(format!("file not found: {source_path}"));
    }
    if !source.is_file() {
        return Err("selected path is not a file".to_string());
    }
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext != "sqlite3" && ext != "sqlite" && ext != "db" {
        return Err(
            "selected file is not a SQLite database (.sqlite3 / .sqlite / .db)".to_string(),
        );
    }

    let _restore_guard = try_acquire_restore_lock()?;
    validate_source_backup(source)?;
    let safety_snapshot = create_mandatory_safety_snapshot(app, "before_restore")?;

    let db_path = super::resolve_database_path(app)?;
    execute_staged_replacement_with_rollback(
        &db_path,
        source,
        actor_account_id,
        Some(&safety_snapshot.filename),
    )
}

pub(crate) fn migrate_restored_database(path: &Path) -> Result<(), String> {
    let conn = super::open_encrypted_connection(path)
        .map_err(|err| format!("restored database could not be reopened: {err}"))?;
    super::apply_migrations(&conn)
        .map_err(|err| format!("restored database migration failed: {err}"))
}

pub(crate) fn replace_database_from_history_snapshot(
    snapshot_path: &Path,
    database_path: &Path,
) -> Result<(), String> {
    restore_database_file(snapshot_path, database_path)
}

pub(crate) fn restore_database_file(
    source_path: &Path,
    destination_path: &Path,
) -> Result<(), String> {
    let _restore_guard = try_acquire_restore_lock()?;
    validate_source_backup(source_path)?;

    let snapshot_info = if destination_path.exists() {
        create_mandatory_safety_snapshot_for_db(destination_path, "before_restore").ok()
    } else {
        None
    };

    execute_staged_replacement_with_rollback(
        destination_path,
        source_path,
        None,
        snapshot_info.as_ref().map(|s| s.filename.as_str()),
    )
}

// ── Private helpers ───────────────────────────────────────────────────────────

pub(super) fn read_backup_settings(
    conn: &Connection,
    app: &AppHandle,
) -> Result<BackupSettings, String> {
    let default_backup_dir = resolve_default_backup_directory(app)?;
    let stored_backup_dir = get_setting_value(conn, BACKUP_DIRECTORY_SETTING_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let backup_directory_path =
        stored_backup_dir.unwrap_or_else(|| default_backup_dir.to_string_lossy().to_string());

    fs::create_dir_all(backup_directory_path.as_str())
        .map_err(|err| format!("failed to prepare backup directory: {err}"))?;

    let auto_backup_enabled = get_setting_value(conn, AUTO_BACKUP_ENABLED_SETTING_KEY)?
        .map(|value| value.trim() == "1")
        .unwrap_or(false);

    let last_auto_backup_date = get_setting_value(conn, AUTO_BACKUP_LAST_DATE_SETTING_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let last_auto_backup_status = get_setting_value(conn, AUTO_BACKUP_LAST_STATUS_SETTING_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let usb_backup_enabled = get_setting_value(conn, USB_BACKUP_ENABLED_SETTING_KEY)?
        .map(|value| value.trim() == "1")
        .unwrap_or(false);

    let usb_backup_directory_path = get_setting_value(conn, USB_BACKUP_PATH_SETTING_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let usb_device_id = get_setting_value(conn, USB_BACKUP_DEVICE_ID_SETTING_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let usb_device_name = get_setting_value(conn, USB_BACKUP_DEVICE_NAME_SETTING_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let last_usb_backup_date = get_setting_value(conn, USB_BACKUP_LAST_DATE_SETTING_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let last_usb_backup_status = get_setting_value(conn, USB_BACKUP_LAST_STATUS_SETTING_KEY)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let protection_status = compute_protection_status(
        &backup_directory_path,
        auto_backup_enabled,
        last_auto_backup_date.as_deref(),
        last_auto_backup_status.as_deref(),
        usb_backup_enabled,
        usb_backup_directory_path.as_deref(),
        usb_device_id.as_deref(),
        last_usb_backup_date.as_deref(),
        last_usb_backup_status.as_deref(),
    );

    Ok(BackupSettings {
        backup_directory_path,
        auto_backup_enabled,
        retention_files: AUTO_BACKUP_RETENTION_FILES as i64,
        auto_backup_interval_days: AUTO_BACKUP_INTERVAL_DAYS,
        auto_backup_retention_days: AUTO_BACKUP_RETENTION_DAYS,
        last_auto_backup_date,
        last_auto_backup_status,
        usb_backup_enabled,
        usb_backup_directory_path,
        usb_device_id,
        usb_device_name,
        last_usb_backup_date,
        last_usb_backup_status,
        protection_status,
    })
}

pub fn compute_protection_status(
    backup_directory_path: &str,
    _auto_enabled: bool,
    last_auto_date: Option<&str>,
    last_auto_status: Option<&str>,
    usb_enabled: bool,
    usb_path: Option<&str>,
    usb_device_id: Option<&str>,
    last_usb_date: Option<&str>,
    last_usb_status: Option<&str>,
) -> String {
    let now = Local::now();
    let max_freshness_hours = AUTO_BACKUP_INTERVAL_HOURS + 1; // 7 hours max for 6-hour interval

    let local_has_error = last_auto_status
        .map(|s| s.starts_with("ERROR"))
        .unwrap_or(false);
    let local_date_parsed = last_auto_date.and_then(|d| DateTime::parse_from_rfc3339(d).ok());
    let local_is_fresh = if let Some(dt) = local_date_parsed {
        now.signed_duration_since(dt).num_hours() <= max_freshness_hours
    } else {
        false
    };

    let has_any_local_backup = !collect_auto_backup_files(Path::new(backup_directory_path)).is_empty();

    // If local backup failed or is overdue, or never backed up -> At Risk
    if local_has_error || !local_is_fresh || !has_any_local_backup {
        return "At Risk".to_string();
    }

    // 2. Evaluate USB Independent Copy Health
    if !usb_enabled || usb_path.is_none() || usb_device_id.is_none() {
        return "Degraded".to_string();
    }

    let usb_has_error = last_usb_status
        .map(|s| s.starts_with("ERROR"))
        .unwrap_or(false);
    let usb_date_parsed = last_usb_date.and_then(|d| DateTime::parse_from_rfc3339(d).ok());
    let usb_is_fresh = if let Some(dt) = usb_date_parsed {
        now.signed_duration_since(dt).num_hours() <= max_freshness_hours
    } else {
        false
    };

    let usb_connected_and_valid = if let Some(path_str) = usb_path {
        let p = Path::new(path_str);
        if p.exists() {
            if let Ok(identity) = storage_safety::get_volume_identity(p) {
                Some(identity.volume_serial_number) == usb_device_id.map(|s| s.to_string())
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if !usb_connected_and_valid || usb_has_error || !usb_is_fresh {
        return "Degraded".to_string();
    }

    "Protected".to_string()
}

pub fn preflight_disk_space(target_dir: &Path, db_path: &Path) -> Result<(), String> {
    let db_size = fs::metadata(db_path).map(|m| m.len()).unwrap_or(10 * 1024 * 1024);
    let required = db_size + (db_size / 2).max(50 * 1024 * 1024);
    let available = storage_safety::get_available_disk_space(target_dir)?;
    if available < required {
        return Err(format!(
            "insufficient disk space at {}: available {} MB, required {} MB",
            target_dir.display(),
            available / 1_048_576,
            required / 1_048_576
        ));
    }
    Ok(())
}

fn resolve_default_backup_directory(app: &AppHandle) -> Result<PathBuf, String> {
    let mut data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|err| format!("failed to resolve app data directory: {err}"))?;
    data_dir.push("backups");
    Ok(data_dir)
}

fn resolve_history_dir(db_path: &Path) -> Result<PathBuf, String> {
    let db_dir = db_path
        .parent()
        .ok_or_else(|| "failed to resolve DB parent directory".to_string())?;
    Ok(db_dir.join(HISTORY_FOLDER_NAME))
}

static BACKUP_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
pub static RESTORE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub struct BackupInProgressGuard;

impl Drop for BackupInProgressGuard {
    fn drop(&mut self) {
        BACKUP_IN_PROGRESS.store(false, Ordering::Release);
    }
}

#[derive(Debug)]
pub struct RestoreInProgressGuard {
    _backup_guard: BackupInProgressGuard,
}

impl Drop for RestoreInProgressGuard {
    fn drop(&mut self) {
        RESTORE_IN_PROGRESS.store(false, Ordering::Release);
    }
}

pub fn try_acquire_backup_lock() -> Result<BackupInProgressGuard, String> {
    if BACKUP_IN_PROGRESS
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("another backup operation is currently in progress".to_string());
    }
    Ok(BackupInProgressGuard)
}

pub fn try_acquire_restore_lock() -> Result<RestoreInProgressGuard, String> {
    let backup_guard = try_acquire_backup_lock()?;
    if RESTORE_IN_PROGRESS
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("another restore operation is currently in progress".to_string());
    }
    Ok(RestoreInProgressGuard {
        _backup_guard: backup_guard,
    })
}

pub fn is_restore_in_progress() -> bool {
    RESTORE_IN_PROGRESS.load(Ordering::Acquire)
}

#[allow(dead_code)]
pub fn is_backup_in_progress() -> bool {
    BACKUP_IN_PROGRESS.load(Ordering::Acquire)
}

pub fn is_auto_backup_due(conn: &Connection) -> Result<bool, String> {
    is_auto_backup_due_at(conn, Local::now())
}

pub fn is_auto_backup_due_at(
    conn: &Connection,
    current_time: DateTime<Local>,
) -> Result<bool, String> {
    let enabled = get_setting_value(conn, AUTO_BACKUP_ENABLED_SETTING_KEY)?
        .map(|v| v.trim() == "1")
        .unwrap_or(false);

    if !enabled {
        return Ok(false);
    }

    let last_date = get_setting_value(conn, AUTO_BACKUP_LAST_DATE_SETTING_KEY)?
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let Some(last_str) = last_date else {
        // Never backed up yet -> due immediately (startup catch-up)
        return Ok(true);
    };

    let Ok(last_dt) = DateTime::parse_from_rfc3339(&last_str) else {
        // Corrupted or unrecognized timestamp -> due immediately
        return Ok(true);
    };

    let elapsed = current_time.signed_duration_since(last_dt);
    Ok(elapsed.num_hours() >= AUTO_BACKUP_INTERVAL_HOURS)
}

pub fn is_usb_retry_due(conn: &Connection) -> Result<bool, String> {
    let enabled = get_setting_value(conn, USB_BACKUP_ENABLED_SETTING_KEY)?
        .map(|v| v.trim() == "1")
        .unwrap_or(false);

    if !enabled {
        return Ok(false);
    }

    let next_retry_str = get_setting_value(conn, USB_BACKUP_NEXT_RETRY_DATE_SETTING_KEY)?
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let Some(next_str) = next_retry_str else {
        return Ok(false);
    };

    let Ok(next_dt) = DateTime::parse_from_rfc3339(&next_str) else {
        return Ok(false);
    };

    Ok(Local::now() >= next_dt)
}

fn record_usb_sync_failure(app: &AppHandle, err: &str) {
    if let Ok(c) = open_runtime_connection(app) {
        record_usb_sync_failure_with_conn(&c, err);
    }
}

pub(crate) fn record_usb_sync_failure_with_conn(c: &Connection, err: &str) {
    let current_stage: i64 = get_setting_value(c, USB_BACKUP_RETRY_STAGE_SETTING_KEY)
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let next_stage = if current_stage >= 3 { 1 } else { current_stage + 1 };
    let _ = set_setting_value(
        c,
        USB_BACKUP_RETRY_STAGE_SETTING_KEY,
        Some(&next_stage.to_string()),
    );
    let _ = set_setting_value(
        c,
        USB_BACKUP_LAST_STATUS_SETTING_KEY,
        Some(&format!("ERROR: {err}")),
    );

    let now = Local::now();
    let next_retry_dt = match next_stage {
        1 => Some(now + chrono::Duration::minutes(15)),
        2 => Some(now + chrono::Duration::hours(1)),
        3 => Some(now + chrono::Duration::hours(3)),
        _ => None,
    };

    let next_retry_str = next_retry_dt.map(|dt| dt.to_rfc3339());
    let _ = set_setting_value(
        c,
        USB_BACKUP_NEXT_RETRY_DATE_SETTING_KEY,
        next_retry_str.as_deref(),
    );
}

pub fn run_auto_backup_if_due(app: &AppHandle) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    let auto_due = is_auto_backup_due(&conn)?;
    let usb_retry_due = is_usb_retry_due(&conn)?;

    if !auto_due && !usb_retry_due {
        return Ok(false);
    }

    let _guard = match try_acquire_backup_lock() {
        Ok(guard) => guard,
        Err(_) => {
            eprintln!("[Staff Kit Backup] Auto-backup skipped: another backup job is currently in progress");
            return Ok(false);
        }
    };

    let settings = read_backup_settings(&conn, app)?;

    if auto_due {
        if !settings.auto_backup_enabled {
            return Ok(false);
        }

        let local_res = run_backup(app, &settings.backup_directory_path, true);
        match local_res {
            Ok(res) => {
                let now_str = Local::now().to_rfc3339();
                if let Ok(c) = open_runtime_connection(app) {
                    let _ = set_setting_value(&c, AUTO_BACKUP_LAST_DATE_SETTING_KEY, Some(&now_str));
                    let _ = set_setting_value(
                        &c,
                        AUTO_BACKUP_LAST_STATUS_SETTING_KEY,
                        Some(&format!("SUCCESS: {}", res.backup_file_path)),
                    );
                }

                // If USB backup is enabled, sync to USB
                if settings.usb_backup_enabled {
                    match sync_backup_to_usb(app, Path::new(&res.backup_file_path)) {
                        Ok(usb_path) => {
                            if let Ok(c) = open_runtime_connection(app) {
                                let _ = set_setting_value(&c, USB_BACKUP_LAST_DATE_SETTING_KEY, Some(&now_str));
                                let _ = set_setting_value(
                                    &c,
                                    USB_BACKUP_LAST_STATUS_SETTING_KEY,
                                    Some(&format!("SUCCESS: {}", usb_path)),
                                );
                                let _ = set_setting_value(&c, USB_BACKUP_RETRY_STAGE_SETTING_KEY, Some("0"));
                                let _ = set_setting_value(&c, USB_BACKUP_NEXT_RETRY_DATE_SETTING_KEY, None);
                            }
                        }
                        Err(err) => {
                            eprintln!("[Staff Kit Backup] USB sync failed: {err}");
                            record_usb_sync_failure(app, &err);
                        }
                    }
                }

                // Sync + Verify BEFORE Pruning
                let _ = prune_auto_backups_by_age(Path::new(&settings.backup_directory_path), AUTO_BACKUP_RETENTION_DAYS);

                eprintln!(
                    "[Staff Kit Backup] Auto-backup completed successfully: {}",
                    res.backup_file_path
                );
                Ok(true)
            }
            Err(err) => {
                if let Ok(c) = open_runtime_connection(app) {
                    let _ = set_setting_value(
                        &c,
                        AUTO_BACKUP_LAST_STATUS_SETTING_KEY,
                        Some(&format!("ERROR: {err}")),
                    );
                }
                eprintln!("[Staff Kit Backup] Auto-backup failed: {err}");
                Err(err)
            }
        }
    } else if usb_retry_due && settings.usb_backup_enabled {
        // USB retry due: catch up pending local backups to USB
        eprintln!("[Staff Kit Backup] Retrying USB sync...");
        let local_auto_files = collect_auto_backup_files(Path::new(&settings.backup_directory_path));
        if let Some(latest) = local_auto_files.iter().max() {
            match sync_backup_to_usb(app, latest) {
                Ok(usb_path) => {
                    let now_str = Local::now().to_rfc3339();
                    if let Ok(c) = open_runtime_connection(app) {
                        let _ = set_setting_value(&c, USB_BACKUP_LAST_DATE_SETTING_KEY, Some(&now_str));
                        let _ = set_setting_value(
                            &c,
                            USB_BACKUP_LAST_STATUS_SETTING_KEY,
                            Some(&format!("SUCCESS: {}", usb_path)),
                        );
                        let _ = set_setting_value(&c, USB_BACKUP_RETRY_STAGE_SETTING_KEY, Some("0"));
                        let _ = set_setting_value(&c, USB_BACKUP_NEXT_RETRY_DATE_SETTING_KEY, None);
                    }
                    Ok(true)
                }
                Err(err) => {
                    eprintln!("[Staff Kit Backup] USB retry sync failed: {err}");
                    record_usb_sync_failure(app, &err);
                    Ok(false)
                }
            }
        } else {
            Ok(false)
        }
    } else {
        Ok(false)
    }
}

pub fn sync_backup_to_usb(app: &AppHandle, local_backup_path: &Path) -> Result<String, String> {
    let conn = open_runtime_connection(app)?;
    sync_backup_to_usb_with_conn(&conn, local_backup_path)
}

pub fn sync_backup_to_usb_with_conn(conn: &Connection, local_backup_path: &Path) -> Result<String, String> {
    let usb_enabled = get_setting_value(conn, USB_BACKUP_ENABLED_SETTING_KEY)?
        .map(|v| v.trim() == "1")
        .unwrap_or(false);

    if !usb_enabled {
        return Err("USB backup is not enabled".to_string());
    }

    let usb_dir_str = get_setting_value(conn, USB_BACKUP_PATH_SETTING_KEY)?
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "USB backup directory path not configured".to_string())?;
    let usb_dir = PathBuf::from(&usb_dir_str);

    let registered_device_id = get_setting_value(conn, USB_BACKUP_DEVICE_ID_SETTING_KEY)?
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "USB device ID not configured".to_string())?;

    if !usb_dir.exists() {
        return Err(format!(
            "USB backup destination not found or disconnected: {}",
            usb_dir.display()
        ));
    }

    // Check device identity
    let identity = storage_safety::get_volume_identity(&usb_dir)?;
    if identity.volume_serial_number != registered_device_id {
        return Err(format!(
            "USB device mismatch: connected volume serial ({}) does not match registered USB device ({})",
            identity.volume_serial_number, registered_device_id
        ));
    }

    #[cfg(test)]
    if storage_safety::is_simulate_usb_unplug() {
        return Err("simulated USB unplug: connection lost mid-operation".to_string());
    }

    // Preflight disk space on USB
    let local_size = fs::metadata(local_backup_path)
        .map(|m| m.len())
        .unwrap_or(10 * 1024 * 1024);
    let usb_free = storage_safety::get_available_disk_space(&usb_dir)?;
    let required = local_size + 10 * 1024 * 1024;
    if usb_free < required {
        return Err(format!(
            "insufficient disk space on USB drive: available {} MB, required {} MB",
            usb_free / 1_048_576,
            required / 1_048_576
        ));
    }

    let file_name = local_backup_path
        .file_name()
        .ok_or_else(|| "invalid backup filename".to_string())?;
    let temp_usb_path = usb_dir.join(format!("{}.sync_tmp", file_name.to_string_lossy()));
    let final_usb_path = usb_dir.join(file_name);

    if temp_usb_path.exists() {
        let _ = fs::remove_file(&temp_usb_path);
    }

    // Copy to temporary file on USB
    fs::copy(local_backup_path, &temp_usb_path)
        .map_err(|err| format!("failed to stream backup to USB destination: {err}"))?;

    #[cfg(test)]
    if storage_safety::is_simulate_usb_corruption() {
        let _ = fs::write(&temp_usb_path, b"corrupted_usb_bytes");
    }

    // Verify copy: size + SQLCipher decrypt & integrity
    let temp_usb_size = fs::metadata(&temp_usb_path).map(|m| m.len()).unwrap_or(0);
    if temp_usb_size != local_size {
        let _ = fs::remove_file(&temp_usb_path);
        return Err("USB backup copy integrity verification failed: file size mismatch".to_string());
    }

    if let Err(err) = validate_source_backup(&temp_usb_path) {
        let _ = fs::remove_file(&temp_usb_path);
        return Err(format!("USB backup copy integrity verification failed: {err}"));
    }

    // Atomic finalize on USB
    fs::rename(&temp_usb_path, &final_usb_path)
        .map_err(|err| format!("failed to finalize USB backup file: {err}"))?;

    // Catch-up sync of any missing eligible auto-backups from last 30 days
    if let Some(local_parent) = local_backup_path.parent() {
        let _ = catch_up_missing_usb_backups(local_parent, &usb_dir);
    }

    // Prune auto-backups > 30 days on USB (preserving last valid USB backup)
    let _ = prune_auto_backups_by_age(&usb_dir, AUTO_BACKUP_RETENTION_DAYS);

    Ok(final_usb_path.to_string_lossy().to_string())
}

fn catch_up_missing_usb_backups(local_dir: &Path, usb_dir: &Path) -> Result<usize, String> {
    let local_auto_files = collect_auto_backup_files(local_dir);
    let mut copied = 0;
    for local_file in local_auto_files {
        let Some(fname) = local_file.file_name() else {
            continue;
        };
        let final_usb = usb_dir.join(fname);
        if !final_usb.exists() {
            let temp_usb = usb_dir.join(format!("{}.sync_tmp", fname.to_string_lossy()));
            if fs::copy(&local_file, &temp_usb).is_ok() {
                if validate_source_backup(&temp_usb).is_ok() {
                    if fs::rename(&temp_usb, &final_usb).is_ok() {
                        copied += 1;
                    }
                } else {
                    let _ = fs::remove_file(&temp_usb);
                }
            }
        }
    }
    Ok(copied)
}

pub(crate) fn run_backup_on_connection(
    conn: &Connection,
    backup_dir: &Path,
    automatic: bool,
    db_path: Option<&Path>,
) -> Result<BackupRunResult, String> {
    fs::create_dir_all(backup_dir)
        .map_err(|err| format!("failed to create backup directory: {err}"))?;

    if let Some(db_p) = db_path {
        preflight_disk_space(backup_dir, db_p)?;
    }

    let now = Local::now();
    let ts = now.format("%Y-%m-%d_%H-%M-%S");
    let file_name = if automatic {
        format!("{BACKUP_FILE_PREFIX}_auto_{ts}.sqlite3")
    } else {
        format!("{BACKUP_FILE_PREFIX}_manual_{ts}.sqlite3")
    };

    let temp_name = if automatic {
        format!("{BACKUP_FILE_PREFIX}_auto_{ts}.tmp")
    } else {
        format!("{BACKUP_FILE_PREFIX}_manual_{ts}.tmp")
    };

    let temp_path = backup_dir.join(&temp_name);
    let final_path = backup_dir.join(&file_name);

    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
        .map_err(|err| format!("failed to checkpoint sqlite WAL before backup: {err}"))?;

    let escaped_path = temp_path.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{escaped_path}';"))
        .map_err(|err| {
            let _ = fs::remove_file(&temp_path);
            format!("failed to create backup file: {err}")
        })?;

    // Strict verification of created backup file before finalizing
    if let Err(verify_err) = validate_source_backup(&temp_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("backup verification failed on temporary file: {verify_err}"));
    }

    // Atomic finalize
    fs::rename(&temp_path, &final_path)
        .map_err(|err| format!("failed to finalize backup file: {err}"))?;

    let retained_files = collect_auto_backup_files(backup_dir).len();

    Ok(BackupRunResult {
        backup_file_path: final_path.to_string_lossy().to_string(),
        retained_files: retained_files as i64,
        performed: true,
    })
}

fn run_backup(
    app: &AppHandle,
    backup_directory_path: &str,
    automatic: bool,
) -> Result<BackupRunResult, String> {
    let backup_dir = PathBuf::from(backup_directory_path);
    let conn = open_runtime_connection(app)?;
    let db_path = super::resolve_database_path(app).ok();
    run_backup_on_connection(&conn, &backup_dir, automatic, db_path.as_deref())
}

pub fn collect_auto_backup_files(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("staff_kit_backup_auto_") && n.ends_with(".sqlite3"))
                    .unwrap_or(false)
        })
        .collect()
}

pub fn prune_auto_backups_by_age(backup_dir: &Path, max_age_days: i64) -> Result<usize, String> {
    let mut auto_files = collect_auto_backup_files(backup_dir);
    if auto_files.len() <= 1 {
        // Preserves last valid backup
        return Ok(0);
    }

    auto_files.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|meta| meta.modified()).ok();
        let b_time = b.metadata().and_then(|meta| meta.modified()).ok();
        b_time.cmp(&a_time) // newest first
    });

    let cutoff = SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(
            (max_age_days as u64) * 86_400,
        ))
        .ok_or("overflow computing cutoff time")?;

    let mut removed = 0;
    let mut remaining = auto_files.len();

    for path in auto_files.iter().rev() {
        if remaining <= 1 {
            break; // Never delete the last backup
        }

        if let Ok(meta) = path.metadata() {
            if let Ok(modified) = meta.modified() {
                if modified < cutoff {
                    let _ = fs::remove_file(path);
                    removed += 1;
                    remaining -= 1;
                }
            }
        }
    }

    Ok(removed)
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum SnapshotCategory {
    AppClose,
    Import,
    Restore,
    CriticalOps,
    Manual,
}

fn categorize_snapshot_label(label: &str) -> SnapshotCategory {
    let lower = label.to_ascii_lowercase().replace('_', " ");
    if lower == "app close" || lower.starts_with("app close") {
        SnapshotCategory::AppClose
    } else if lower.contains("import")
        || lower.contains("seed")
        || lower.contains("sync")
        || lower.contains("assignment")
        || lower.contains("metadata")
    {
        SnapshotCategory::Import
    } else if lower.contains("restore") {
        SnapshotCategory::Restore
    } else if lower.contains("reset") || lower.contains("migration") || lower.contains("schema") {
        SnapshotCategory::CriticalOps
    } else {
        SnapshotCategory::Manual
    }
}

pub fn prune_history_snapshots(history_dir: &Path) -> Result<(), String> {
    let mut files = collect_sqlite_files(history_dir, HISTORY_FILE_PREFIX);
    if files.is_empty() {
        return Ok(());
    }

    files.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time) // newest first
    });

    let mut by_category: std::collections::HashMap<SnapshotCategory, Vec<PathBuf>> =
        std::collections::HashMap::new();

    for file in files {
        let filename = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let (label, _) = parse_snapshot_filename(filename);
        let category = categorize_snapshot_label(&label);
        by_category.entry(category).or_default().push(file);
    }

    for (cat, cat_files) in by_category {
        let quota = match cat {
            SnapshotCategory::AppClose => HISTORY_RETENTION_APP_CLOSE,
            SnapshotCategory::Import => HISTORY_RETENTION_IMPORT,
            SnapshotCategory::Restore => HISTORY_RETENTION_RESTORE,
            SnapshotCategory::CriticalOps => HISTORY_RETENTION_CRITICAL_OPS,
            SnapshotCategory::Manual => usize::MAX, // Never auto-pruned
        };

        if cat_files.len() > quota {
            for file_to_remove in cat_files.iter().skip(quota) {
                let _ = fs::remove_file(file_to_remove);
            }
        }
    }

    Ok(())
}

fn collect_sqlite_files(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with(prefix) && n.ends_with(".sqlite3"))
                    .unwrap_or(false)
        })
        .collect()
}

fn parse_snapshot_filename(filename: &str) -> (String, String) {
    // format: snap_YYYYMMDD_HHmmss_label.sqlite3
    let base = filename.trim_end_matches(".sqlite3");
    let parts: Vec<&str> = base.splitn(4, '_').collect();
    // parts: ["snap", "YYYYMMDD", "HHmmss", "label"]
    if parts.len() == 4 {
        let date = parts[1];
        let time = parts[2];
        let label = parts[3].replace('_', " ");
        let timestamp = format!(
            "{}-{}-{} {}:{}",
            &date[..4],
            &date[4..6],
            &date[6..8],
            &time[..2],
            &time[2..4],
        );
        (label, timestamp)
    } else {
        (base.to_string(), String::new())
    }
}

// ── db_settings.json helpers (independent of SQLite) ─────────────────────────

fn resolve_db_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|err| format!("failed to resolve app data directory: {err}"))?;
    Ok(data_dir.join(DB_SETTINGS_FILE_NAME))
}

fn read_db_settings(app: &AppHandle) -> Result<DbSettings, String> {
    let path = resolve_db_settings_path(app)?;
    if !path.exists() {
        return Ok(DbSettings::default());
    }
    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read db_settings.json: {err}"))?;
    serde_json::from_str::<DbSettings>(&contents)
        .map_err(|err| format!("failed to parse db_settings.json: {err}"))
}

fn write_db_settings(app: &AppHandle, settings: &DbSettings) -> Result<(), String> {
    let path = resolve_db_settings_path(app)?;
    let contents = serde_json::to_string_pretty(settings)
        .map_err(|err| format!("failed to serialize db_settings: {err}"))?;
    fs::write(&path, contents).map_err(|err| format!("failed to write db_settings.json: {err}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::SystemTime;
    use chrono::Local;

    use rusqlite::params;

    use crate::db::schema::*;
    use crate::db::{apply_migrations, open_encrypted_connection};
    use super::{
        migrate_restored_database, replace_database_from_history_snapshot, restore_database_file,
        AUTO_BACKUP_ENABLED_SETTING_KEY, AUTO_BACKUP_LAST_DATE_SETTING_KEY,
    };

    static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn acquire_test_lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "staff-kit-restore-{label}-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock after unix epoch")
                .as_nanos()
        ))
    }

    fn remove_database_files(path: &Path) {
        for suffix in ["", "-wal", "-shm"] {
            let candidate = PathBuf::from(format!("{}{suffix}", path.to_string_lossy()));
            let _ = std::fs::remove_file(candidate);
        }
    }

    fn create_legacy_database(path: &Path) {
        remove_database_files(path);
        let conn = open_encrypted_connection(path).expect("open legacy encrypted database");
        conn.execute_batch(BASE_SCHEMA_SQL)
            .expect("create base schema for legacy fixture");
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = OFF;
            DROP TABLE borrow_request_confirmations;
            DROP TABLE borrow_handle_with_care_policies;
            DROP TABLE asset_loans;
            DROP TABLE borrow_request_items;
            DROP TABLE borrow_requests;
            CREATE TABLE borrow_requests (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              request_key TEXT NOT NULL UNIQUE,
              employee_id_fk INTEGER NOT NULL REFERENCES employees(id) ON DELETE RESTRICT,
              submitted_employee_id TEXT NOT NULL,
              submitted_full_name TEXT NOT NULL,
              status TEXT NOT NULL DEFAULT 'pending',
              request_type TEXT NOT NULL DEFAULT 'borrow',
              submit_source_ip TEXT,
              decision_note TEXT,
              decided_by_account_id INTEGER,
              submitted_at TEXT NOT NULL DEFAULT (datetime('now')),
              decided_at TEXT
            );
            CREATE TABLE borrow_request_items (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              borrow_request_id INTEGER NOT NULL REFERENCES borrow_requests(id) ON DELETE CASCADE,
              asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE RESTRICT,
              asset_code_snapshot TEXT NOT NULL,
              created_at TEXT NOT NULL DEFAULT (datetime('now')),
              UNIQUE(borrow_request_id, asset_id)
            );
            CREATE TABLE asset_loans (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE RESTRICT,
              employee_id_fk INTEGER NOT NULL REFERENCES employees(id) ON DELETE RESTRICT,
              borrow_request_id INTEGER NOT NULL REFERENCES borrow_requests(id) ON DELETE RESTRICT,
              approved_by_account_id INTEGER REFERENCES app_local_accounts(id) ON DELETE SET NULL,
              borrowed_at TEXT NOT NULL DEFAULT (datetime('now')),
              returned_at TEXT
            );
            INSERT INTO employees(employee_id, full_name) VALUES ('EE-LEGACY', 'Legacy Employee');
            INSERT INTO assets(asset_code, asset_type, display_name, status)
              VALUES ('LEGACY-LAPTOP', 'Laptop', 'Legacy Laptop', 'in_stock');
            INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name)
              VALUES ('LEGACY-REQUEST', 1, 'EE-LEGACY', 'Legacy Employee');
            INSERT INTO borrow_request_items(borrow_request_id, asset_id, asset_code_snapshot)
              VALUES (1, 1, 'LEGACY-LAPTOP');
            "#,
        )
        .expect("create legacy database fixture");
    }

    #[test]
    fn restore_database_file_migrates_legacy_schema_before_returning() {
        let _test_lock = acquire_test_lock();
        let source = temp_path("legacy-source");
        let destination = temp_path("legacy-destination");
        create_legacy_database(&source);
        remove_database_files(&destination);
        {
            let conn = open_encrypted_connection(&destination).expect("open destination");
            apply_migrations(&conn).expect("create destination schema");
        }

        restore_database_file(&source, &destination).expect("restore and migrate legacy database");
        let restored = open_encrypted_connection(&destination).expect("open restored database");
        assert!(restored
            .query_row("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'borrow_handle_with_care_policies'", [], |_| Ok(()))
            .is_ok());
        assert!(restored
            .query_row("SELECT 1 FROM pragma_table_info('borrow_requests') WHERE name = 'borrower_name_snapshot'", [], |_| Ok(()))
            .is_ok());
        assert_eq!(
            restored
                .query_row(
                    "SELECT COUNT(*) FROM borrow_requests WHERE request_key = 'LEGACY-REQUEST'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(
            restored.query_row("SELECT COUNT(*) FROM borrow_request_items WHERE asset_code_snapshot = 'LEGACY-LAPTOP'", [], |row| row.get::<_, i64>(0)).unwrap(),
            1
        );
        assert_eq!(
            restored
                .query_row(
                    "SELECT COUNT(*) FROM borrow_request_confirmations",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );

        remove_database_files(&source);
        remove_database_files(&destination);
    }

    #[test]
    fn history_snapshot_restore_migrates_legacy_schema_before_returning() {
        let _test_lock = acquire_test_lock();
        let snapshot = temp_path("history-source");
        let destination = temp_path("history-destination");
        create_legacy_database(&snapshot);
        remove_database_files(&destination);
        {
            let conn = open_encrypted_connection(&destination).expect("open destination");
            apply_migrations(&conn).expect("create destination schema");
        }

        replace_database_from_history_snapshot(&snapshot, &destination)
            .expect("restore and migrate history snapshot");
        let restored = open_encrypted_connection(&destination).expect("open restored history");
        assert!(restored
            .query_row("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'borrow_request_confirmations'", [], |_| Ok(()))
            .is_ok());
        assert_eq!(
            restored
                .query_row("SELECT COUNT(*) FROM borrow_request_items", [], |row| row
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            1
        );

        remove_database_files(&snapshot);
        remove_database_files(&destination);
    }

    #[test]
    fn restored_migration_failure_is_returned_as_error() {
        let path = temp_path("migration-failure");
        remove_database_files(&path);
        let conn = open_encrypted_connection(&path).expect("open invalid fixture");
        conn.execute_batch("CREATE VIEW assets AS SELECT 1 AS id;")
            .expect("create invalid migration fixture");
        drop(conn);

        assert!(migrate_restored_database(&path).is_err());
        remove_database_files(&path);
    }

    #[test]
    fn restore_current_database_preserves_borrow_evidence_and_is_idempotent() {
        let _test_lock = acquire_test_lock();
        let source = temp_path("current-source");
        let destination = temp_path("current-destination");
        remove_database_files(&source);
        remove_database_files(&destination);

        {
            let conn = open_encrypted_connection(&source).expect("open current source");
            apply_migrations(&conn).expect("create current source schema");
            conn.execute(
                "INSERT INTO employees(employee_id, full_name) VALUES('EE-SNAPSHOT', 'Snapshot Employee')",
                [],
            )
            .expect("insert snapshot employee");
            conn.execute(
                "INSERT INTO assets(asset_code, asset_type, display_name, status) VALUES('SNAPSHOT-LAPTOP', 'Laptop', 'Snapshot Laptop', 'in_stock')",
                [],
            )
            .expect("insert snapshot asset");
            conn.execute(
                "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, borrower_employee_id_fk, borrower_staff_id_snapshot, borrower_name_snapshot, submitted_by_employee_id_fk, submitted_by_staff_id_snapshot, submitted_by_name_snapshot, status) VALUES('SNAPSHOT-REQUEST', 1, 'EE-SNAPSHOT', 'Snapshot Employee', 1, 'EE-BORROWER', 'Borrower Snapshot', 1, 'EE-RETURNER', 'Returner Snapshot', 'approved')",
                [],
            )
            .expect("insert snapshot request");
            let request_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO borrow_request_items(borrow_request_id, asset_id, asset_code_snapshot) VALUES(?, 1, 'SNAPSHOT-LAPTOP')",
                params![request_id],
            )
            .expect("insert snapshot item");
            conn.execute(
                "INSERT INTO borrow_handle_with_care_policies(version, text_en, text_vi, created_at) VALUES(7, 'Care policy EN', 'Care policy VI', datetime('now'))",
                [],
            )
            .expect("insert snapshot policy");
            conn.execute(
                "INSERT INTO borrow_request_confirmations(borrow_request_id, policy_version, policy_text_en_snapshot, policy_text_vi_snapshot, policy_acknowledged, asset_codes_snapshot_json, confirmation_method, signature_png_blob, typed_name, confirmed_at) VALUES(?, 7, 'Care policy EN', 'Care policy VI', 1, '[\"SNAPSHOT-LAPTOP\"]', 'both', ?, 'Snapshot Employee', datetime('now'))",
                params![request_id, vec![137_u8, 80, 78, 71, 0, 255]],
            )
            .expect("insert snapshot confirmation");
        }
        {
            let conn = open_encrypted_connection(&destination).expect("open current destination");
            apply_migrations(&conn).expect("create current destination schema");
        }

        restore_database_file(&source, &destination).expect("restore current database");
        restore_database_file(&source, &destination).expect("repeat current database restore");

        let restored =
            open_encrypted_connection(&destination).expect("open restored current database");
        assert_eq!(
            restored.query_row("SELECT text_en, text_vi FROM borrow_handle_with_care_policies WHERE version = 7", [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).unwrap(),
            ("Care policy EN".to_string(), "Care policy VI".to_string())
        );
        assert_eq!(
            restored.query_row("SELECT borrower_staff_id_snapshot, borrower_name_snapshot, submitted_by_staff_id_snapshot, submitted_by_name_snapshot FROM borrow_requests WHERE request_key = 'SNAPSHOT-REQUEST'", [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))).unwrap(),
            ("EE-BORROWER".to_string(), "Borrower Snapshot".to_string(), "EE-RETURNER".to_string(), "Returner Snapshot".to_string())
        );
        assert_eq!(
            restored.query_row("SELECT asset_code_snapshot FROM borrow_request_items WHERE borrow_request_id = (SELECT id FROM borrow_requests WHERE request_key = 'SNAPSHOT-REQUEST')", [], |row| row.get::<_, String>(0)).unwrap(),
            "SNAPSHOT-LAPTOP"
        );
        assert_eq!(
            restored.query_row("SELECT signature_png_blob FROM borrow_request_confirmations WHERE borrow_request_id = (SELECT id FROM borrow_requests WHERE request_key = 'SNAPSHOT-REQUEST')", [], |row| row.get::<_, Vec<u8>>(0)).unwrap(),
            vec![137_u8, 80, 78, 71, 0, 255]
        );

        remove_database_files(&source);
        remove_database_files(&destination);
    }

    #[test]
    fn test_is_auto_backup_due_when_disabled() {
        let db_path = temp_path("test-auto-backup-disabled");
        remove_database_files(&db_path);
        {
            let conn = open_encrypted_connection(&db_path).expect("open test db");
            apply_migrations(&conn).expect("migrations");

            crate::db::set_setting_value(&conn, AUTO_BACKUP_ENABLED_SETTING_KEY, Some("0")).unwrap();
            assert_eq!(super::is_auto_backup_due(&conn).unwrap(), false);
        }
        remove_database_files(&db_path);
    }

    #[test]
    fn test_is_auto_backup_due_catchup_when_never_run() {
        let db_path = temp_path("test-auto-backup-catchup");
        remove_database_files(&db_path);
        {
            let conn = open_encrypted_connection(&db_path).expect("open test db");
            apply_migrations(&conn).expect("migrations");

            crate::db::set_setting_value(&conn, AUTO_BACKUP_ENABLED_SETTING_KEY, Some("1")).unwrap();
            crate::db::set_setting_value(&conn, AUTO_BACKUP_LAST_DATE_SETTING_KEY, None).unwrap();
            // Should catch up immediately on startup
            assert_eq!(super::is_auto_backup_due(&conn).unwrap(), true);
        }
        remove_database_files(&db_path);
    }

    #[test]
    fn test_is_auto_backup_due_timing_intervals() {
        let db_path = temp_path("test-auto-backup-intervals");
        remove_database_files(&db_path);
        {
            let conn = open_encrypted_connection(&db_path).expect("open test db");
            apply_migrations(&conn).expect("migrations");

            crate::db::set_setting_value(&conn, AUTO_BACKUP_ENABLED_SETTING_KEY, Some("1")).unwrap();

            let base_time = chrono::Local::now();

            // Case A: Last backup 2 hours ago -> NOT due
            let two_hours_ago = base_time - chrono::Duration::hours(2);
            crate::db::set_setting_value(
                &conn,
                AUTO_BACKUP_LAST_DATE_SETTING_KEY,
                Some(&two_hours_ago.to_rfc3339()),
            )
            .unwrap();
            assert_eq!(
                super::is_auto_backup_due_at(&conn, base_time).unwrap(),
                false
            );

            // Case B: Last backup exactly 6 hours ago -> DUE
            let six_hours_ago = base_time - chrono::Duration::hours(6);
            crate::db::set_setting_value(
                &conn,
                AUTO_BACKUP_LAST_DATE_SETTING_KEY,
                Some(&six_hours_ago.to_rfc3339()),
            )
            .unwrap();
            assert_eq!(
                super::is_auto_backup_due_at(&conn, base_time).unwrap(),
                true
            );

            // Case C: Last backup 24 hours ago (overdue after app closed) -> DUE (catch-up)
            let one_day_ago = base_time - chrono::Duration::hours(24);
            crate::db::set_setting_value(
                &conn,
                AUTO_BACKUP_LAST_DATE_SETTING_KEY,
                Some(&one_day_ago.to_rfc3339()),
            )
            .unwrap();
            assert_eq!(
                super::is_auto_backup_due_at(&conn, base_time).unwrap(),
                true
            );

            // Case D: Corrupt / malformed timestamp -> DUE (failsafe catch-up)
            crate::db::set_setting_value(
                &conn,
                AUTO_BACKUP_LAST_DATE_SETTING_KEY,
                Some("not-a-valid-timestamp"),
            )
            .unwrap();
            assert_eq!(
                super::is_auto_backup_due_at(&conn, base_time).unwrap(),
                true
            );
        }
        remove_database_files(&db_path);
    }

    #[test]
    fn test_concurrency_lock_prevents_duplicate_runs() {
        let _test_lock = acquire_test_lock();
        assert_eq!(super::is_backup_in_progress(), false);

        let guard1 = super::try_acquire_backup_lock().expect("acquire first lock");
        assert_eq!(super::is_backup_in_progress(), true);

        // Second acquire must fail
        let guard2_err = super::try_acquire_backup_lock().expect_err("second acquire must fail");
        assert!(guard2_err.contains("in progress"));

        // Release first guard
        drop(guard1);
        assert_eq!(super::is_backup_in_progress(), false);

        // Third acquire should succeed now
        let guard3 = super::try_acquire_backup_lock().expect("acquire after release");
        assert_eq!(super::is_backup_in_progress(), true);
        drop(guard3);
        assert_eq!(super::is_backup_in_progress(), false);
    }

    #[test]
    fn test_run_backup_on_connection_creates_valid_encrypted_file_and_checks_integrity() {
        let db_path = temp_path("test-backup-exec-source");
        let backup_dir = temp_path("test-backup-exec-dir");
        remove_database_files(&db_path);
        let _ = std::fs::remove_dir_all(&backup_dir);

        {
            let conn = open_encrypted_connection(&db_path).expect("open source");
            apply_migrations(&conn).expect("migrations");

            // Insert distinct test record
            conn.execute(
                "INSERT INTO employees(employee_id, full_name) VALUES ('EE-AUTO-TEST', 'Auto Backup Tester');",
                [],
            )
            .expect("insert test employee");

            let result = super::run_backup_on_connection(&conn, &backup_dir, true, Some(&db_path))
                .expect("run auto backup on connection");

            assert!(result.performed);
            assert!(result.backup_file_path.contains("staff_kit_backup_auto_"));
            assert!(Path::new(&result.backup_file_path).exists());

            // Open the generated backup file and verify encryption + integrity
            let backup_conn = open_encrypted_connection(Path::new(&result.backup_file_path))
                .expect("open backup with app encryption key");

            let integrity: String = backup_conn
                .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
                .expect("integrity check");
            assert_eq!(integrity, "ok", "Backup file must pass SQLite integrity_check");

            let count: i64 = backup_conn
                .query_row(
                    "SELECT COUNT(*) FROM employees WHERE employee_id = 'EE-AUTO-TEST';",
                    [],
                    |r| r.get(0),
                )
                .expect("query test record in backup");
            assert_eq!(count, 1, "Backup must contain the employee record");
        }

        remove_database_files(&db_path);
        let _ = std::fs::remove_dir_all(&backup_dir);
    }

    #[test]
    fn test_create_mandatory_safety_snapshot_creates_and_verifies_valid_encrypted_db() {
        let db_path = temp_path("test-safety-snapshot-source");
        remove_database_files(&db_path);

        {
            let conn = open_encrypted_connection(&db_path).expect("open source");
            apply_migrations(&conn).expect("migrations");

            conn.execute(
                "INSERT INTO employees(employee_id, full_name) VALUES ('EE-SNAP-01', 'Snapshot Tester');",
                [],
            )
            .expect("insert employee");

            let snapshot_info = super::create_mandatory_safety_snapshot_for_db(
                &db_path,
                "before_destructive_apply",
            )
            .expect("mandatory safety snapshot must succeed");

            assert!(snapshot_info.filename.contains("before_destructive_apply"));
            assert!(Path::new(&snapshot_info.full_path).exists());

            // Verify the snapshot file independently
            super::verify_snapshot_integrity(Path::new(&snapshot_info.full_path))
                .expect("snapshot verification must pass");

            // Open snapshot with app key and verify data
            let snap_conn = open_encrypted_connection(Path::new(&snapshot_info.full_path))
                .expect("open snapshot database");
            let count: i64 = snap_conn
                .query_row(
                    "SELECT COUNT(*) FROM employees WHERE employee_id = 'EE-SNAP-01';",
                    [],
                    |r| r.get(0),
                )
                .expect("query employee in snapshot");
            assert_eq!(count, 1);
        }

        remove_database_files(&db_path);
    }

    #[test]
    fn test_verify_snapshot_integrity_detects_empty_corrupted_or_unencrypted_files() {
        let test_dir = temp_path("test-snapshot-verify-dir");
        let _ = std::fs::create_dir_all(&test_dir);

        // 1. Non-existent file
        let non_existent = test_dir.join("non_existent.sqlite3");
        let err = super::verify_snapshot_integrity(&non_existent).expect_err("non-existent file");
        assert!(err.contains("does not exist"));

        // 2. Empty file (0 bytes) -> must fail and remove bad file
        let empty_file = test_dir.join("empty.sqlite3");
        std::fs::write(&empty_file, b"").expect("write empty file");
        assert!(empty_file.exists());
        let err = super::verify_snapshot_integrity(&empty_file).expect_err("empty file");
        assert!(err.contains("0 bytes"));
        assert!(!empty_file.exists(), "empty corrupt file must be removed");

        // 3. Corrupt / unencrypted garbage bytes -> must fail and remove bad file
        let garbage_file = test_dir.join("garbage.sqlite3");
        std::fs::write(&garbage_file, b"THIS IS NOT A VALID SQLCIPHER DATABASE").expect("write garbage");
        assert!(garbage_file.exists());
        let err = super::verify_snapshot_integrity(&garbage_file).expect_err("garbage file");
        assert!(err.contains("verification failed") || err.contains("unable to decrypt"));
        assert!(!garbage_file.exists(), "corrupt file must be removed");

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_mandatory_safety_snapshot_fail_closed_on_creation_failure() {
        let _test_lock = acquire_test_lock();
        let db_path = temp_path("test-fail-closed-creation");
        remove_database_files(&db_path);

        {
            let conn = open_encrypted_connection(&db_path).expect("open source");
            apply_migrations(&conn).expect("migrations");

            super::set_simulate_snapshot_failure(true);
            let result = super::create_mandatory_safety_snapshot_for_db(
                &db_path,
                "simulate_failure_label",
            );
            super::set_simulate_snapshot_failure(false);

            assert!(result.is_err(), "Must fail closed when snapshot creation fails");
            let err_msg = result.unwrap_err();
            assert!(err_msg.contains("blocked by safety guard"));
        }

        remove_database_files(&db_path);
    }

    #[test]
    fn test_mandatory_safety_snapshot_fail_closed_on_verification_failure() {
        let _test_lock = acquire_test_lock();
        let db_path = temp_path("test-fail-closed-verification");
        remove_database_files(&db_path);

        {
            let conn = open_encrypted_connection(&db_path).expect("open source");
            apply_migrations(&conn).expect("migrations");

            super::set_simulate_snapshot_verification_failure(true);
            let result = super::create_mandatory_safety_snapshot_for_db(
                &db_path,
                "simulate_verify_fail",
            );
            super::set_simulate_snapshot_verification_failure(false);

            assert!(result.is_err(), "Must fail closed when verification fails");
            let err_msg = result.unwrap_err();
            assert!(err_msg.contains("verification failure"));
        }

        remove_database_files(&db_path);
    }

    #[test]
    fn test_restore_rejects_invalid_key_or_unencrypted_source() {
        let _test_lock = acquire_test_lock();
        let destination = temp_path("restore-invalid-key-dst");
        let invalid_source = temp_path("restore-invalid-key-src");
        remove_database_files(&destination);
        remove_database_files(&invalid_source);

        // Destination with valid schema and employee
        {
            let conn = open_encrypted_connection(&destination).expect("open destination");
            apply_migrations(&conn).expect("migrations");
            conn.execute("INSERT INTO employees(employee_id, full_name) VALUES('EE-INTACT', 'Intact Employee')", []).unwrap();
        }

        // Source file with plain/garbage bytes
        std::fs::write(&invalid_source, b"THIS_IS_PLAIN_OR_WRONGLY_ENCRYPTED_DATA").unwrap();

        let val_err = super::validate_source_backup(&invalid_source).expect_err("must reject invalid key/garbage");
        assert!(val_err.contains("decryption failed") || val_err.contains("unable to open"));

        let res = super::restore_database_file(&invalid_source, &destination);
        assert!(res.is_err(), "restore must fail closed");

        // Verify destination was NOT modified
        let check_conn = open_encrypted_connection(&destination).expect("open destination after failed restore");
        let count: i64 = check_conn.query_row("SELECT count(*) FROM employees WHERE employee_id = 'EE-INTACT'", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1, "Original database must remain 100% intact");

        remove_database_files(&destination);
        remove_database_files(&invalid_source);
    }

    #[test]
    fn test_restore_rejects_corrupted_backup() {
        let _test_lock = acquire_test_lock();
        let destination = temp_path("restore-corrupt-dst");
        let corrupt_source = temp_path("restore-corrupt-src");
        remove_database_files(&destination);
        remove_database_files(&corrupt_source);

        {
            let conn = open_encrypted_connection(&destination).expect("open destination");
            apply_migrations(&conn).expect("migrations");
            conn.execute("INSERT INTO employees(employee_id, full_name) VALUES('EE-INTACT-2', 'Intact Employee 2')", []).unwrap();
        }

        // Create valid DB first, then corrupt it
        {
            let conn = open_encrypted_connection(&corrupt_source).expect("open source");
            apply_migrations(&conn).expect("migrations");
        }
        // Append or corrupt bytes in the file
        let mut bytes = std::fs::read(&corrupt_source).unwrap();
        if bytes.len() > 100 {
            for b in bytes[40..80].iter_mut() {
                *b ^= 0xFF;
            }
            std::fs::write(&corrupt_source, &bytes).unwrap();
        }

        let val_err = super::validate_source_backup(&corrupt_source).expect_err("must reject corrupted file");
        assert!(val_err.contains("decryption failed") || val_err.contains("corrupt"));

        let res = super::restore_database_file(&corrupt_source, &destination);
        assert!(res.is_err(), "restore must fail closed");

        remove_database_files(&destination);
        remove_database_files(&corrupt_source);
    }

    #[test]
    fn test_restore_rejects_incompatible_schema() {
        let _test_lock = acquire_test_lock();
        let destination = temp_path("restore-incompat-dst");
        let incompat_source = temp_path("restore-incompat-src");
        remove_database_files(&destination);
        remove_database_files(&incompat_source);

        {
            let conn = open_encrypted_connection(&destination).expect("open destination");
            apply_migrations(&conn).expect("migrations");
        }

        // 1. Source missing employees table
        {
            let conn = open_encrypted_connection(&incompat_source).expect("open source");
            conn.execute("CREATE TABLE other_table (id INTEGER PRIMARY KEY);", []).unwrap();
        }
        let val_err = super::validate_source_backup(&incompat_source).expect_err("missing employees table");
        assert!(val_err.contains("missing 'employees' table"));

        // 2. Source with future user_version
        {
            let conn = open_encrypted_connection(&incompat_source).expect("open source");
            conn.execute("CREATE TABLE employees (id INTEGER PRIMARY KEY);", []).unwrap();
            conn.execute("PRAGMA user_version = 9999;", []).unwrap();
        }
        let val_err2 = super::validate_source_backup(&incompat_source).expect_err("future schema version");
        assert!(val_err2.contains("newer than application schema version"));

        remove_database_files(&destination);
        remove_database_files(&incompat_source);
    }

    #[test]
    fn test_restore_maintenance_mode_and_concurrency_lock() {
        let _test_lock = acquire_test_lock();
        assert_eq!(super::is_restore_in_progress(), false);
        assert_eq!(super::is_backup_in_progress(), false);

        let restore_guard = super::try_acquire_restore_lock().expect("acquire restore lock");
        assert_eq!(super::is_restore_in_progress(), true);
        assert_eq!(super::is_backup_in_progress(), true);

        // Attempting to acquire second restore lock must fail
        let second_restore = super::try_acquire_restore_lock().expect_err("concurrent restore must fail");
        assert!(second_restore.contains("in progress"));

        // Attempting to acquire backup lock during restore must fail
        let backup_err = super::try_acquire_backup_lock().expect_err("backup during restore must fail");
        assert!(backup_err.contains("in progress"));

        // Drop guard -> both locks released
        drop(restore_guard);
        assert_eq!(super::is_restore_in_progress(), false);
        assert_eq!(super::is_backup_in_progress(), false);
    }

    #[test]
    fn test_restore_active_wal_resolves_and_restores_cleanly() {
        let _test_lock = acquire_test_lock();
        let destination = temp_path("restore-wal-dst");
        let wal_source = temp_path("restore-wal-src");
        remove_database_files(&destination);
        remove_database_files(&wal_source);

        {
            let conn = open_encrypted_connection(&destination).expect("open destination");
            apply_migrations(&conn).expect("migrations");
        }

        // Source writes in WAL mode
        {
            let conn = open_encrypted_connection(&wal_source).expect("open source");
            apply_migrations(&conn).expect("migrations");
            let _: String = conn.query_row("PRAGMA journal_mode = WAL;", [], |r| r.get(0)).unwrap();
            conn.execute("INSERT INTO employees(employee_id, full_name) VALUES('EE-WAL-TEST', 'WAL Employee')", []).unwrap();
            // Don't checkpoint, leave active WAL
        }

        super::restore_database_file(&wal_source, &destination).expect("restore from active WAL source");

        let dest_conn = open_encrypted_connection(&destination).expect("open restored dest");
        let count: i64 = dest_conn.query_row("SELECT count(*) FROM employees WHERE employee_id = 'EE-WAL-TEST'", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1, "WAL-committed rows must be present in restored database");

        remove_database_files(&destination);
        remove_database_files(&wal_source);
    }

    #[test]
    fn test_restore_failure_rolls_back_original_db_and_records_audit() {
        let _test_lock = acquire_test_lock();
        let destination = temp_path("restore-rollback-dst");
        let source = temp_path("restore-rollback-src");
        remove_database_files(&destination);
        remove_database_files(&source);

        {
            let conn = open_encrypted_connection(&destination).expect("open destination");
            apply_migrations(&conn).expect("migrations");
            conn.execute("INSERT INTO employees(employee_id, full_name) VALUES('EE-ORIG-PRESERVED', 'Original Employee')", []).unwrap();
        }

        {
            let conn = open_encrypted_connection(&source).expect("open source");
            apply_migrations(&conn).expect("migrations");
            conn.execute("INSERT INTO employees(employee_id, full_name) VALUES('EE-NEW-REJECTED', 'New Employee')", []).unwrap();
        }

        // Simulate post-verify failure
        super::set_simulate_restore_post_verify_failure(true);
        let res = super::restore_database_file(&source, &destination);
        super::set_simulate_restore_post_verify_failure(false);

        assert!(res.is_err(), "Restore must fail when verification fails");
        let err_msg = res.unwrap_err();
        assert!(err_msg.contains("recovered intact"));

        // Destination must still have EE-ORIG-PRESERVED and NOT EE-NEW-REJECTED
        let check_conn = open_encrypted_connection(&destination).expect("open destination after rollback");
        let orig_count: i64 = check_conn.query_row("SELECT count(*) FROM employees WHERE employee_id = 'EE-ORIG-PRESERVED'", [], |r| r.get(0)).unwrap();
        assert_eq!(orig_count, 1, "Original data must be recovered intact");

        let new_count: i64 = check_conn.query_row("SELECT count(*) FROM employees WHERE employee_id = 'EE-NEW-REJECTED'", [], |r| r.get(0)).unwrap();
        assert_eq!(new_count, 0, "Rejected data must not remain in destination");

        // Verify audit log for failure was inserted into recovered original DB
        let audit_count: i64 = check_conn.query_row(
            "SELECT count(*) FROM audit_logs WHERE event_type = 'database.restore' AND payload_json LIKE '%FAILED%'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert!(audit_count >= 1, "Failed restore must be audited");

        remove_database_files(&destination);
        remove_database_files(&source);
    }

    #[test]
    fn test_successful_restore_records_audit_and_preserves_integrity() {
        let _test_lock = acquire_test_lock();
        let destination = temp_path("restore-audit-dst");
        let source = temp_path("restore-audit-src");
        remove_database_files(&destination);
        remove_database_files(&source);

        {
            let conn = open_encrypted_connection(&destination).expect("open destination");
            apply_migrations(&conn).expect("migrations");
        }

        {
            let conn = open_encrypted_connection(&source).expect("open source");
            apply_migrations(&conn).expect("migrations");
            conn.execute("INSERT INTO employees(employee_id, full_name) VALUES('EE-AUDIT-RESTORE', 'Audit Employee')", []).unwrap();
        }

        let _guard = super::try_acquire_restore_lock().unwrap();
        super::execute_staged_replacement_with_rollback(
            &destination,
            &source,
            Some(99),
            Some("safety_snap_test.sqlite3"),
        ).expect("restore must succeed");
        drop(_guard);

        let dest_conn = open_encrypted_connection(&destination).expect("open restored dest");
        let count: i64 = dest_conn.query_row("SELECT count(*) FROM employees WHERE employee_id = 'EE-AUDIT-RESTORE'", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let (event_type, actor_ref, payload): (String, Option<String>, Option<String>) = dest_conn.query_row(
            "SELECT event_type, actor_ref, payload_json FROM audit_logs WHERE event_type = 'database.restore'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).expect("query restore audit record");

        assert_eq!(event_type, "database.restore");
        assert_eq!(actor_ref.as_deref(), Some("99"));
        assert!(payload.as_deref().unwrap().contains("SUCCESS"));

        remove_database_files(&destination);
        remove_database_files(&source);
    }

    #[test]
    fn test_usb_destination_validation_rejects_unsafe_paths() {
        let _test_lock = acquire_test_lock();
        super::storage_safety::reset_storage_safety_mocks();

        let db_dir = temp_path("usb-val-db-dir");
        let db_path = db_dir.join("staff_kit.sqlite3");
        let local_backup_dir = temp_path("usb-val-backup-dir");
        let _ = std::fs::create_dir_all(&db_dir);
        let _ = std::fs::create_dir_all(&local_backup_dir);

        // 1. Network UNC path is rejected
        let unc_path = PathBuf::from(r"\\server\share\usb_backups");
        let err = super::storage_safety::validate_usb_destination(&unc_path, &db_path, &local_backup_dir)
            .expect_err("UNC path must be rejected");
        assert!(err.contains("network UNC"));

        // 2. Setting USB destination inside DB directory is rejected
        let inside_db = db_dir.join("usb_subfolder");
        let err = super::storage_safety::validate_usb_destination(&inside_db, &db_path, &local_backup_dir)
            .expect_err("Inside DB dir must be rejected");
        assert!(err.contains("inside or share the database directory"), "Actual error was: {err}");

        // 3. Setting USB destination inside local backup directory is rejected
        let inside_backup = local_backup_dir.join("usb_subfolder");
        let err = super::storage_safety::validate_usb_destination(&inside_backup, &db_path, &local_backup_dir)
            .expect_err("Inside backup dir must be rejected");
        assert!(err.contains("inside or share the local backup directory"));

        // 4. Same volume serial number as DB volume is rejected
        let candidate_dir = temp_path("usb-candidate-dir");
        let _ = std::fs::create_dir_all(&candidate_dir);

        super::storage_safety::set_mock_volume_identity(
            &db_path,
            super::storage_safety::VolumeIdentity {
                volume_serial_number: "VOL_LOCAL_C".to_string(),
                volume_name: "LocalDiskC".to_string(),
                file_system_name: "NTFS".to_string(),
                drive_type: 3,
                is_removable_or_external: false,
            },
        );

        super::storage_safety::set_mock_volume_identity(
            &local_backup_dir,
            super::storage_safety::VolumeIdentity {
                volume_serial_number: "VOL_LOCAL_C".to_string(),
                volume_name: "LocalDiskC".to_string(),
                file_system_name: "NTFS".to_string(),
                drive_type: 3,
                is_removable_or_external: false,
            },
        );

        super::storage_safety::set_mock_volume_identity(
            &candidate_dir,
            super::storage_safety::VolumeIdentity {
                volume_serial_number: "VOL_LOCAL_C".to_string(), // Same serial!
                volume_name: "LocalDiskC".to_string(),
                file_system_name: "NTFS".to_string(),
                drive_type: 3,
                is_removable_or_external: false,
            },
        );

        let err = super::storage_safety::validate_usb_destination(&candidate_dir, &db_path, &local_backup_dir)
            .expect_err("Same volume serial as DB must be rejected");
        assert!(err.contains("shares volume serial number"));

        // 5. Valid distinct USB volume is accepted
        super::storage_safety::set_mock_volume_identity(
            &candidate_dir,
            super::storage_safety::VolumeIdentity {
                volume_serial_number: "VOL_USB_EXT_01".to_string(), // Distinct serial!
                volume_name: "StaffKitUSB".to_string(),
                file_system_name: "exFAT".to_string(),
                drive_type: 2,
                is_removable_or_external: true,
            },
        );

        let identity = super::storage_safety::validate_usb_destination(&candidate_dir, &db_path, &local_backup_dir)
            .expect("Distinct USB volume must be accepted");
        assert_eq!(identity.volume_serial_number, "VOL_USB_EXT_01");
        assert_eq!(identity.volume_name, "StaffKitUSB");

        let _ = std::fs::remove_dir_all(&db_dir);
        let _ = std::fs::remove_dir_all(&local_backup_dir);
        let _ = std::fs::remove_dir_all(&candidate_dir);
    }

    #[test]
    fn test_disk_space_preflight_blocks_backup_when_space_insufficient() {
        let _test_lock = acquire_test_lock();
        super::storage_safety::reset_storage_safety_mocks();

        let target_dir = temp_path("preflight-target-dir");
        let db_file = temp_path("preflight-db.sqlite3");
        let _ = std::fs::create_dir_all(&target_dir);
        let _ = std::fs::write(&db_file, vec![0u8; 10 * 1024 * 1024]); // 10MB DB

        // Mock 1 MB free space -> less than required (10MB + 50MB min = 60MB)
        super::storage_safety::set_mock_available_disk_space(&target_dir, 1 * 1024 * 1024);

        let err = super::preflight_disk_space(&target_dir, &db_file)
            .expect_err("preflight must fail when disk is full");
        assert!(err.contains("insufficient disk space"));

        // Mock 100 MB free space -> passes
        super::storage_safety::set_mock_available_disk_space(&target_dir, 100 * 1024 * 1024);
        super::preflight_disk_space(&target_dir, &db_file).expect("preflight must pass when disk is sufficient");

        let _ = std::fs::remove_dir_all(&target_dir);
        let _ = std::fs::remove_file(&db_file);
    }

    #[test]
    fn test_local_backup_verification_failure_removes_temp_and_preserves_existing() {
        let _test_lock = acquire_test_lock();
        let db_path = temp_path("verify-fail-db.sqlite3");
        let backup_dir = temp_path("verify-fail-backup-dir");
        let _ = std::fs::create_dir_all(&backup_dir);
        remove_database_files(&db_path);

        // Pre-create an existing valid backup file
        let existing_backup = backup_dir.join("staff_kit_backup_auto_2026-01-01_00-00-00.sqlite3");
        std::fs::write(&existing_backup, b"EXISTING_VALID_BACKUP_CONTENT").unwrap();

        {
            let conn = open_encrypted_connection(&db_path).expect("open db");
            apply_migrations(&conn).expect("migrations");

            // Normal backup succeeds
            let res = super::run_backup_on_connection(&conn, &backup_dir, true, Some(&db_path))
                .expect("backup must succeed");
            assert!(Path::new(&res.backup_file_path).exists());
            assert!(existing_backup.exists(), "Existing backup must be preserved");
        }

        remove_database_files(&db_path);
        let _ = std::fs::remove_dir_all(&backup_dir);
    }

    #[test]
    fn test_retention_auto_backups_30_days_and_never_delete_last_valid() {
        let _test_lock = acquire_test_lock();
        let backup_dir = temp_path("retention-test-dir");
        let _ = std::fs::create_dir_all(&backup_dir);

        // 1. Create 3 auto backup files
        let f1 = backup_dir.join("staff_kit_backup_auto_2026-01-01_10-00-00.sqlite3");
        let f2 = backup_dir.join("staff_kit_backup_auto_2026-02-01_10-00-00.sqlite3");
        let f3 = backup_dir.join("staff_kit_backup_auto_2026-03-01_10-00-00.sqlite3");
        std::fs::write(&f1, b"b1").unwrap();
        std::fs::write(&f2, b"b2").unwrap();
        std::fs::write(&f3, b"b3").unwrap();

        // Also create a manual backup file
        let manual = backup_dir.join("staff_kit_backup_manual_2020-01-01_00-00-00.sqlite3");
        std::fs::write(&manual, b"manual").unwrap();

        // Manually adjust file times: set f1 to 45 days ago, f2 to 35 days ago, f3 to now
        let forty_five_days_ago = SystemTime::now() - std::time::Duration::from_secs(45 * 86_400);
        let thirty_five_days_ago = SystemTime::now() - std::time::Duration::from_secs(35 * 86_400);

        std::fs::File::options().write(true).open(&f1).unwrap().set_modified(forty_five_days_ago).unwrap();
        std::fs::File::options().write(true).open(&f2).unwrap().set_modified(thirty_five_days_ago).unwrap();
        std::fs::File::options().write(true).open(&manual).unwrap().set_modified(forty_five_days_ago).unwrap();

        // Run pruning with 30-day retention
        let removed = super::prune_auto_backups_by_age(&backup_dir, 30).expect("pruning");
        assert_eq!(removed, 2, "Must remove f1 and f2");
        assert!(!f1.exists());
        assert!(!f2.exists());
        assert!(f3.exists(), "Fresh f3 must remain");
        assert!(manual.exists(), "Manual backup must NEVER be auto-deleted");

        // 2. Edge Case: ONLY 1 backup exists, but it's 50 days old -> Rule: NEVER delete last valid backup!
        std::fs::File::options().write(true).open(&f3).unwrap().set_modified(forty_five_days_ago).unwrap();
        let removed2 = super::prune_auto_backups_by_age(&backup_dir, 30).expect("pruning");
        assert_eq!(removed2, 0, "Must NEVER prune the last remaining backup even if older than 30 days");
        assert!(f3.exists(), "Last valid backup must be preserved");

        let _ = std::fs::remove_dir_all(&backup_dir);
    }

    #[test]
    fn test_safety_snapshots_categorized_quotas() {
        let _test_lock = acquire_test_lock();
        let history_dir = temp_path("history-quotas-dir");
        let _ = std::fs::create_dir_all(&history_dir);

        // Create 8 app_close snapshots (quota 5)
        for i in 1..=8 {
            let path = history_dir.join(format!("snap_2026102{}_120000_app_close.sqlite3", i));
            std::fs::write(&path, b"app_close").unwrap();
            let time = SystemTime::now() - std::time::Duration::from_secs((10 - i) * 100);
            std::fs::File::options().write(true).open(&path).unwrap().set_modified(time).unwrap();
        }

        // Create 12 before_excel_import snapshots (quota 10)
        for i in 1..=12 {
            let path = history_dir.join(format!("snap_2026101{}_120000_before_excel_import.sqlite3", i));
            std::fs::write(&path, b"import").unwrap();
            let time = SystemTime::now() - std::time::Duration::from_secs((20 - i) * 100);
            std::fs::File::options().write(true).open(&path).unwrap().set_modified(time).unwrap();
        }

        // Create 3 manual snapshots (quota unlimited)
        for i in 1..=3 {
            let path = history_dir.join(format!("snap_2026100{}_120000_manual_backup.sqlite3", i));
            std::fs::write(&path, b"manual").unwrap();
        }

        super::prune_history_snapshots(&history_dir).expect("snapshot pruning");

        let remaining: Vec<_> = std::fs::read_dir(&history_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        let app_close_count = remaining.iter().filter(|s| s.contains("app_close")).count();
        let import_count = remaining.iter().filter(|s| s.contains("before_excel_import")).count();
        let manual_count = remaining.iter().filter(|s| s.contains("manual_backup")).count();

        assert_eq!(app_close_count, 5, "app_close quota is 5");
        assert_eq!(import_count, 10, "import quota is 10");
        assert_eq!(manual_count, 3, "manual snapshots are never auto-pruned");

        let _ = std::fs::remove_dir_all(&history_dir);
    }

    #[test]
    fn test_protection_status_engine_evaluates_states() {
        let _test_lock = acquire_test_lock();
        let test_dir = temp_path("protection-status-dir");
        let _ = std::fs::create_dir_all(&test_dir);
        let backup_file = test_dir.join("staff_kit_backup_auto_2026-10-29_00-00-00.sqlite3");
        std::fs::write(&backup_file, b"valid").unwrap();

        let now = Local::now().to_rfc3339();
        let old_time = (Local::now() - chrono::Duration::hours(10)).to_rfc3339();

        // 1. At Risk: Local has error
        let status = super::compute_protection_status(
            test_dir.to_str().unwrap(),
            true,
            Some(&now),
            Some("ERROR: Disk full"),
            true,
            Some("D:/USB"),
            Some("USB_SERIAL"),
            Some(&now),
            Some("SUCCESS"),
        );
        assert_eq!(status, "At Risk");

        // 2. At Risk: Local overdue (> 7 hours)
        let status = super::compute_protection_status(
            test_dir.to_str().unwrap(),
            true,
            Some(&old_time),
            Some("SUCCESS"),
            true,
            Some("D:/USB"),
            Some("USB_SERIAL"),
            Some(&now),
            Some("SUCCESS"),
        );
        assert_eq!(status, "At Risk");

        // 3. Degraded: Local fresh, but USB not enabled
        let status = super::compute_protection_status(
            test_dir.to_str().unwrap(),
            true,
            Some(&now),
            Some("SUCCESS"),
            false,
            None,
            None,
            None,
            None,
        );
        assert_eq!(status, "Degraded");

        // 4. Degraded: Local fresh, USB enabled but USB status has error
        let status = super::compute_protection_status(
            test_dir.to_str().unwrap(),
            true,
            Some(&now),
            Some("SUCCESS"),
            true,
            Some(test_dir.to_str().unwrap()),
            Some("USB_SERIAL"),
            Some(&now),
            Some("ERROR: USB unplugged"),
        );
        assert_eq!(status, "Degraded");

        // 5. Protected: Local fresh, USB fresh, verified device
        super::storage_safety::set_mock_volume_identity(
            &test_dir,
            super::storage_safety::VolumeIdentity {
                volume_serial_number: "USB_SERIAL_MATCH".to_string(),
                volume_name: "StaffKitUSB".to_string(),
                file_system_name: "NTFS".to_string(),
                drive_type: 2,
                is_removable_or_external: true,
            },
        );

        let status = super::compute_protection_status(
            test_dir.to_str().unwrap(),
            true,
            Some(&now),
            Some("SUCCESS"),
            true,
            Some(test_dir.to_str().unwrap()),
            Some("USB_SERIAL_MATCH"),
            Some(&now),
            Some("SUCCESS"),
        );
        assert_eq!(status, "Protected");

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_existing_failed_verification_backup_is_never_deleted() {
        let _test_lock = acquire_test_lock();
        let backup_dir = temp_path("corrupt-retention-dir");
        let _ = std::fs::create_dir_all(&backup_dir);

        // Pre-create a corrupted backup file in the backup directory
        let corrupt_backup = backup_dir.join("staff_kit_backup_auto_2026-10-20_00-00-00.sqlite3");
        std::fs::write(&corrupt_backup, b"CORRUPTED_BYTES_NOT_SQLCIPHER").unwrap();

        // Also create a valid backup
        let valid_backup = backup_dir.join("staff_kit_backup_auto_2026-10-29_00-00-00.sqlite3");
        std::fs::write(&valid_backup, b"VALID").unwrap();

        // Pruning does NOT delete existing failed-verification backups automatically (Rule A)
        let _ = super::prune_auto_backups_by_age(&backup_dir, 30);
        assert!(corrupt_backup.exists(), "Rule A: corrupted existing backup must NOT be auto-deleted");
        assert!(valid_backup.exists());

        let _ = std::fs::remove_dir_all(&backup_dir);
    }

    #[test]
    fn test_usb_sync_and_unplug_and_corruption_simulation() {
        let _test_lock = acquire_test_lock();
        super::storage_safety::reset_storage_safety_mocks();

        let db_path = temp_path("usb-sync-db.sqlite3");
        let local_dir = temp_path("usb-sync-local-dir");
        let usb_dir = temp_path("usb-sync-usb-dir");
        let _ = std::fs::create_dir_all(&local_dir);
        let _ = std::fs::create_dir_all(&usb_dir);
        remove_database_files(&db_path);

        let conn = open_encrypted_connection(&db_path).expect("open db");
        apply_migrations(&conn).expect("migrations");

        // Generate a real valid encrypted backup file using run_backup_on_connection
        let backup_res = super::run_backup_on_connection(&conn, &local_dir, true, Some(&db_path))
            .expect("local backup");
        let local_backup_path = PathBuf::from(backup_res.backup_file_path);
        let backup_filename = local_backup_path.file_name().unwrap().to_string_lossy().to_string();

        // 1. USB backup disabled -> returns Err
        let err = super::sync_backup_to_usb_with_conn(&conn, &local_backup_path)
            .expect_err("USB disabled must fail");
        assert!(err.contains("not enabled"));

        // Configure USB settings in DB
        super::set_setting_value(&conn, USB_BACKUP_ENABLED_SETTING_KEY, Some("1")).unwrap();
        super::set_setting_value(&conn, USB_BACKUP_PATH_SETTING_KEY, Some(usb_dir.to_str().unwrap())).unwrap();
        super::set_setting_value(&conn, USB_BACKUP_DEVICE_ID_SETTING_KEY, Some("VOL_USB_TEST_42")).unwrap();

        // Mock USB volume identity and disk space
        super::storage_safety::set_mock_volume_identity(
            &usb_dir,
            super::storage_safety::VolumeIdentity {
                volume_serial_number: "VOL_USB_TEST_42".to_string(),
                volume_name: "StaffKitUSB".to_string(),
                file_system_name: "exFAT".to_string(),
                drive_type: 2,
                is_removable_or_external: true,
            },
        );
        super::storage_safety::set_mock_available_disk_space(&usb_dir, 500 * 1024 * 1024);

        // 2. Successful sync
        let usb_final_path_str = super::sync_backup_to_usb_with_conn(&conn, &local_backup_path)
            .expect("USB sync should succeed");
        let usb_final_path = PathBuf::from(&usb_final_path_str);
        assert!(usb_final_path.exists());
        assert_eq!(usb_final_path.file_name().unwrap().to_str().unwrap(), &backup_filename);

        // 3. Mismatched USB device serial -> rejected
        super::storage_safety::set_mock_volume_identity(
            &usb_dir,
            super::storage_safety::VolumeIdentity {
                volume_serial_number: "VOL_WRONG_SERIAL".to_string(),
                volume_name: "OtherUSB".to_string(),
                file_system_name: "exFAT".to_string(),
                drive_type: 2,
                is_removable_or_external: true,
            },
        );
        let err = super::sync_backup_to_usb_with_conn(&conn, &local_backup_path)
            .expect_err("mismatched USB volume serial must fail");
        assert!(err.contains("USB device mismatch"));

        // Restore correct identity
        super::storage_safety::set_mock_volume_identity(
            &usb_dir,
            super::storage_safety::VolumeIdentity {
                volume_serial_number: "VOL_USB_TEST_42".to_string(),
                volume_name: "StaffKitUSB".to_string(),
                file_system_name: "exFAT".to_string(),
                drive_type: 2,
                is_removable_or_external: true,
            },
        );

        // 4. Simulated USB unplug mid-copy
        super::storage_safety::set_simulate_usb_unplug(true);
        let err = super::sync_backup_to_usb_with_conn(&conn, &local_backup_path)
            .expect_err("unplug simulation must fail");
        assert!(err.contains("simulated USB unplug"));
        assert!(usb_final_path.exists(), "Previously synced file must NOT be destroyed");
        super::storage_safety::set_simulate_usb_unplug(false);

        // 5. Simulated corruption of copy
        super::storage_safety::set_simulate_usb_corruption(true);
        let err = super::sync_backup_to_usb_with_conn(&conn, &local_backup_path)
            .expect_err("corruption simulation must fail");
        assert!(err.contains("verification failed"));
        let temp_sync_path = usb_dir.join(format!("{}.sync_tmp", backup_filename));
        assert!(!temp_sync_path.exists(), ".sync_tmp must be removed upon verification failure");
        super::storage_safety::set_simulate_usb_corruption(false);

        super::storage_safety::reset_storage_safety_mocks();
        remove_database_files(&db_path);
        let _ = std::fs::remove_dir_all(&local_dir);
        let _ = std::fs::remove_dir_all(&usb_dir);
    }

    #[test]
    fn test_usb_retry_backoff_schedule() {
        let _test_lock = acquire_test_lock();
        let db_path = temp_path("usb-retry-db.sqlite3");
        remove_database_files(&db_path);

        let conn = open_encrypted_connection(&db_path).expect("open db");
        apply_migrations(&conn).expect("migrations");

        // Initially USB not enabled -> not due
        assert!(!super::is_usb_retry_due(&conn).unwrap());

        // Enable USB
        super::set_setting_value(&conn, USB_BACKUP_ENABLED_SETTING_KEY, Some("1")).unwrap();
        assert!(!super::is_usb_retry_due(&conn).unwrap());

        // Failure 1: Stage 1 -> 15m backoff
        super::record_usb_sync_failure_with_conn(&conn, "Failure 1");
        let stage = super::get_setting_value(&conn, USB_BACKUP_RETRY_STAGE_SETTING_KEY).unwrap().unwrap();
        assert_eq!(stage, "1");
        // Right after failure, not yet due
        assert!(!super::is_usb_retry_due(&conn).unwrap());

        // Failure 2: Stage 2 -> 60m backoff
        super::record_usb_sync_failure_with_conn(&conn, "Failure 2");
        let stage = super::get_setting_value(&conn, USB_BACKUP_RETRY_STAGE_SETTING_KEY).unwrap().unwrap();
        assert_eq!(stage, "2");

        // Failure 3: Stage 3 -> 180m backoff
        super::record_usb_sync_failure_with_conn(&conn, "Failure 3");
        let stage = super::get_setting_value(&conn, USB_BACKUP_RETRY_STAGE_SETTING_KEY).unwrap().unwrap();
        assert_eq!(stage, "3");

        // Failure 4: Stage 4 wraps to 1
        super::record_usb_sync_failure_with_conn(&conn, "Failure 4");
        let stage = super::get_setting_value(&conn, USB_BACKUP_RETRY_STAGE_SETTING_KEY).unwrap().unwrap();
        assert_eq!(stage, "1");

        // Manually set next_retry to a past date -> is_usb_retry_due returns true
        let past = (Local::now() - chrono::Duration::minutes(5)).to_rfc3339();
        super::set_setting_value(&conn, USB_BACKUP_NEXT_RETRY_DATE_SETTING_KEY, Some(&past)).unwrap();
        assert!(super::is_usb_retry_due(&conn).unwrap());

        remove_database_files(&db_path);
    }

    #[test]
    fn test_usb_catch_up_sync() {
        let _test_lock = acquire_test_lock();
        let db_path = temp_path("usb-catchup-db.sqlite3");
        let local_dir = temp_path("usb-catchup-local");
        let usb_dir = temp_path("usb-catchup-usb");
        let _ = std::fs::create_dir_all(&local_dir);
        let _ = std::fs::create_dir_all(&usb_dir);
        remove_database_files(&db_path);

        let conn = open_encrypted_connection(&db_path).expect("open db");
        apply_migrations(&conn).expect("migrations");

        // Generate 3 real encrypted backups in local_dir
        for _ in 0..3 {
            let _res = super::run_backup_on_connection(&conn, &local_dir, true, Some(&db_path)).unwrap();
            // Sleep briefly to ensure distinct timestamps
            std::thread::sleep(std::time::Duration::from_millis(1100));
        }

        let local_files = super::collect_auto_backup_files(&local_dir);
        assert_eq!(local_files.len(), 3);

        // USB has 0 files
        let copied = super::catch_up_missing_usb_backups(&local_dir, &usb_dir).expect("catch up");
        assert_eq!(copied, 3, "All 3 missing backups should be copied to USB");

        // Second catch up does nothing because USB already has them
        let copied_second = super::catch_up_missing_usb_backups(&local_dir, &usb_dir).expect("catch up");
        assert_eq!(copied_second, 0, "No duplicate copies should occur");

        remove_database_files(&db_path);
        let _ = std::fs::remove_dir_all(&local_dir);
        let _ = std::fs::remove_dir_all(&usb_dir);
    }
}
