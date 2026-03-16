// ──────────────────────────────────────────────────────────────────────────────
// db/mod.rs – entry point for the `db` module
//
// Declares sub-modules and owns shared infrastructure:
// * DB_READY static (one-time init guard)
// * Core connection helpers (open, configure, resolve path)
// * apply_migrations + all ensure_* migration steps
// * Shared utility functions used across domain modules
// * reset_all_data (cross-domain operation)
// * pub fn init_database / database_status (Tauri command targets)
// * Re-exports all public types from sub-modules
// ──────────────────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension};
use tauri::{AppHandle, Manager};

use schema::*;

// ── Sub-modules ───────────────────────────────────────────────────────────────
pub(crate) mod asset;
pub(crate) mod audit;
pub mod auth;
pub mod backup;
pub mod borrow;
pub mod column;
pub mod employee;
pub mod import;
mod schema;
pub mod team;

// ── Re-exports (all public types bubble up to `db::`) ─────────────────────────
pub use auth::*;
pub use backup::*;
pub use asset::*;
pub use borrow::*;
pub use column::*;
pub use employee::*;
pub use import::*;
pub use team::*;

// ── One-time init guard ───────────────────────────────────────────────────────
static DB_READY: OnceLock<Result<(), String>> = OnceLock::new();

// ── Public entry-point functions ──────────────────────────────────────────────

pub fn init_database(app: &AppHandle) -> Result<DatabaseStatus, String> {
    ensure_database_ready(app)?;
    let db_path = resolve_database_path(app)?;
    let conn = open_encrypted_connection(&db_path)?;

    let version = sqlite_version(&conn)?;

    Ok(DatabaseStatus {
        initialized: true,
        db_path: db_path.to_string_lossy().to_string(),
        sqlite_version: version,
    })
}

pub fn database_status(app: &AppHandle) -> Result<DatabaseStatus, String> {
    let db_path = resolve_database_path(app)?;
    let initialized = db_path.exists();

    if !initialized {
        return Ok(DatabaseStatus {
            initialized: false,
            db_path: db_path.to_string_lossy().to_string(),
            sqlite_version: String::new(),
        });
    }

    let conn = open_encrypted_connection(&db_path)?;
    let version = sqlite_version(&conn)?;

    Ok(DatabaseStatus {
        initialized: true,
        db_path: db_path.to_string_lossy().to_string(),
        sqlite_version: version,
    })
}

pub fn reset_all_data(app: &AppHandle) -> Result<bool, String> {
    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start reset transaction: {err}"))?;

    // Disable FTS triggers first. Contentless FTS table does not support plain
    // DELETE operations from trigger bodies.
    tx.execute_batch(
        r#"
        DROP TRIGGER IF EXISTS trg_employees_ai;
        DROP TRIGGER IF EXISTS trg_employees_au;
        DROP TRIGGER IF EXISTS trg_employees_ad;
        DROP TRIGGER IF EXISTS trg_teams_au;
        "#,
    )
    .map_err(|err| format!("failed to drop FTS triggers before reset: {err}"))?;

    tx.execute("DELETE FROM employee_dynamic_values", [])
        .map_err(humanize_sqlite_error)?;
    tx.execute("DELETE FROM employees", [])
        .map_err(humanize_sqlite_error)?;
    tx.execute("DELETE FROM teams", [])
        .map_err(humanize_sqlite_error)?;
    tx.execute("DELETE FROM employee_dynamic_fields", [])
        .map_err(humanize_sqlite_error)?;
    tx.execute("DELETE FROM sqlite_sequence WHERE name = 'employees'", [])
        .map_err(humanize_sqlite_error)?;

    // Rebuild empty FTS table + triggers for clean state after reset.
    tx.execute_batch("DROP TABLE IF EXISTS employees_fts;")
        .map_err(|err| format!("failed to drop FTS table during reset: {err}"))?;
    tx.execute_batch(FTS_TABLE_SQL)
        .map_err(|err| format!("failed to recreate FTS table during reset: {err}"))?;
    tx.execute_batch(FTS_TRIGGERS_SQL)
        .map_err(|err| format!("failed to recreate FTS triggers during reset: {err}"))?;

    tx.commit()
        .map_err(|err| format!("failed to commit reset transaction: {err}"))?;

    Ok(true)
}

// ── Core connection infrastructure (pub(crate) for use by sub-modules) ────────

pub(crate) fn open_runtime_connection(app: &AppHandle) -> Result<Connection, String> {
    ensure_database_ready(app)?;
    let db_path = resolve_database_path(app)?;
    open_encrypted_connection(&db_path)
}

/// Open a connection and apply the AES-256 SQLCipher key.
/// All reads/writes go through this — the DB file stays encrypted at rest.
pub(crate) fn open_encrypted_connection(path: &std::path::Path) -> Result<Connection, String> {
    let conn =
        Connection::open(path).map_err(|err| format!("failed to open sqlite database: {err}"))?;
    // Apply encryption key FIRST, before any other PRAGMA
    conn.execute_batch(&format!("PRAGMA key = '{APP_DB_ENCRYPTION_KEY}';"))
        .map_err(|err| format!("failed to apply database encryption key: {err}"))?;
    configure_connection(&conn)?;
    Ok(conn)
}

pub(crate) fn ensure_database_ready(app: &AppHandle) -> Result<(), String> {
    let result = DB_READY.get_or_init(|| {
        let db_path = resolve_database_path(app)?;

        // Migrate existing unencrypted DB to encrypted before first use
        migrate_to_encrypted(&db_path)?;

        let conn = open_encrypted_connection(&db_path)?;
        apply_migrations(&conn)?;
        Ok(())
    });

    match result {
        Ok(()) => Ok(()),
        Err(err) => Err(err.clone()),
    }
}

pub(crate) fn resolve_database_path(app: &AppHandle) -> Result<PathBuf, String> {
    // Check for custom path stored in db_settings.json (not the DB itself)
    if let Ok(Some(custom_folder)) = backup::get_db_custom_path(app) {
        let custom_dir = PathBuf::from(&custom_folder);
        if custom_dir.exists() || fs::create_dir_all(&custom_dir).is_ok() {
            return Ok(custom_dir.join(DB_FILE_NAME));
        }
    }

    // Default: app local data dir
    let mut data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|err| format!("failed to resolve app data directory: {err}"))?;

    fs::create_dir_all(&data_dir)
        .map_err(|err| format!("failed to create app data directory: {err}"))?;

    data_dir.push(DB_FILE_NAME);
    Ok(data_dir)
}

pub(crate) fn configure_connection(conn: &Connection) -> Result<(), String> {
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|err| format!("failed to set sqlite busy timeout: {err}"))?;

    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
        .map_err(|err| format!("failed to configure sqlite pragmas: {err}"))?;

    Ok(())
}

/// Detect and migrate an existing unencrypted SQLite database to SQLCipher.
///
/// Strategy:
/// 1. Try opening the DB with the encryption key → if it works, already encrypted.
/// 2. If key fails, the DB is plaintext → use `sqlcipher_export` to re-encrypt.
/// 3. Replace the original file with the encrypted copy.
/// 4. Record the migration in a sidecar settings file to avoid re-running.
fn migrate_to_encrypted(db_path: &std::path::Path) -> Result<(), String> {
    // If DB doesn't exist yet → nothing to migrate, new DBs are encrypted by default
    if !db_path.exists() {
        return Ok(());
    }

    // Check sidecar flag: if already migrated, skip
    let sidecar_path = db_path.with_extension("migration.json");
    if sidecar_path.exists() {
        if let Ok(content) = fs::read_to_string(&sidecar_path) {
            if content.contains(DB_ENCRYPTION_MIGRATION_SETTING_KEY) {
                return Ok(());
            }
        }
    }

    // Try to open WITH the encryption key → if it queries ok, already encrypted
    let is_already_encrypted = {
        match Connection::open(db_path) {
            Ok(test_conn) => {
                let key_result =
                    test_conn.execute_batch(&format!("PRAGMA key = '{APP_DB_ENCRYPTION_KEY}';"));
                if key_result.is_err() {
                    false
                } else {
                    // Try a simple query to verify key is correct
                    test_conn
                        .query_row("SELECT count(*) FROM sqlite_master", [], |r| {
                            r.get::<_, i64>(0)
                        })
                        .is_ok()
                }
            }
            Err(_) => false,
        }
    };

    if is_already_encrypted {
        // Mark migration as done
        let _ = fs::write(
            &sidecar_path,
            format!("{{\"{DB_ENCRYPTION_MIGRATION_SETTING_KEY}\": true}}"),
        );
        return Ok(());
    }

    // DB is plaintext — open without key and export to an encrypted copy
    let encrypted_path = db_path.with_extension("sqlite3.encrypting");

    let plain_conn = Connection::open(db_path)
        .map_err(|err| format!("failed to open plain database for migration: {err}"))?;

    plain_conn
        .execute_batch(&format!(
            "PRAGMA foreign_keys = OFF;\
             ATTACH DATABASE '{}' AS encrypted KEY '{APP_DB_ENCRYPTION_KEY}';\
             SELECT sqlcipher_export('encrypted');\
             DETACH DATABASE encrypted;",
            encrypted_path.to_string_lossy().replace('\\', "/")
        ))
        .map_err(|err| format!("failed to encrypt database during migration: {err}"))?;

    drop(plain_conn);

    // Replace original with encrypted copy
    fs::rename(&encrypted_path, db_path)
        .map_err(|err| format!("failed to replace database with encrypted version: {err}"))?;

    // Mark migration done
    let _ = fs::write(
        &sidecar_path,
        format!("{{\"{DB_ENCRYPTION_MIGRATION_SETTING_KEY}\": true}}"),
    );

    Ok(())
}

fn sqlite_version(conn: &Connection) -> Result<String, String> {
    conn.query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .map_err(|err| format!("failed to query sqlite version: {err}"))
}

pub(crate) fn apply_migrations(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(BASE_SCHEMA_SQL)
        .map_err(|err| format!("failed to initialize schema: {err}"))?;

    ensure_employee_columns(conn)?;
    ensure_team_columns(conn)?;
    ensure_local_account_columns(conn)?;
    auth::ensure_local_accounts_seed(conn)?;
    normalize_staff_group_values(conn)?;
    normalize_eml_security_tool_values(conn)?;
    ensure_search_index(conn)?;

    Ok(())
}

fn ensure_team_columns(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(teams)")
        .map_err(|err| format!("failed to inspect teams table: {err}"))?;

    let existing: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| format!("failed to read teams table columns: {err}"))?
        .filter_map(|r| r.ok())
        .collect();

    if !existing.iter().any(|name| name == "parent_id") {
        conn.execute_batch(
            "ALTER TABLE teams ADD COLUMN parent_id INTEGER REFERENCES teams(id) ON DELETE SET NULL;",
        )
        .map_err(|err| format!("failed to add teams.parent_id column: {err}"))?;
    }

    Ok(())
}

fn ensure_employee_columns(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(employees)")
        .map_err(|err| format!("failed to inspect employee table: {err}"))?;

    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| format!("failed to read employee table columns: {err}"))?;

    let mut existing = Vec::new();
    for row in rows {
        existing.push(row.map_err(|err| format!("failed to read column info: {err}"))?);
    }

    for (column_name, column_type) in EMPLOYEE_ADDITIONAL_COLUMNS {
        if existing.iter().any(|name| name == column_name) {
            continue;
        }

        conn.execute(
            &format!("ALTER TABLE employees ADD COLUMN {column_name} {column_type}"),
            [],
        )
        .map_err(|err| format!("failed to add column '{column_name}': {err}"))?;
    }

    Ok(())
}

fn ensure_local_account_columns(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(app_local_accounts)")
        .map_err(|err| format!("failed to inspect local account table: {err}"))?;

    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| format!("failed to read local account table columns: {err}"))?;

    let mut existing = Vec::new();
    for row in rows {
        existing
            .push(row.map_err(|err| format!("failed to read local account column info: {err}"))?);
    }

    let additional_columns = [
        ("username", "TEXT"),
        ("password_hash", "TEXT"),
        ("recovery_code_hash", "TEXT"),
        ("force_password_reset", "INTEGER NOT NULL DEFAULT 0"),
    ];

    for (column_name, column_type) in additional_columns {
        if existing.iter().any(|name| name == column_name) {
            continue;
        }

        conn.execute(
            &format!("ALTER TABLE app_local_accounts ADD COLUMN {column_name} {column_type}"),
            [],
        )
        .map_err(|err| format!("failed to add local-account column '{column_name}': {err}"))?;
    }

    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_local_accounts_username_unique ON app_local_accounts(username COLLATE NOCASE);",
    )
    .map_err(|err| format!("failed to ensure username unique index: {err}"))?;

    Ok(())
}

fn normalize_staff_group_values(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "UPDATE employees SET staff_group = ? WHERE staff_group = ?",
        params![STAFF_GROUP_INTERNAL_MOVEMENT, "internal_movent"],
    )
    .map_err(|err| format!("failed to normalize legacy staff_group values: {err}"))?;

    Ok(())
}

fn normalize_eml_security_tool_values(conn: &Connection) -> Result<(), String> {
    conn.execute(
        r#"
        UPDATE employee_dynamic_values
        SET value = 'Yes',
            updated_at = datetime('now')
        WHERE lower(trim(COALESCE(value, ''))) = 'v'
          AND replace(lower(field_key), '_', '') IN ('sentinelone', 'endpointagent')
          AND employee_id IN (
            SELECT e.id
            FROM employees e
            INNER JOIN teams t ON t.id = e.team_id
            WHERE lower(trim(COALESCE(t.name, ''))) = 'eml'
          )
        "#,
        [],
    )
    .map_err(|err| format!("failed to normalize EML security tool values: {err}"))?;

    Ok(())
}

fn ensure_search_index(conn: &Connection) -> Result<(), String> {
    let fts_sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'employees_fts'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to inspect FTS table: {err}"))?;

    let needs_rebuild = match fts_sql {
        Some(sql) => {
            let normalized = sql.to_lowercase();
            !FTS_COLUMNS
                .iter()
                .all(|column| normalized.contains(&column.to_lowercase()))
        }
        None => true,
    };

    conn.execute_batch(
        r#"
        DROP TRIGGER IF EXISTS trg_employees_ai;
        DROP TRIGGER IF EXISTS trg_employees_au;
        DROP TRIGGER IF EXISTS trg_employees_ad;
        DROP TRIGGER IF EXISTS trg_teams_au;
        "#,
    )
    .map_err(|err| format!("failed to refresh FTS triggers: {err}"))?;

    if needs_rebuild {
        conn.execute_batch("DROP TABLE IF EXISTS employees_fts;")
            .map_err(|err| format!("failed to drop outdated FTS table: {err}"))?;

        conn.execute_batch(FTS_TABLE_SQL)
            .map_err(|err| format!("failed to create FTS table: {err}"))?;

        conn.execute(
            r#"
            INSERT INTO employees_fts(
              rowid,
              employee_id,
              full_name,
              nick_name,
              email,
              project,
              job_title,
              notes,
              team_name,
              computername
            )
            SELECT
              e.id,
              e.employee_id,
              e.full_name,
              COALESCE(e.nick_name, ''),
              COALESCE(e.email, ''),
              COALESCE(e.project, ''),
              COALESCE(e.job_title, ''),
              COALESCE(e.notes, ''),
              COALESCE(t.name, ''),
              COALESCE(e.computername, '')
            FROM employees e
            LEFT JOIN teams t ON t.id = e.team_id
            "#,
            [],
        )
        .map_err(|err| format!("failed to backfill FTS table: {err}"))?;
    }

    conn.execute_batch(FTS_TRIGGERS_SQL)
        .map_err(|err| format!("failed to create FTS triggers: {err}"))?;

    Ok(())
}

// ── Shared utility functions (pub(crate) – used by domain modules) ────────────

pub(crate) fn get_setting_value(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT setting_value FROM app_settings WHERE setting_key = ?",
        params![key],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| format!("failed to read setting '{key}': {err}"))
}

pub(crate) fn set_setting_value(
    conn: &Connection,
    key: &str,
    value: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        r#"
        INSERT INTO app_settings(setting_key, setting_value, updated_at)
        VALUES(?, ?, datetime('now'))
        ON CONFLICT(setting_key) DO UPDATE SET
          setting_value = excluded.setting_value,
          updated_at = datetime('now')
        "#,
        params![key, value],
    )
    .map_err(humanize_sqlite_error)?;
    Ok(())
}

pub(crate) fn humanize_sqlite_error(err: rusqlite::Error) -> String {
    match err {
        rusqlite::Error::SqliteFailure(_, Some(message)) => {
            if message.contains("employees.employee_id") {
                return "employeeId already exists".to_string();
            }
            if message.contains("employees.email") {
                return "email already exists".to_string();
            }
            if message.contains("teams.name") {
                return "team name already exists".to_string();
            }
            message
        }
        other => other.to_string(),
    }
}

pub(crate) fn require_text(value: String, field_name: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_name} is required"));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "0" {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(crate) fn normalize_date_value(value: Option<String>) -> Option<String> {
    let normalized = normalize_optional_text(value)?;
    normalize_date_text(&normalized).or(Some(normalized))
}

pub(crate) fn normalize_date_text(value: &str) -> Option<String> {
    use chrono::{NaiveDate, NaiveDateTime};

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let date_patterns = [
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%d/%m/%Y",
        "%d-%m-%Y",
        "%m/%d/%Y",
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
    ];

    for pattern in date_patterns {
        if let Ok(date) = NaiveDate::parse_from_str(trimmed, pattern) {
            return Some(date.format("%Y-%m-%d").to_string());
        }
        if let Ok(date_time) = NaiveDateTime::parse_from_str(trimmed, pattern) {
            return Some(date_time.date().format("%Y-%m-%d").to_string());
        }
    }

    use chrono::NaiveDateTime as NDT;
    NDT::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|date_time| date_time.date().format("%Y-%m-%d").to_string())
}

pub(crate) fn normalize_dynamic_key(value: &str) -> String {
    let mut output = String::new();
    let mut last_is_separator = false;

    for ch in value.trim().chars() {
        if ch.is_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
            last_is_separator = false;
            continue;
        }

        if !output.is_empty() && !last_is_separator {
            output.push('_');
            last_is_separator = true;
        }
    }

    output.trim_matches('_').to_string()
}

pub(crate) fn normalize_dynamic_fields(
    input: Option<HashMap<String, String>>,
) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    let Some(items) = input else {
        return fields;
    };

    for (raw_key, raw_value) in items {
        let key = normalize_dynamic_key(&raw_key);
        if key.is_empty() {
            continue;
        }

        let Some(value) = normalize_optional_text(Some(raw_value)) else {
            continue;
        };

        fields.insert(key, value);
    }

    fields
}

pub(crate) fn dynamic_key_to_label(key: &str) -> String {
    key.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn is_reserved_column_key(key: &str) -> bool {
    matches!(key, "rownumber" | "row_number" | "actions" | "action")
        || CORE_COLUMN_DEFINITIONS
            .iter()
            .any(|(core_key, _)| normalize_dynamic_key(core_key) == key)
}

pub(crate) fn normalize_staff_group(value: &str) -> Option<&'static str> {
    let normalized = normalize_header_key(value);
    match normalized.as_str() {
        "employeelist" | "eelist" | "employees" | "eegroup" | "eelistgroup" => {
            Some(STAFF_GROUP_EMPLOYEE_LIST)
        }
        "onboarding" => Some(STAFF_GROUP_ONBOARDING),
        "offboarding" => Some(STAFF_GROUP_OFFBOARDING),
        "internalmovent" | "internalmovement" | "internalmove" => {
            Some(STAFF_GROUP_INTERNAL_MOVEMENT)
        }
        _ => None,
    }
}

fn normalize_header_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_exists(conn: &Connection, table_name: &str) -> bool {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)",
            params![table_name],
            |row| row.get::<_, i64>(0),
        )
        .map(|exists| exists > 0)
        .unwrap_or(false)
    }

    #[test]
    fn apply_migrations_creates_borrow_flow_tables() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        configure_connection(&conn).expect("configure sqlite pragmas");
        apply_migrations(&conn).expect("apply database migrations");

        for table_name in [
            "assets",
            "borrow_requests",
            "borrow_request_items",
            "asset_loans",
            "audit_logs",
        ] {
            assert!(
                table_exists(&conn, table_name),
                "expected table '{table_name}' to exist after migrations",
            );
        }
    }
}
