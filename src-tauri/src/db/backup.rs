use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::Local;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use super::{get_setting_value, open_runtime_connection, require_text, set_setting_value};
use super::schema::{
    AUTO_BACKUP_ENABLED_SETTING_KEY, AUTO_BACKUP_INTERVAL_DAYS, AUTO_BACKUP_LAST_DATE_SETTING_KEY,
    AUTO_BACKUP_RETENTION_DAYS, AUTO_BACKUP_RETENTION_FILES, BACKUP_DIRECTORY_SETTING_KEY,
    BACKUP_FILE_PREFIX, DB_FILE_NAME, DB_SETTINGS_FILE_NAME,
    HISTORY_FILE_PREFIX, HISTORY_FOLDER_NAME, HISTORY_RETENTION_COUNT,
};

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
        Some(if payload.auto_backup_enabled { "1" } else { "0" }),
    )?;

    read_backup_settings(&conn, app)
}

pub fn backup_database_now(app: &AppHandle) -> Result<BackupRunResult, String> {
    let conn = open_runtime_connection(app)?;
    let settings = read_backup_settings(&conn, app)?;
    run_backup(app, settings.backup_directory_path.as_str(), false)
}

pub fn run_auto_backup_if_due(app: &AppHandle) -> Result<BackupRunResult, String> {
    let conn = open_runtime_connection(app)?;
    let settings = read_backup_settings(&conn, app)?;
    if !settings.auto_backup_enabled {
        return Ok(BackupRunResult {
            backup_file_path: String::new(),
            retained_files: AUTO_BACKUP_RETENTION_FILES as i64,
            performed: false,
        });
    }

    let today = Local::now();
    let today_str = today.format("%Y-%m-%d").to_string();
    let last_auto_backup = get_setting_value(&conn, AUTO_BACKUP_LAST_DATE_SETTING_KEY)?
        .map(|value| value.trim().to_string())
        .unwrap_or_default();

    // Check: at least AUTO_BACKUP_INTERVAL_DAYS since last backup
    let is_due = if last_auto_backup.is_empty() {
        true
    } else {
        match chrono::NaiveDate::parse_from_str(&last_auto_backup, "%Y-%m-%d") {
            Ok(last_date) => {
                let diff = today.date_naive().signed_duration_since(last_date);
                diff.num_days() >= AUTO_BACKUP_INTERVAL_DAYS
            }
            Err(_) => true,
        }
    };

    if !is_due {
        return Ok(BackupRunResult {
            backup_file_path: String::new(),
            retained_files: AUTO_BACKUP_RETENTION_FILES as i64,
            performed: false,
        });
    }

    let result = run_backup(app, settings.backup_directory_path.as_str(), true)?;
    set_setting_value(&conn, AUTO_BACKUP_LAST_DATE_SETTING_KEY, Some(today_str.as_str()))?;
    Ok(result)
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

/// Change DB to a new location: copies current DB file there, then updates the setting.
/// Returns the new full DB file path.
pub fn move_database_to(app: &AppHandle, target_folder: &str) -> Result<String, String> {
    let target_folder = target_folder.trim();
    if target_folder.is_empty() {
        return Err("Target folder path cannot be empty.".to_string());
    }

    let current_path = super::resolve_database_path(app)?;

    let target_dir = PathBuf::from(target_folder);
    fs::create_dir_all(&target_dir)
        .map_err(|err| format!("failed to create target directory: {err}"))?;

    let target_path = target_dir.join(DB_FILE_NAME);

    // Checkpoint WAL fully BEFORE copying so the copied file is complete
    {
        let conn = super::open_encrypted_connection(&current_path)?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|err| format!("failed to checkpoint WAL before move: {err}"))?;
    }

    // Copy current DB to new location (keep old one as fallback until restart)
    fs::copy(&current_path, &target_path)
        .map_err(|err| format!("failed to copy database to new location: {err}"))?;

    set_db_custom_path(app, Some(target_folder))?;

    Ok(target_path.to_string_lossy().to_string())
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

    // Close connections by using raw copy (no WAL needed since VACUUM INTO was used for snapshot)
    let conn = open_runtime_connection(app)?;
    conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
        .map_err(|err| format!("failed to checkpoint WAL before restore: {err}"))?;
    drop(conn);

    fs::copy(&snapshot_path, &db_path)
        .map_err(|err| format!("failed to restore snapshot: {err}"))?;

    Ok(())
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

    let backup_directory_path = stored_backup_dir
        .unwrap_or_else(|| default_backup_dir.to_string_lossy().to_string());

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
        format!("{BACKUP_FILE_PREFIX}_auto_{}.sqlite3", now.format("%Y-%m-%d"))
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

    let escaped_path = backup_file_path
        .to_string_lossy()
        .replace('\'', "''");
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
    fs::write(&path, contents)
        .map_err(|err| format!("failed to write db_settings.json: {err}"))?;
    Ok(())
}


