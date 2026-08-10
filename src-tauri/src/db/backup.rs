use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::Local;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use super::schema::{
    AUTO_BACKUP_ENABLED_SETTING_KEY, AUTO_BACKUP_INTERVAL_DAYS, AUTO_BACKUP_RETENTION_DAYS,
    AUTO_BACKUP_RETENTION_FILES, BACKUP_DIRECTORY_SETTING_KEY, BACKUP_FILE_PREFIX, DB_FILE_NAME,
    DB_SETTINGS_FILE_NAME, HISTORY_FILE_PREFIX, HISTORY_FOLDER_NAME, HISTORY_RETENTION_COUNT,
};
use super::{get_setting_value, open_runtime_connection, require_text, set_setting_value};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatus {
    pub initialized: bool,
    pub db_path: String,
    pub sqlite_version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettings {
    pub backup_directory_path: String,
    pub auto_backup_enabled: bool,
    pub retention_files: i64,
    pub auto_backup_interval_days: i64,
    pub auto_backup_retention_days: i64,
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

#[derive(Debug, Serialize, Deserialize, Default)]
struct DbSettings {
    custom_path: Option<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn get_backup_settings(app: &AppHandle) -> Result<BackupSettings, String> {
    let conn = open_runtime_connection(app)?;
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
    write_db_settings(app, &settings)
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

        fs::copy(&current_path, &target_path)
            .map_err(|err| format!("failed to copy database to new location: {err}"))?;

        set_db_custom_path(app, Some(target_folder))?;

        Ok(format!("COPIED:{}", target_path.to_string_lossy()))
    }
}

// ── History snapshots ─────────────────────────────────────────────────────────

/// Create a snapshot of the current DB. `label` is a short descriptor like
/// "before_import" or "app_close". Keeps at most HISTORY_RETENTION_COUNT files.
pub fn create_history_snapshot(app: &AppHandle, label: &str) -> Result<SnapshotInfo, String> {
    let db_path = super::resolve_database_path(app)?;
    let history_dir = resolve_history_dir(&db_path)?;

    fs::create_dir_all(&history_dir)
        .map_err(|err| format!("failed to create history directory: {err}"))?;

    let now = Local::now();
    let timestamp_str = now.format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("{HISTORY_FILE_PREFIX}_{timestamp_str}_{label}.sqlite3");
    let snapshot_path = history_dir.join(&filename);

    // Checkpoint WAL so snapshot is complete
    {
        let conn = open_runtime_connection(app)?;
        conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
            .map_err(|err| format!("failed to checkpoint WAL before snapshot: {err}"))?;
        let escaped = snapshot_path.to_string_lossy().replace('\'', "''");
        conn.execute_batch(&format!("VACUUM INTO '{escaped}';"))
            .map_err(|err| format!("failed to create snapshot: {err}"))?;
    }

    prune_history_snapshots(&history_dir)?;

    let size_mb = fs::metadata(&snapshot_path)
        .map(|m| m.len() as f64 / 1_048_576.0)
        .unwrap_or(0.0);

    Ok(SnapshotInfo {
        filename,
        label: label.to_string(),
        timestamp: now.format("%Y-%m-%d %H:%M").to_string(),
        size_mb: (size_mb * 100.0).round() / 100.0,
        full_path: snapshot_path.to_string_lossy().to_string(),
    })
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

/// Restores a snapshot: saves current state as "before_restore", then replaces active DB.
pub fn restore_history_snapshot(app: &AppHandle, filename: &str) -> Result<(), String> {
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

    // Save current state before restore
    let _ = create_history_snapshot(app, "before_restore");

    // Close connections by checkpointing WAL before replacing the main file.
    let conn = open_runtime_connection(app)?;
    conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
        .map_err(|err| format!("failed to checkpoint WAL before restore: {err}"))?;
    drop(conn);

    replace_database_from_history_snapshot(&snapshot_path, &db_path)
}

/// Restores the database from any arbitrary backup file chosen by the user.
/// Uses SQLite's Online Backup API to safely copy data while connections may be live.
pub fn restore_database_from_file(app: &AppHandle, source_path: &str) -> Result<(), String> {
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

    // Open and validate the source backup (confirm it is a valid Staff Kit DB)
    super::open_encrypted_connection(source).map_err(|_| {
        "the selected database file could not be opened. Make sure it is a valid Staff Kit database.".to_string()
    })?;

    // Save a snapshot of the current state before restore
    let _ = create_history_snapshot(app, "before_restore");

    // Open the active DB and fully checkpoint WAL so all pages are in the main file
    let db_path = super::resolve_database_path(app)?;
    restore_database_file(source, &db_path)
}

fn migrate_restored_database(path: &Path) -> Result<(), String> {
    let conn = super::open_encrypted_connection(path)
        .map_err(|err| format!("restored database could not be reopened: {err}"))?;
    super::apply_migrations(&conn)
        .map_err(|err| format!("restored database migration failed: {err}"))
}

fn replace_database_from_history_snapshot(
    snapshot_path: &Path,
    database_path: &Path,
) -> Result<(), String> {
    fs::copy(snapshot_path, database_path)
        .map_err(|err| format!("failed to restore snapshot: {err}"))?;
    migrate_restored_database(database_path)
}

fn restore_database_file(source_path: &Path, destination_path: &Path) -> Result<(), String> {
    use rusqlite::backup::Backup;
    use std::time::Duration;

    let src_conn = super::open_encrypted_connection(source_path)
        .map_err(|err| format!("failed to open restore source: {err}"))?;
    let mut dst_conn = super::open_encrypted_connection(destination_path)
        .map_err(|err| format!("failed to open restore destination: {err}"))?;
    dst_conn
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|err| format!("failed to checkpoint active database: {err}"))?;

    // SQLite Online Backup API: stream every page from src → dst.
    // This safely handles Windows file locking (os error 32) that fs::copy cannot.
    let backup = Backup::new(&src_conn, &mut dst_conn)
        .map_err(|err| format!("failed to initialize database restore: {err}"))?;
    backup
        .run_to_completion(5, Duration::from_millis(0), None)
        .map_err(|err| format!("failed to restore database from file: {err}"))?;
    drop(backup);
    drop(dst_conn);
    drop(src_conn);

    // The source may predate Borrow/Return. Migrate before restore success is
    // reported or the caller invalidates the current session.
    migrate_restored_database(destination_path)
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

    Ok(BackupSettings {
        backup_directory_path,
        auto_backup_enabled,
        retention_files: AUTO_BACKUP_RETENTION_FILES as i64,
        auto_backup_interval_days: AUTO_BACKUP_INTERVAL_DAYS,
        auto_backup_retention_days: AUTO_BACKUP_RETENTION_DAYS,
    })
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

fn run_backup(
    app: &AppHandle,
    backup_directory_path: &str,
    automatic: bool,
) -> Result<BackupRunResult, String> {
    let backup_dir = PathBuf::from(backup_directory_path);
    fs::create_dir_all(&backup_dir)
        .map_err(|err| format!("failed to create backup directory: {err}"))?;

    let now = Local::now();
    let file_name = if automatic {
        format!(
            "{BACKUP_FILE_PREFIX}_auto_{}.sqlite3",
            now.format("%Y-%m-%d")
        )
    } else {
        format!(
            "{BACKUP_FILE_PREFIX}_manual_{}.sqlite3",
            now.format("%Y-%m-%d_%H-%M-%S")
        )
    };

    let backup_file_path = backup_dir.join(file_name);
    if backup_file_path.exists() {
        fs::remove_file(&backup_file_path)
            .map_err(|err| format!("failed to replace existing backup file: {err}"))?;
    }

    let conn = open_runtime_connection(app)?;
    conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
        .map_err(|err| format!("failed to checkpoint sqlite WAL before backup: {err}"))?;

    let escaped_path = backup_file_path.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{escaped_path}';"))
        .map_err(|err| format!("failed to create backup file: {err}"))?;

    let retained_files = prune_old_backups(&backup_dir, AUTO_BACKUP_RETENTION_FILES)?;

    // Clean up backups older than 400 days
    let _ = prune_backups_by_age(&backup_dir, AUTO_BACKUP_RETENTION_DAYS);

    Ok(BackupRunResult {
        backup_file_path: backup_file_path.to_string_lossy().to_string(),
        retained_files: retained_files as i64,
        performed: true,
    })
}

fn prune_old_backups(backup_dir: &Path, retention: usize) -> Result<usize, String> {
    let mut files = collect_sqlite_files(backup_dir, BACKUP_FILE_PREFIX);
    files.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|meta| meta.modified()).ok();
        let b_time = b.metadata().and_then(|meta| meta.modified()).ok();
        b_time.cmp(&a_time)
    });

    if files.len() > retention {
        for path in files.iter().skip(retention) {
            let _ = fs::remove_file(path);
        }
    }

    Ok(files.len().min(retention))
}

fn prune_backups_by_age(backup_dir: &Path, max_age_days: i64) -> Result<(), String> {
    let cutoff = SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(
            (max_age_days as u64) * 86_400,
        ))
        .ok_or("overflow computing cutoff time")?;

    for path in collect_sqlite_files(backup_dir, BACKUP_FILE_PREFIX) {
        if let Ok(meta) = path.metadata() {
            if let Ok(modified) = meta.modified() {
                if modified < cutoff {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    Ok(())
}

fn prune_history_snapshots(history_dir: &Path) -> Result<(), String> {
    let mut files = collect_sqlite_files(history_dir, HISTORY_FILE_PREFIX);
    files.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time) // newest first
    });

    if files.len() > HISTORY_RETENTION_COUNT {
        for path in files.iter().skip(HISTORY_RETENTION_COUNT) {
            let _ = fs::remove_file(path);
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

    use rusqlite::params;

    use crate::db::schema::BASE_SCHEMA_SQL;
    use crate::db::{apply_migrations, open_encrypted_connection};

    use super::{
        migrate_restored_database, replace_database_from_history_snapshot, restore_database_file,
    };

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
}
