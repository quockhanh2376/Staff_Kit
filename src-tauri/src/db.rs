use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use calamine::{open_workbook_auto, Data, DataType, Reader};
use chrono::{Duration as ChronoDuration, NaiveDate, NaiveDateTime};
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const DB_FILE_NAME: &str = "staff_kit.sqlite3";

const BASE_SCHEMA_SQL: &str = r#"
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS teams (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS employees (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  employee_id TEXT NOT NULL UNIQUE,
  full_name TEXT NOT NULL,
  nick_name TEXT,
  team_id INTEGER REFERENCES teams(id) ON UPDATE CASCADE ON DELETE SET NULL,
  project TEXT,
  job_title TEXT,
  email TEXT UNIQUE,
  cellphone TEXT,
  date_of_birth TEXT,
  gender TEXT,
  asw_start_date TEXT,
  client_start_date TEXT,
  contract_end_date TEXT,
  client_year_of_services TEXT,
  start_date TEXT,
  computername TEXT,
  notes TEXT,
  staff_group TEXT NOT NULL DEFAULT 'employee_list',
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS employee_dynamic_fields (
  field_key TEXT PRIMARY KEY,
  field_label TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS employee_dynamic_values (
  employee_id INTEGER NOT NULL REFERENCES employees(id) ON DELETE CASCADE,
  field_key TEXT NOT NULL REFERENCES employee_dynamic_fields(field_key) ON DELETE CASCADE,
  value TEXT,
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY(employee_id, field_key)
);

CREATE TABLE IF NOT EXISTS app_local_accounts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  account_key TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  role TEXT NOT NULL DEFAULT 'user',
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS app_settings (
  setting_key TEXT PRIMARY KEY,
  setting_value TEXT,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_employees_team_id ON employees(team_id);
CREATE INDEX IF NOT EXISTS idx_employees_full_name ON employees(full_name);
CREATE INDEX IF NOT EXISTS idx_employees_asw_start_date ON employees(asw_start_date);
CREATE UNIQUE INDEX IF NOT EXISTS idx_employees_email_unique ON employees(email) WHERE email IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_dynamic_values_employee_id ON employee_dynamic_values(employee_id);
CREATE INDEX IF NOT EXISTS idx_dynamic_values_field_key ON employee_dynamic_values(field_key);
CREATE UNIQUE INDEX IF NOT EXISTS idx_local_accounts_display_name_unique
  ON app_local_accounts(display_name COLLATE NOCASE);
"#;

const EMPLOYEE_SELECT_COLUMNS: &str = r#"
  e.id,
  e.employee_id,
  e.full_name,
  e.nick_name,
  e.team_id,
  t.name AS team_name,
  e.project,
  e.job_title,
  e.email,
  e.cellphone,
  e.date_of_birth,
  e.gender,
  e.asw_start_date,
  e.client_start_date,
  e.contract_end_date,
  e.client_year_of_services,
  COALESCE(e.asw_start_date, e.start_date) AS start_date,
  e.computername,
  e.notes,
  CASE
    WHEN COALESCE(NULLIF(TRIM(e.staff_group), ''), 'employee_list') = 'internal_movent' THEN 'internal_movement'
    ELSE COALESCE(NULLIF(TRIM(e.staff_group), ''), 'employee_list')
  END AS staff_group,
  e.updated_at
"#;

const EMPLOYEE_ADDITIONAL_COLUMNS: &[(&str, &str)] = &[
    ("nick_name", "TEXT"),
    ("project", "TEXT"),
    ("job_title", "TEXT"),
    ("cellphone", "TEXT"),
    ("date_of_birth", "TEXT"),
    ("gender", "TEXT"),
    ("asw_start_date", "TEXT"),
    ("client_start_date", "TEXT"),
    ("contract_end_date", "TEXT"),
    ("client_year_of_services", "TEXT"),
    ("staff_group", "TEXT NOT NULL DEFAULT 'employee_list'"),
];

const STAFF_GROUP_EMPLOYEE_LIST: &str = "employee_list";
const STAFF_GROUP_ONBOARDING: &str = "onboarding";
const STAFF_GROUP_OFFBOARDING: &str = "offboarding";
const STAFF_GROUP_INTERNAL_MOVEMENT: &str = "internal_movement";
const LOCAL_ACCOUNT_ROLE_ADMIN: &str = "admin";
const LOCAL_ACCOUNT_ROLE_USER: &str = "user";
const DEFAULT_LOCAL_ACCOUNT_NAME: &str = "IT Admin";
const DEFAULT_LOCAL_ACCOUNT_KEY: &str = "it_admin";
const ACTIVE_LOCAL_ACCOUNT_SETTING_KEY: &str = "active_local_account_id";

const FTS_COLUMNS: &[&str] = &[
    "employee_id",
    "full_name",
    "nick_name",
    "email",
    "project",
    "job_title",
    "notes",
    "team_name",
];

const CORE_COLUMN_DEFINITIONS: &[(&str, &str)] = &[
    ("employeeId", "EE. ID"),
    ("fullName", "Vietnamese Name"),
    ("nickName", "Nick Name"),
    ("teamName", "Client (PMD)"),
    ("project", "Project"),
    ("jobTitle", "Current Job Title"),
    ("email", "Working Email"),
    ("cellphone", "Cellphone"),
    ("dateOfBirth", "D.O.B"),
    ("gender", "Gender"),
    ("aswStartDate", "ASW Start Date"),
    ("clientStartDate", "Client Start Date"),
    ("contractEndDate", "Contract End Date"),
    ("clientYearOfServices", "Client Year Of Services"),
    ("computerName", "Computer Name"),
    ("notes", "Notes"),
];

const FTS_TABLE_SQL: &str = r#"
CREATE VIRTUAL TABLE employees_fts USING fts5(
  employee_id,
  full_name,
  nick_name,
  email,
  project,
  job_title,
  notes,
  team_name,
  content=''
);
"#;

const FTS_TRIGGERS_SQL: &str = r#"
CREATE TRIGGER IF NOT EXISTS trg_employees_ai AFTER INSERT ON employees BEGIN
  INSERT INTO employees_fts(rowid, employee_id, full_name, nick_name, email, project, job_title, notes, team_name)
  VALUES (
    NEW.id,
    NEW.employee_id,
    NEW.full_name,
    COALESCE(NEW.nick_name, ''),
    COALESCE(NEW.email, ''),
    COALESCE(NEW.project, ''),
    COALESCE(NEW.job_title, ''),
    COALESCE(NEW.notes, ''),
    COALESCE((SELECT name FROM teams WHERE id = NEW.team_id), '')
  );
END;

CREATE TRIGGER IF NOT EXISTS trg_employees_au AFTER UPDATE ON employees BEGIN
  DELETE FROM employees_fts WHERE rowid = OLD.id;
  INSERT INTO employees_fts(rowid, employee_id, full_name, nick_name, email, project, job_title, notes, team_name)
  VALUES (
    NEW.id,
    NEW.employee_id,
    NEW.full_name,
    COALESCE(NEW.nick_name, ''),
    COALESCE(NEW.email, ''),
    COALESCE(NEW.project, ''),
    COALESCE(NEW.job_title, ''),
    COALESCE(NEW.notes, ''),
    COALESCE((SELECT name FROM teams WHERE id = NEW.team_id), '')
  );
END;

CREATE TRIGGER IF NOT EXISTS trg_employees_ad AFTER DELETE ON employees BEGIN
  DELETE FROM employees_fts WHERE rowid = OLD.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_teams_au AFTER UPDATE ON teams BEGIN
  DELETE FROM employees_fts
  WHERE rowid IN (SELECT id FROM employees WHERE team_id = NEW.id);

  INSERT INTO employees_fts(rowid, employee_id, full_name, nick_name, email, project, job_title, notes, team_name)
  SELECT
    e.id,
    e.employee_id,
    e.full_name,
    COALESCE(e.nick_name, ''),
    COALESCE(e.email, ''),
    COALESCE(e.project, ''),
    COALESCE(e.job_title, ''),
    COALESCE(e.notes, ''),
    COALESCE(NEW.name, '')
  FROM employees e
  WHERE e.team_id = NEW.id;
END;
"#;

static DB_READY: OnceLock<Result<(), String>> = OnceLock::new();

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatus {
    pub initialized: bool,
    pub db_path: String,
    pub sqlite_version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeRecord {
    pub id: i64,
    pub employee_id: String,
    pub full_name: String,
    pub nick_name: Option<String>,
    pub team_id: Option<i64>,
    pub team_name: Option<String>,
    pub project: Option<String>,
    pub job_title: Option<String>,
    pub email: Option<String>,
    pub cellphone: Option<String>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub asw_start_date: Option<String>,
    pub client_start_date: Option<String>,
    pub contract_end_date: Option<String>,
    pub client_year_of_services: Option<String>,
    pub start_date: Option<String>,
    pub computer_name: Option<String>,
    pub notes: Option<String>,
    pub staff_group: String,
    pub dynamic_fields: HashMap<String, String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeColumnDefinition {
    pub key: String,
    pub label: String,
    pub source: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeListResponse {
    pub items: Vec<EmployeeRecord>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeQuery {
    pub query: Option<String>,
    pub team_name: Option<String>,
    pub staff_group: Option<String>,
    pub start_date_from: Option<String>,
    pub start_date_to: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeGroupCounts {
    pub employee_list: i64,
    pub onboarding: i64,
    pub offboarding: i64,
    pub internal_movement: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAccountRecord {
    pub id: i64,
    pub account_key: String,
    pub display_name: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAccountCreateInput {
    pub display_name: String,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAccountUpdateInput {
    pub id: i64,
    pub display_name: String,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmployeePayload {
    pub employee_id: String,
    pub full_name: String,
    pub nick_name: Option<String>,
    pub team_name: Option<String>,
    pub project: Option<String>,
    pub job_title: Option<String>,
    pub email: Option<String>,
    pub cellphone: Option<String>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub asw_start_date: Option<String>,
    pub client_start_date: Option<String>,
    pub contract_end_date: Option<String>,
    pub client_year_of_services: Option<String>,
    pub computer_name: Option<String>,
    pub notes: Option<String>,
    pub dynamic_fields: Option<HashMap<String, String>>,
}
#[derive(Debug)]
struct NormalizedEmployeePayload {
    employee_id: String,
    full_name: String,
    nick_name: Option<String>,
    team_name: Option<String>,
    project: Option<String>,
    job_title: Option<String>,
    email: Option<String>,
    cellphone: Option<String>,
    date_of_birth: Option<String>,
    gender: Option<String>,
    asw_start_date: Option<String>,
    client_start_date: Option<String>,
    contract_end_date: Option<String>,
    client_year_of_services: Option<String>,
    start_date: Option<String>,
    computer_name: Option<String>,
    notes: Option<String>,
    dynamic_fields: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamRecord {
    pub id: i64,
    pub name: String,
    pub member_count: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamUpsertInput {
    pub id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeColumnUpsertInput {
    pub key: Option<String>,
    pub label: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportExcelInput {
    pub file_path: Option<String>,
    pub file_paths: Option<Vec<String>>,
    pub selected_column_keys: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportErrorItem {
    pub row: u32,
    pub employee_id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub source_file: String,
    pub source_files: Vec<String>,
    pub sheet_name: String,
    pub header_row: u32,
    pub processed_sheets: Vec<String>,
    pub total_rows: u32,
    pub inserted: u32,
    pub updated: u32,
    pub skipped: u32,
    pub failed: u32,
    pub errors: Vec<ImportErrorItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportColumnOption {
    pub key: String,
    pub label: String,
    pub source: String,
    pub required: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportColumnsPreview {
    pub source_files: Vec<String>,
    pub detected_columns: Vec<ImportColumnOption>,
}

#[derive(Debug)]
struct ImportColumns {
    asw_start_date: Option<usize>,
    employee_id: usize,
    client_start_date: Option<usize>,
    full_name: Option<usize>,
    nick_name: Option<usize>,
    client_pmd: Option<usize>,
    project: Option<usize>,
    job_title: Option<usize>,
    email: Option<usize>,
    cellphone: Option<usize>,
    date_of_birth: Option<usize>,
    gender: Option<usize>,
    contract_end_date: Option<usize>,
    client_year_of_services: Option<usize>,
    computer_name: Option<usize>,
    notes: Option<usize>,
    dynamic_columns: Vec<DynamicImportColumn>,
}

#[derive(Debug, Clone)]
struct DynamicImportColumn {
    index: usize,
    field_key: String,
    field_label: String,
}

enum UpsertAction {
    Inserted,
    Updated,
}

impl TryFrom<EmployeePayload> for NormalizedEmployeePayload {
    type Error = String;

    fn try_from(value: EmployeePayload) -> Result<Self, Self::Error> {
        let employee_id = normalize_employee_id(value.employee_id)?;
        let full_name = require_text(value.full_name, "fullName")?;
        let asw_start_date = normalize_date_value(value.asw_start_date);

        Ok(Self {
            employee_id,
            full_name,
            nick_name: normalize_optional_text(value.nick_name),
            team_name: normalize_optional_text(value.team_name),
            project: normalize_optional_text(value.project),
            job_title: normalize_optional_text(value.job_title),
            email: normalize_email(value.email),
            cellphone: normalize_optional_text(value.cellphone),
            date_of_birth: normalize_date_value(value.date_of_birth),
            gender: normalize_optional_text(value.gender),
            asw_start_date: asw_start_date.clone(),
            client_start_date: normalize_date_value(value.client_start_date),
            contract_end_date: normalize_optional_or_date(value.contract_end_date),
            client_year_of_services: normalize_optional_text(value.client_year_of_services),
            start_date: asw_start_date,
            computer_name: normalize_optional_text(value.computer_name),
            notes: normalize_optional_text(value.notes),
            dynamic_fields: normalize_dynamic_fields(value.dynamic_fields),
        })
    }
}

pub fn init_database(app: &AppHandle) -> Result<DatabaseStatus, String> {
    ensure_database_ready(app)?;
    let db_path = resolve_database_path(app)?;
    let conn = Connection::open(&db_path)
        .map_err(|err| format!("failed to open sqlite database: {err}"))?;
    configure_connection(&conn)?;

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

    let conn = Connection::open(&db_path)
        .map_err(|err| format!("failed to open sqlite database: {err}"))?;
    configure_connection(&conn)?;
    let version = sqlite_version(&conn)?;

    Ok(DatabaseStatus {
        initialized: true,
        db_path: db_path.to_string_lossy().to_string(),
        sqlite_version: version,
    })
}

pub fn list_employees(
    app: &AppHandle,
    filters: EmployeeQuery,
) -> Result<EmployeeListResponse, String> {
    let conn = open_runtime_connection(app)?;
    query_employees(&conn, filters)
}

pub fn search_employees(
    app: &AppHandle,
    filters: EmployeeQuery,
) -> Result<EmployeeListResponse, String> {
    let conn = open_runtime_connection(app)?;
    query_employees(&conn, filters)
}

pub fn list_employee_group_counts(app: &AppHandle) -> Result<EmployeeGroupCounts, String> {
    let conn = open_runtime_connection(app)?;
    conn.query_row(
        r#"
        SELECT
          COALESCE(SUM(CASE
            WHEN COALESCE(NULLIF(TRIM(staff_group), ''), 'employee_list') = 'employee_list' THEN 1
            ELSE 0
          END), 0),
          COALESCE(SUM(CASE
            WHEN COALESCE(NULLIF(TRIM(staff_group), ''), 'employee_list') = 'onboarding' THEN 1
            ELSE 0
          END), 0),
          COALESCE(SUM(CASE
            WHEN COALESCE(NULLIF(TRIM(staff_group), ''), 'employee_list') = 'offboarding' THEN 1
            ELSE 0
          END), 0),
          COALESCE(SUM(CASE
            WHEN COALESCE(NULLIF(TRIM(staff_group), ''), 'employee_list') IN ('internal_movement', 'internal_movent') THEN 1
            ELSE 0
          END), 0)
        FROM employees
        "#,
        [],
        |row| {
            Ok(EmployeeGroupCounts {
                employee_list: row.get(0)?,
                onboarding: row.get(1)?,
                offboarding: row.get(2)?,
                internal_movement: row.get(3)?,
            })
        },
    )
    .map_err(|err| format!("failed to query employee group counts: {err}"))
}

pub fn list_local_accounts(app: &AppHandle) -> Result<Vec<LocalAccountRecord>, String> {
    let conn = open_runtime_connection(app)?;
    query_local_accounts(&conn)
}

pub fn create_local_account(
    app: &AppHandle,
    payload: LocalAccountCreateInput,
) -> Result<LocalAccountRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start local-account transaction: {err}"))?;

    let display_name = require_text(payload.display_name, "displayName")?;
    let role = normalize_local_account_role(payload.role);
    let account_key = generate_local_account_key(&tx, display_name.as_str())?;

    tx.execute(
        r#"
        INSERT INTO app_local_accounts(account_key, display_name, role, created_at, updated_at)
        VALUES(?, ?, ?, datetime('now'), datetime('now'))
        "#,
        params![account_key.as_str(), display_name.as_str(), role.as_str()],
    )
    .map_err(humanize_sqlite_error)?;

    let id = tx.last_insert_rowid();
    let active_id = get_active_local_account_id_tx(&tx)?;
    if active_id.is_none() {
        set_active_local_account_id_tx(&tx, id)?;
    }

    tx.commit()
        .map_err(|err| format!("failed to commit local-account transaction: {err}"))?;

    let conn = open_runtime_connection(app)?;
    load_local_account_by_id(&conn, id)
}

pub fn update_local_account(
    app: &AppHandle,
    payload: LocalAccountUpdateInput,
) -> Result<LocalAccountRecord, String> {
    let conn = open_runtime_connection(app)?;
    let display_name = require_text(payload.display_name, "displayName")?;
    let role = normalize_local_account_role(payload.role);

    let changed = conn
        .execute(
            r#"
            UPDATE app_local_accounts
            SET display_name = ?, role = ?, updated_at = datetime('now')
            WHERE id = ?
            "#,
            params![display_name.as_str(), role.as_str(), payload.id],
        )
        .map_err(humanize_sqlite_error)?;

    if changed == 0 {
        return Err(format!("local account with id {} was not found", payload.id));
    }

    load_local_account_by_id(&conn, payload.id)
}

pub fn delete_local_account(app: &AppHandle, id: i64) -> Result<bool, String> {
    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start local-account transaction: {err}"))?;

    let total_accounts: i64 = tx
        .query_row("SELECT COUNT(*) FROM app_local_accounts", [], |row| row.get(0))
        .map_err(|err| format!("failed to count local accounts: {err}"))?;
    if total_accounts <= 1 {
        return Err("at least one local account is required".to_string());
    }

    let active_id = get_active_local_account_id_tx(&tx)?;
    let changed = tx
        .execute("DELETE FROM app_local_accounts WHERE id = ?", params![id])
        .map_err(humanize_sqlite_error)?;
    if changed == 0 {
        return Err(format!("local account with id {id} was not found"));
    }

    if active_id == Some(id) {
        let next_active: i64 = tx
            .query_row(
                "SELECT id FROM app_local_accounts ORDER BY created_at ASC, id ASC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|err| format!("failed to resolve fallback local account: {err}"))?;
        set_active_local_account_id_tx(&tx, next_active)?;
    }

    tx.commit()
        .map_err(|err| format!("failed to commit local-account transaction: {err}"))?;
    Ok(true)
}

pub fn set_active_local_account(app: &AppHandle, id: i64) -> Result<bool, String> {
    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start local-account transaction: {err}"))?;

    let exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM app_local_accounts WHERE id = ?",
            params![id],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to verify local account: {err}"))?;
    if exists <= 0 {
        return Err(format!("local account with id {id} was not found"));
    }

    set_active_local_account_id_tx(&tx, id)?;
    tx.commit()
        .map_err(|err| format!("failed to commit local-account transaction: {err}"))?;

    Ok(true)
}

pub fn list_employee_columns(app: &AppHandle) -> Result<Vec<EmployeeColumnDefinition>, String> {
    let conn = open_runtime_connection(app)?;
    let mut columns = CORE_COLUMN_DEFINITIONS
        .iter()
        .map(|(key, label)| EmployeeColumnDefinition {
            key: (*key).to_string(),
            label: (*label).to_string(),
            source: "core".to_string(),
        })
        .collect::<Vec<_>>();

    let mut stmt = conn
        .prepare(
            "SELECT field_key, field_label FROM employee_dynamic_fields ORDER BY field_label COLLATE NOCASE ASC",
        )
        .map_err(|err| format!("failed to prepare employee dynamic columns query: {err}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(EmployeeColumnDefinition {
                key: row.get::<_, String>(0)?,
                label: row.get::<_, String>(1)?,
                source: "dynamic".to_string(),
            })
        })
        .map_err(|err| format!("failed to query employee dynamic columns: {err}"))?;

    for row in rows {
        columns
            .push(row.map_err(|err| format!("failed to read employee dynamic column row: {err}"))?);
    }

    Ok(columns)
}

pub fn upsert_employee_column(
    app: &AppHandle,
    payload: EmployeeColumnUpsertInput,
) -> Result<EmployeeColumnDefinition, String> {
    let conn = open_runtime_connection(app)?;
    let label = require_text(payload.label, "label")?;

    let key = if let Some(raw_key) = payload.key {
        let normalized_key = normalize_dynamic_key(raw_key.as_str());
        if normalized_key.is_empty() {
            return Err("column key is invalid".to_string());
        }

        if is_reserved_column_key(normalized_key.as_str()) {
            return Err("cannot override a reserved core/system column".to_string());
        }

        normalized_key
    } else {
        generate_dynamic_field_key(&conn, label.as_str())?
    };

    conn.execute(
        r#"
        INSERT INTO employee_dynamic_fields(field_key, field_label, updated_at)
        VALUES(?, ?, datetime('now'))
        ON CONFLICT(field_key) DO UPDATE SET
          field_label = excluded.field_label,
          updated_at = datetime('now')
        "#,
        params![key.as_str(), label.as_str()],
    )
    .map_err(humanize_sqlite_error)?;

    Ok(EmployeeColumnDefinition {
        key,
        label,
        source: "dynamic".to_string(),
    })
}

pub fn delete_employee_column(app: &AppHandle, key: String) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    let normalized = normalize_dynamic_key(key.as_str());
    if normalized.is_empty() {
        return Err("column key is invalid".to_string());
    }

    if is_reserved_column_key(normalized.as_str()) {
        return Err("cannot delete a reserved core/system column".to_string());
    }

    let changed = conn
        .execute(
            "DELETE FROM employee_dynamic_fields WHERE field_key = ?",
            params![normalized.as_str()],
        )
        .map_err(humanize_sqlite_error)?;

    Ok(changed > 0)
}

pub fn create_employee(
    app: &AppHandle,
    payload: EmployeePayload,
) -> Result<EmployeeRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start transaction: {err}"))?;

    let normalized = NormalizedEmployeePayload::try_from(payload)?;
    let team_id = resolve_team_id_tx(&tx, normalized.team_name.as_deref())?;

    tx.execute(
        r#"
        INSERT INTO employees (
          employee_id,
          full_name,
          nick_name,
          team_id,
          project,
          job_title,
          email,
          cellphone,
          date_of_birth,
          gender,
          asw_start_date,
          client_start_date,
          contract_end_date,
          client_year_of_services,
          start_date,
          computername,
          notes,
          updated_at
        ) VALUES (
          ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now')
        )
        "#,
        params![
            normalized.employee_id.as_str(),
            normalized.full_name.as_str(),
            normalized.nick_name.as_deref(),
            team_id,
            normalized.project.as_deref(),
            normalized.job_title.as_deref(),
            normalized.email.as_deref(),
            normalized.cellphone.as_deref(),
            normalized.date_of_birth.as_deref(),
            normalized.gender.as_deref(),
            normalized.asw_start_date.as_deref(),
            normalized.client_start_date.as_deref(),
            normalized.contract_end_date.as_deref(),
            normalized.client_year_of_services.as_deref(),
            normalized.start_date.as_deref(),
            normalized.computer_name.as_deref(),
            normalized.notes.as_deref(),
        ],
    )
    .map_err(humanize_sqlite_error)?;

    let id = tx.last_insert_rowid();
    if !normalized.dynamic_fields.is_empty() {
        upsert_dynamic_field_definitions_for_map(&tx, &normalized.dynamic_fields)?;
        upsert_dynamic_fields_tx(&tx, id, &normalized.dynamic_fields)?;
    }
    tx.commit()
        .map_err(|err| format!("failed to commit transaction: {err}"))?;

    let conn = open_runtime_connection(app)?;
    load_employee_by_id(&conn, id)
}

pub fn update_employee(
    app: &AppHandle,
    id: i64,
    payload: EmployeePayload,
) -> Result<EmployeeRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start transaction: {err}"))?;

    let normalized = NormalizedEmployeePayload::try_from(payload)?;
    let team_id = resolve_team_id_tx(&tx, normalized.team_name.as_deref())?;

    let changed = tx
        .execute(
            r#"
            UPDATE employees
            SET
              employee_id = ?,
              full_name = ?,
              nick_name = ?,
              team_id = ?,
              project = ?,
              job_title = ?,
              email = ?,
              cellphone = ?,
              date_of_birth = ?,
              gender = ?,
              asw_start_date = ?,
              client_start_date = ?,
              contract_end_date = ?,
              client_year_of_services = ?,
              start_date = ?,
              computername = ?,
              notes = ?,
              updated_at = datetime('now')
            WHERE id = ?
            "#,
            params![
                normalized.employee_id.as_str(),
                normalized.full_name.as_str(),
                normalized.nick_name.as_deref(),
                team_id,
                normalized.project.as_deref(),
                normalized.job_title.as_deref(),
                normalized.email.as_deref(),
                normalized.cellphone.as_deref(),
                normalized.date_of_birth.as_deref(),
                normalized.gender.as_deref(),
                normalized.asw_start_date.as_deref(),
                normalized.client_start_date.as_deref(),
                normalized.contract_end_date.as_deref(),
                normalized.client_year_of_services.as_deref(),
                normalized.start_date.as_deref(),
                normalized.computer_name.as_deref(),
                normalized.notes.as_deref(),
                id,
            ],
        )
        .map_err(humanize_sqlite_error)?;

    if changed == 0 {
        return Err(format!("employee with id {id} was not found"));
    }

    if !normalized.dynamic_fields.is_empty() {
        upsert_dynamic_field_definitions_for_map(&tx, &normalized.dynamic_fields)?;
        upsert_dynamic_fields_tx(&tx, id, &normalized.dynamic_fields)?;
    }

    tx.commit()
        .map_err(|err| format!("failed to commit transaction: {err}"))?;

    let conn = open_runtime_connection(app)?;
    load_employee_by_id(&conn, id)
}

pub fn delete_employee(app: &AppHandle, id: i64) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    let changed = conn
        .execute("DELETE FROM employees WHERE id = ?", params![id])
        .map_err(humanize_sqlite_error)?;
    Ok(changed > 0)
}
pub fn list_teams(app: &AppHandle) -> Result<Vec<TeamRecord>, String> {
    let conn = open_runtime_connection(app)?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              t.id,
              t.name,
              COUNT(e.id) AS member_count
            FROM teams t
            LEFT JOIN employees e ON e.team_id = t.id
            GROUP BY t.id, t.name
            ORDER BY t.name COLLATE NOCASE ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare teams query: {err}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(TeamRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                member_count: row.get(2)?,
            })
        })
        .map_err(|err| format!("failed to query teams: {err}"))?;

    let mut teams = Vec::new();
    for row in rows {
        teams.push(row.map_err(|err| format!("failed to read team row: {err}"))?);
    }

    Ok(teams)
}

pub fn upsert_team(app: &AppHandle, payload: TeamUpsertInput) -> Result<TeamRecord, String> {
    let conn = open_runtime_connection(app)?;
    let normalized_name = require_text(payload.name, "name")?;

    let team_id = if let Some(id) = payload.id {
        let changed = conn
            .execute(
                "UPDATE teams SET name = ? WHERE id = ?",
                params![normalized_name.as_str(), id],
            )
            .map_err(humanize_sqlite_error)?;

        if changed == 0 {
            return Err(format!("team with id {id} was not found"));
        }
        id
    } else {
        conn.execute(
            "INSERT INTO teams(name) VALUES (?) ON CONFLICT(name) DO UPDATE SET name = excluded.name",
            params![normalized_name.as_str()],
        )
        .map_err(humanize_sqlite_error)?;

        conn.query_row(
            "SELECT id FROM teams WHERE name = ?",
            params![normalized_name.as_str()],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to resolve team id after upsert: {err}"))?
    };

    load_team_by_id(&conn, team_id)
}

pub fn delete_team(app: &AppHandle, id: i64) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    let in_use: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM employees WHERE team_id = ?",
            params![id],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to check team references: {err}"))?;

    if in_use > 0 {
        return Err(format!(
            "team is currently assigned to {in_use} employee(s) and cannot be deleted"
        ));
    }

    let changed = conn
        .execute("DELETE FROM teams WHERE id = ?", params![id])
        .map_err(humanize_sqlite_error)?;

    Ok(changed > 0)
}

pub fn reset_all_data(app: &AppHandle) -> Result<bool, String> {
    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start reset transaction: {err}"))?;

    // Disable FTS triggers first. Contentless FTS table does not support plain DELETE
    // operations from trigger bodies.
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

pub fn inspect_import_columns(
    app: &AppHandle,
    payload: ImportExcelInput,
) -> Result<ImportColumnsPreview, String> {
    ensure_database_ready(app)?;
    let source_paths = resolve_import_source_paths(payload.file_path, payload.file_paths)?;

    let mut detected_map: HashMap<String, ImportColumnOption> = HashMap::new();
    let mut source_files = Vec::new();

    for source_path in source_paths {
        source_files.push(source_path.to_string_lossy().to_string());

        let mut workbook = match open_workbook_auto(&source_path) {
            Ok(workbook) => workbook,
            Err(_) => continue,
        };

        let sheet_names = workbook.sheet_names().to_vec();
        for sheet_name in sheet_names {
            let range = match workbook.worksheet_range(&sheet_name) {
                Ok(range) => range,
                Err(_) => continue,
            };

            let (_, columns) = match detect_import_columns(&range) {
                Ok(value) => value,
                Err(_) => continue,
            };

            for option in collect_import_column_options(&columns) {
                detected_map.entry(option.key.clone()).or_insert(option);
            }
        }
    }

    if !detected_map.contains_key("employeeId") {
        return Err("failed to detect a valid staff-id column in selected files".to_string());
    }

    let mut detected_columns = detected_map.into_values().collect::<Vec<_>>();
    detected_columns.sort_by(|a, b| {
        b.required
            .cmp(&a.required)
            .then_with(|| a.source.cmp(&b.source))
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });

    Ok(ImportColumnsPreview {
        source_files,
        detected_columns,
    })
}

pub fn import_excel(app: &AppHandle, payload: ImportExcelInput) -> Result<ImportReport, String> {
    ensure_database_ready(app)?;
    let source_paths = resolve_import_source_paths(payload.file_path, payload.file_paths)?;
    let selected_column_keys = normalize_selected_column_keys(payload.selected_column_keys);

    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start import transaction: {err}"))?;

    let mut report = ImportReport {
        source_file: source_paths
            .first()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        source_files: source_paths
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>(),
        sheet_name: String::new(),
        header_row: 0,
        processed_sheets: Vec::new(),
        total_rows: 0,
        inserted: 0,
        updated: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
    };

    for source_path in source_paths {
        let source_name = source_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("workbook")
            .to_string();

        let mut workbook = open_workbook_auto(&source_path)
            .map_err(|err| format!("failed to open workbook '{}': {err}", source_path.display()))?;

        let sheet_names = workbook.sheet_names().to_vec();
        for sheet_name in sheet_names {
            let sheet_staff_group = infer_staff_group_from_source(
                source_name.as_str(),
                sheet_name.as_str(),
            );

            let range = match workbook.worksheet_range(&sheet_name) {
                Ok(range) => range,
                Err(_) => continue,
            };

            let (header_row_index, columns) = match detect_import_columns(&range) {
                Ok(value) => value,
                Err(_) => continue,
            };

            if report.sheet_name.is_empty() {
                report.sheet_name = format!("{source_name}::{sheet_name}");
                report.header_row = (header_row_index + 1) as u32;
            }
            report
                .processed_sheets
                .push(format!("{source_name}::{sheet_name}"));

            let selected_dynamic_columns = columns
                .dynamic_columns
                .iter()
                .filter(|column| column_selected(&selected_column_keys, column.field_key.as_str()))
                .cloned()
                .collect::<Vec<_>>();

            if !selected_dynamic_columns.is_empty() {
                upsert_dynamic_field_definitions_tx(&tx, &selected_dynamic_columns)?;
            }

            for (row_index, row) in range.rows().enumerate().skip(header_row_index + 1) {
                if row_is_empty(row) {
                    continue;
                }

                report.total_rows += 1;
                let row_number = (row_index + 1) as u32;
                let sheet_ref = format!("{source_name}::{sheet_name}");

                let raw_employee_id = extract_optional_value(row, Some(columns.employee_id));
                let employee_id = match raw_employee_id {
                    Some(value) => match normalize_employee_id(value) {
                        Ok(parsed) => parsed,
                        Err(err) => {
                            report.skipped += 1;
                            report.errors.push(ImportErrorItem {
                                row: row_number,
                                employee_id: None,
                                reason: format!("[{sheet_ref}] {err}"),
                            });
                            continue;
                        }
                    },
                    None => {
                        report.skipped += 1;
                        continue;
                    }
                };

                let existing = load_employee_by_employee_id(&tx, employee_id.as_str())?;
                let existing_ref = existing.as_ref();

                let full_name_index = if column_selected(&selected_column_keys, "fullName") {
                    columns.full_name
                } else {
                    None
                };

                let full_name = extract_optional_value(row, full_name_index)
                    .or_else(|| existing_ref.map(|item| item.full_name.clone()));

                let Some(full_name) = full_name else {
                    report.failed += 1;
                    report.errors.push(ImportErrorItem {
                        row: row_number,
                        employee_id: Some(employee_id),
                        reason: format!("[{sheet_ref}] missing full name for new employee"),
                    });
                    continue;
                };

                let mut dynamic_fields = existing_ref
                    .map(|employee| employee.dynamic_fields.clone())
                    .unwrap_or_default();

                for column in &selected_dynamic_columns {
                    if let Some(value) = extract_optional_value(row, Some(column.index)) {
                        dynamic_fields.insert(column.field_key.clone(), value);
                    }
                }

                let payload = EmployeePayload {
                    employee_id: employee_id.clone(),
                    full_name,
                    nick_name: merge_import_text(
                        existing_ref.and_then(|item| item.nick_name.clone()),
                        row,
                        selected_column_index(&selected_column_keys, "nickName", columns.nick_name),
                    ),
                    team_name: merge_import_team_name(
                        existing_ref.and_then(|item| item.team_name.clone()),
                        row,
                        if column_selected(&selected_column_keys, "teamName") {
                            columns.client_pmd
                        } else {
                            None
                        },
                        if column_selected(&selected_column_keys, "teamName") {
                            columns.project
                        } else {
                            None
                        },
                    ),
                    project: merge_import_text(
                        existing_ref.and_then(|item| item.project.clone()),
                        row,
                        selected_column_index(&selected_column_keys, "project", columns.project),
                    ),
                    job_title: merge_import_text(
                        existing_ref.and_then(|item| item.job_title.clone()),
                        row,
                        selected_column_index(&selected_column_keys, "jobTitle", columns.job_title),
                    ),
                    email: merge_import_text(
                        existing_ref.and_then(|item| item.email.clone()),
                        row,
                        selected_column_index(&selected_column_keys, "email", columns.email),
                    ),
                    cellphone: merge_import_text(
                        existing_ref.and_then(|item| item.cellphone.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "cellphone",
                            columns.cellphone,
                        ),
                    ),
                    date_of_birth: merge_import_date(
                        existing_ref.and_then(|item| item.date_of_birth.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "dateOfBirth",
                            columns.date_of_birth,
                        ),
                    ),
                    gender: merge_import_text(
                        existing_ref.and_then(|item| item.gender.clone()),
                        row,
                        selected_column_index(&selected_column_keys, "gender", columns.gender),
                    ),
                    asw_start_date: merge_import_date(
                        existing_ref.and_then(|item| item.asw_start_date.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "aswStartDate",
                            columns.asw_start_date,
                        ),
                    ),
                    client_start_date: merge_import_date(
                        existing_ref.and_then(|item| item.client_start_date.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "clientStartDate",
                            columns.client_start_date,
                        ),
                    ),
                    contract_end_date: merge_import_optional_or_date(
                        existing_ref.and_then(|item| item.contract_end_date.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "contractEndDate",
                            columns.contract_end_date,
                        ),
                    ),
                    client_year_of_services: merge_import_text(
                        existing_ref.and_then(|item| item.client_year_of_services.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "clientYearOfServices",
                            columns.client_year_of_services,
                        ),
                    ),
                    computer_name: merge_import_text(
                        existing_ref.and_then(|item| item.computer_name.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "computerName",
                            columns.computer_name,
                        ),
                    ),
                    notes: merge_import_text(
                        existing_ref.and_then(|item| item.notes.clone()),
                        row,
                        selected_column_index(&selected_column_keys, "notes", columns.notes),
                    ),
                    dynamic_fields: if dynamic_fields.is_empty() {
                        None
                    } else {
                        Some(dynamic_fields)
                    },
                };

                match upsert_employee_from_payload(&tx, payload, sheet_staff_group) {
                    Ok(UpsertAction::Inserted) => report.inserted += 1,
                    Ok(UpsertAction::Updated) => report.updated += 1,
                    Err(err) => {
                        report.failed += 1;
                        report.errors.push(ImportErrorItem {
                            row: row_number,
                            employee_id: Some(employee_id),
                            reason: format!("[{sheet_ref}] {err}"),
                        });
                    }
                }
            }
        }
    }

    if report.processed_sheets.is_empty() {
        return Err("failed to detect a valid staff-id column in workbook sheets".to_string());
    }

    tx.commit()
        .map_err(|err| format!("failed to commit import transaction: {err}"))?;

    Ok(report)
}

fn normalize_selected_column_keys(input: Option<Vec<String>>) -> HashSet<String> {
    let mut keys = HashSet::new();
    let Some(items) = input else {
        return keys;
    };

    for item in items {
        let trimmed = item.trim();
        if !trimmed.is_empty() {
            keys.insert(trimmed.to_string());
        }
    }

    keys
}

fn column_selected(selected_keys: &HashSet<String>, key: &str) -> bool {
    selected_keys.is_empty() || selected_keys.contains(key)
}

fn selected_column_index(
    selected_keys: &HashSet<String>,
    key: &str,
    index: Option<usize>,
) -> Option<usize> {
    if column_selected(selected_keys, key) {
        index
    } else {
        None
    }
}

fn collect_import_column_options(columns: &ImportColumns) -> Vec<ImportColumnOption> {
    let mut options = vec![ImportColumnOption {
        key: "employeeId".to_string(),
        label: "EE. ID".to_string(),
        source: "required".to_string(),
        required: true,
    }];

    let mut push_core = |key: &str, present: bool| {
        if !present {
            return;
        }

        options.push(ImportColumnOption {
            key: key.to_string(),
            label: core_column_label(key),
            source: "core".to_string(),
            required: false,
        });
    };

    push_core("fullName", columns.full_name.is_some());
    push_core("nickName", columns.nick_name.is_some());
    push_core(
        "teamName",
        columns.client_pmd.is_some() || columns.project.is_some(),
    );
    push_core("project", columns.project.is_some());
    push_core("jobTitle", columns.job_title.is_some());
    push_core("email", columns.email.is_some());
    push_core("cellphone", columns.cellphone.is_some());
    push_core("dateOfBirth", columns.date_of_birth.is_some());
    push_core("gender", columns.gender.is_some());
    push_core("aswStartDate", columns.asw_start_date.is_some());
    push_core("clientStartDate", columns.client_start_date.is_some());
    push_core("contractEndDate", columns.contract_end_date.is_some());
    push_core(
        "clientYearOfServices",
        columns.client_year_of_services.is_some(),
    );
    push_core("computerName", columns.computer_name.is_some());
    push_core("notes", columns.notes.is_some());

    for dynamic in &columns.dynamic_columns {
        options.push(ImportColumnOption {
            key: dynamic.field_key.clone(),
            label: dynamic.field_label.clone(),
            source: "dynamic".to_string(),
            required: false,
        });
    }

    options
}

fn core_column_label(key: &str) -> String {
    CORE_COLUMN_DEFINITIONS
        .iter()
        .find_map(|(item_key, item_label)| {
            if *item_key == key {
                Some((*item_label).to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| key.to_string())
}

fn merge_import_text(
    existing: Option<String>,
    row: &[Data],
    index: Option<usize>,
) -> Option<String> {
    match index {
        Some(_) => extract_optional_value(row, index).or(existing),
        None => existing,
    }
}

fn merge_import_date(
    existing: Option<String>,
    row: &[Data],
    index: Option<usize>,
) -> Option<String> {
    match index {
        Some(_) => extract_date_value(row, index).or(existing),
        None => existing,
    }
}

fn merge_import_optional_or_date(
    existing: Option<String>,
    row: &[Data],
    index: Option<usize>,
) -> Option<String> {
    match index {
        Some(_) => extract_optional_or_date_value(row, index).or(existing),
        None => existing,
    }
}

fn merge_import_team_name(
    existing: Option<String>,
    row: &[Data],
    client_pmd_index: Option<usize>,
    project_index: Option<usize>,
) -> Option<String> {
    let imported = extract_optional_value(row, client_pmd_index)
        .or_else(|| extract_optional_value(row, project_index));
    imported.or(existing)
}

fn resolve_database_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|err| format!("failed to resolve app data directory: {err}"))?;

    fs::create_dir_all(&data_dir)
        .map_err(|err| format!("failed to create app data directory: {err}"))?;

    data_dir.push(DB_FILE_NAME);
    Ok(data_dir)
}

fn configure_connection(conn: &Connection) -> Result<(), String> {
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|err| format!("failed to set sqlite busy timeout: {err}"))?;

    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
        .map_err(|err| format!("failed to configure sqlite pragmas: {err}"))?;

    Ok(())
}

fn ensure_database_ready(app: &AppHandle) -> Result<(), String> {
    let result = DB_READY.get_or_init(|| {
        let db_path = resolve_database_path(app)?;
        let conn = Connection::open(&db_path)
            .map_err(|err| format!("failed to open sqlite database: {err}"))?;

        configure_connection(&conn)?;
        apply_migrations(&conn)?;
        Ok(())
    });

    match result {
        Ok(()) => Ok(()),
        Err(err) => Err(err.clone()),
    }
}

fn apply_migrations(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(BASE_SCHEMA_SQL)
        .map_err(|err| format!("failed to initialize schema: {err}"))?;

    ensure_employee_columns(conn)?;
    ensure_local_accounts_seed(conn)?;
    normalize_staff_group_values(conn)?;
    ensure_search_index(conn)?;

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

fn normalize_staff_group_values(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "UPDATE employees SET staff_group = ? WHERE staff_group = ?",
        params![STAFF_GROUP_INTERNAL_MOVEMENT, "internal_movent"],
    )
    .map_err(|err| format!("failed to normalize legacy staff_group values: {err}"))?;

    Ok(())
}

fn ensure_local_accounts_seed(conn: &Connection) -> Result<(), String> {
    let total_accounts: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_local_accounts", [], |row| row.get(0))
        .map_err(|err| format!("failed to count local accounts: {err}"))?;

    if total_accounts <= 0 {
        conn.execute(
            r#"
            INSERT INTO app_local_accounts(account_key, display_name, role, created_at, updated_at)
            VALUES(?, ?, ?, datetime('now'), datetime('now'))
            "#,
            params![
                DEFAULT_LOCAL_ACCOUNT_KEY,
                DEFAULT_LOCAL_ACCOUNT_NAME,
                LOCAL_ACCOUNT_ROLE_ADMIN
            ],
        )
        .map_err(humanize_sqlite_error)?;
    }

    let active_id = get_active_local_account_id(conn)?;
    if let Some(id) = active_id {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM app_local_accounts WHERE id = ?",
                params![id],
                |row| row.get(0),
            )
            .map_err(|err| format!("failed to verify active local account: {err}"))?;
        if exists > 0 {
            return Ok(());
        }
    }

    let fallback_id: i64 = conn
        .query_row(
            "SELECT id FROM app_local_accounts ORDER BY created_at ASC, id ASC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to resolve fallback local account: {err}"))?;

    set_active_local_account_id(conn, fallback_id)?;
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
              team_name
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
              COALESCE(t.name, '')
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

fn open_runtime_connection(app: &AppHandle) -> Result<Connection, String> {
    ensure_database_ready(app)?;

    let db_path = resolve_database_path(app)?;
    let conn = Connection::open(db_path)
        .map_err(|err| format!("failed to open sqlite database: {err}"))?;
    configure_connection(&conn)?;

    Ok(conn)
}

fn sqlite_version(conn: &Connection) -> Result<String, String> {
    conn.query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .map_err(|err| format!("failed to query sqlite version: {err}"))
}

fn query_employees(
    conn: &Connection,
    filters: EmployeeQuery,
) -> Result<EmployeeListResponse, String> {
    let limit = i64::from(filters.limit.unwrap_or(20).clamp(1, 5000));
    let offset = i64::from(filters.offset.unwrap_or(0));

    let mut from_clause = " FROM employees e LEFT JOIN teams t ON t.id = e.team_id ".to_string();
    let mut where_clauses: Vec<String> = Vec::new();
    let mut filter_params: Vec<Value> = Vec::new();

    if let Some(query) = normalize_optional_text(filters.query) {
        if let Some(fts_query) = build_fts_query(&query) {
            from_clause.push_str(" INNER JOIN employees_fts ON employees_fts.rowid = e.id ");
            where_clauses.push("employees_fts MATCH ?".to_string());
            filter_params.push(Value::Text(fts_query));
        }
    }

    if let Some(team_name) = normalize_optional_text(filters.team_name) {
        where_clauses.push("t.name = ?".to_string());
        filter_params.push(Value::Text(team_name));
    }

    if let Some(raw_group) = normalize_optional_text(filters.staff_group) {
        let Some(staff_group) = normalize_staff_group(raw_group.as_str()) else {
            return Err(format!("invalid staff group filter: {raw_group}"));
        };

        where_clauses.push("CASE WHEN COALESCE(NULLIF(TRIM(e.staff_group), ''), 'employee_list') = 'internal_movent' THEN 'internal_movement' ELSE COALESCE(NULLIF(TRIM(e.staff_group), ''), 'employee_list') END = ?".to_string());
        filter_params.push(Value::Text(staff_group.to_string()));
    }

    if let Some(start_from) = normalize_date_value(filters.start_date_from) {
        where_clauses.push("COALESCE(e.asw_start_date, e.start_date, '') >= ?".to_string());
        filter_params.push(Value::Text(start_from));
    }

    if let Some(start_to) = normalize_date_value(filters.start_date_to) {
        where_clauses.push("COALESCE(e.asw_start_date, e.start_date, '') <= ?".to_string());
        filter_params.push(Value::Text(start_to));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_clauses.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) {from_clause}{where_sql}");
    let total: i64 = conn
        .query_row(&count_sql, params_from_iter(filter_params.iter()), |row| {
            row.get(0)
        })
        .map_err(|err| format!("failed to count employees: {err}"))?;

    let select_sql = format!(
        "SELECT {EMPLOYEE_SELECT_COLUMNS} {from_clause}{where_sql} ORDER BY e.full_name COLLATE NOCASE ASC LIMIT ? OFFSET ?"
    );

    let mut select_params = filter_params;
    select_params.push(Value::Integer(limit));
    select_params.push(Value::Integer(offset));

    let mut stmt = conn
        .prepare(&select_sql)
        .map_err(|err| format!("failed to prepare employee query: {err}"))?;

    let rows = stmt
        .query_map(params_from_iter(select_params.iter()), map_employee_row)
        .map_err(|err| format!("failed to query employees: {err}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|err| format!("failed to read employee row: {err}"))?);
    }
    hydrate_dynamic_fields(conn, &mut items)?;

    Ok(EmployeeListResponse {
        items,
        total,
        limit,
        offset,
    })
}

fn load_employee_by_id(conn: &Connection, id: i64) -> Result<EmployeeRecord, String> {
    let sql = format!(
        "SELECT {EMPLOYEE_SELECT_COLUMNS} FROM employees e LEFT JOIN teams t ON t.id = e.team_id WHERE e.id = ?"
    );

    let mut employee = conn
        .query_row(&sql, params![id], map_employee_row)
        .optional()
        .map_err(|err| format!("failed to load employee: {err}"))?
        .ok_or_else(|| format!("employee with id {id} was not found"))?;

    let mut single = vec![employee];
    hydrate_dynamic_fields(conn, &mut single)?;
    employee = single
        .into_iter()
        .next()
        .ok_or_else(|| "failed to hydrate employee dynamic fields".to_string())?;

    Ok(employee)
}

fn load_employee_by_employee_id(
    conn: &Connection,
    employee_id: &str,
) -> Result<Option<EmployeeRecord>, String> {
    let sql = format!(
        "SELECT {EMPLOYEE_SELECT_COLUMNS} FROM employees e LEFT JOIN teams t ON t.id = e.team_id WHERE e.employee_id = ?"
    );

    let maybe_employee = conn
        .query_row(&sql, params![employee_id], map_employee_row)
        .optional()
        .map_err(|err| format!("failed to load employee by employee_id: {err}"))?;

    let Some(employee) = maybe_employee else {
        return Ok(None);
    };

    let mut single = vec![employee];
    hydrate_dynamic_fields(conn, &mut single)?;
    Ok(single.into_iter().next())
}

fn load_team_by_id(conn: &Connection, id: i64) -> Result<TeamRecord, String> {
    conn.query_row(
        r#"
        SELECT
          t.id,
          t.name,
          COUNT(e.id) AS member_count
        FROM teams t
        LEFT JOIN employees e ON e.team_id = t.id
        WHERE t.id = ?
        GROUP BY t.id, t.name
        "#,
        params![id],
        |row| {
            Ok(TeamRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                member_count: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to load team: {err}"))?
    .ok_or_else(|| format!("team with id {id} was not found"))
}

fn normalize_local_account_role(value: Option<String>) -> String {
    let normalized = normalize_optional_text(value).unwrap_or_else(|| LOCAL_ACCOUNT_ROLE_USER.to_string());
    if normalized.eq_ignore_ascii_case(LOCAL_ACCOUNT_ROLE_ADMIN) {
        LOCAL_ACCOUNT_ROLE_ADMIN.to_string()
    } else {
        LOCAL_ACCOUNT_ROLE_USER.to_string()
    }
}

fn local_account_key_exists(conn: &Connection, key: &str) -> Result<bool, String> {
    let exists: i64 = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM app_local_accounts WHERE account_key = ?)",
            params![key],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to check account key existence: {err}"))?;
    Ok(exists > 0)
}

fn generate_local_account_key(conn: &Connection, display_name: &str) -> Result<String, String> {
    let base = normalize_dynamic_key(display_name);
    let base = if base.is_empty() { "user".to_string() } else { base };

    if !local_account_key_exists(conn, base.as_str())? {
        return Ok(base);
    }

    for index in 2..=9999 {
        let candidate = format!("{base}_{index}");
        if !local_account_key_exists(conn, candidate.as_str())? {
            return Ok(candidate);
        }
    }

    Err("failed to allocate local account key".to_string())
}

fn get_active_local_account_id(conn: &Connection) -> Result<Option<i64>, String> {
    let maybe_value: Option<String> = conn
        .query_row(
            "SELECT setting_value FROM app_settings WHERE setting_key = ?",
            params![ACTIVE_LOCAL_ACCOUNT_SETTING_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to read active local account setting: {err}"))?;

    let Some(value) = maybe_value else {
        return Ok(None);
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    trimmed
        .parse::<i64>()
        .map(Some)
        .map_err(|_| "active local account setting is invalid".to_string())
}

fn get_active_local_account_id_tx(tx: &Transaction<'_>) -> Result<Option<i64>, String> {
    let maybe_value: Option<String> = tx
        .query_row(
            "SELECT setting_value FROM app_settings WHERE setting_key = ?",
            params![ACTIVE_LOCAL_ACCOUNT_SETTING_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to read active local account setting: {err}"))?;

    let Some(value) = maybe_value else {
        return Ok(None);
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    trimmed
        .parse::<i64>()
        .map(Some)
        .map_err(|_| "active local account setting is invalid".to_string())
}

fn set_active_local_account_id(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute(
        r#"
        INSERT INTO app_settings(setting_key, setting_value, updated_at)
        VALUES(?, ?, datetime('now'))
        ON CONFLICT(setting_key) DO UPDATE SET
          setting_value = excluded.setting_value,
          updated_at = datetime('now')
        "#,
        params![ACTIVE_LOCAL_ACCOUNT_SETTING_KEY, id.to_string()],
    )
    .map_err(humanize_sqlite_error)?;

    Ok(())
}

fn set_active_local_account_id_tx(tx: &Transaction<'_>, id: i64) -> Result<(), String> {
    tx.execute(
        r#"
        INSERT INTO app_settings(setting_key, setting_value, updated_at)
        VALUES(?, ?, datetime('now'))
        ON CONFLICT(setting_key) DO UPDATE SET
          setting_value = excluded.setting_value,
          updated_at = datetime('now')
        "#,
        params![ACTIVE_LOCAL_ACCOUNT_SETTING_KEY, id.to_string()],
    )
    .map_err(humanize_sqlite_error)?;

    Ok(())
}

fn query_local_accounts(conn: &Connection) -> Result<Vec<LocalAccountRecord>, String> {
    let mut active_id = get_active_local_account_id(conn)?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, account_key, display_name, role, created_at, updated_at
            FROM app_local_accounts
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare local account query: {err}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|err| format!("failed to query local accounts: {err}"))?;

    let mut items = Vec::new();
    for row in rows {
        let (id, account_key, display_name, role_raw, created_at, updated_at) =
            row.map_err(|err| format!("failed to read local account row: {err}"))?;
        let role = if role_raw.eq_ignore_ascii_case(LOCAL_ACCOUNT_ROLE_ADMIN) {
            LOCAL_ACCOUNT_ROLE_ADMIN.to_string()
        } else {
            LOCAL_ACCOUNT_ROLE_USER.to_string()
        };

        items.push(LocalAccountRecord {
            id,
            account_key,
            display_name,
            role,
            is_active: false,
            created_at,
            updated_at,
        });
    }

    if active_id.is_none() && !items.is_empty() {
        active_id = Some(items[0].id);
        set_active_local_account_id(conn, items[0].id)?;
    }

    for item in &mut items {
        item.is_active = active_id == Some(item.id);
    }

    Ok(items)
}

fn load_local_account_by_id(conn: &Connection, id: i64) -> Result<LocalAccountRecord, String> {
    let active_id = get_active_local_account_id(conn)?;
    conn.query_row(
        r#"
        SELECT id, account_key, display_name, role, created_at, updated_at
        FROM app_local_accounts
        WHERE id = ?
        "#,
        params![id],
        |row| {
            let role_raw: String = row.get(3)?;
            let role = if role_raw.eq_ignore_ascii_case(LOCAL_ACCOUNT_ROLE_ADMIN) {
                LOCAL_ACCOUNT_ROLE_ADMIN.to_string()
            } else {
                LOCAL_ACCOUNT_ROLE_USER.to_string()
            };
            Ok(LocalAccountRecord {
                id: row.get(0)?,
                account_key: row.get(1)?,
                display_name: row.get(2)?,
                role,
                is_active: active_id == Some(id),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to load local account: {err}"))?
    .ok_or_else(|| format!("local account with id {id} was not found"))
}

fn map_employee_row(row: &Row<'_>) -> rusqlite::Result<EmployeeRecord> {
    Ok(EmployeeRecord {
        id: row.get(0)?,
        employee_id: row.get(1)?,
        full_name: row.get(2)?,
        nick_name: row.get(3)?,
        team_id: row.get(4)?,
        team_name: row.get(5)?,
        project: row.get(6)?,
        job_title: row.get(7)?,
        email: row.get(8)?,
        cellphone: row.get(9)?,
        date_of_birth: row.get(10)?,
        gender: row.get(11)?,
        asw_start_date: row.get(12)?,
        client_start_date: row.get(13)?,
        contract_end_date: row.get(14)?,
        client_year_of_services: row.get(15)?,
        start_date: row.get(16)?,
        computer_name: row.get(17)?,
        notes: row.get(18)?,
        staff_group: row.get(19)?,
        dynamic_fields: HashMap::new(),
        updated_at: row.get(20)?,
    })
}

fn hydrate_dynamic_fields(conn: &Connection, items: &mut [EmployeeRecord]) -> Result<(), String> {
    if items.is_empty() {
        return Ok(());
    }

    let ids = items.iter().map(|item| item.id).collect::<Vec<_>>();
    let placeholders = vec!["?"; ids.len()].join(", ");
    let sql = format!(
        "SELECT employee_id, field_key, value FROM employee_dynamic_values WHERE employee_id IN ({placeholders})"
    );

    let id_to_index = items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.id, index))
        .collect::<HashMap<_, _>>();

    let params = ids.iter().map(|id| Value::Integer(*id)).collect::<Vec<_>>();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("failed to prepare dynamic field query: {err}"))?;
    let rows = stmt
        .query_map(params_from_iter(params.iter()), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|err| format!("failed to query dynamic fields: {err}"))?;

    for row in rows {
        let (employee_id, field_key, value) =
            row.map_err(|err| format!("failed to read dynamic field row: {err}"))?;
        let Some(index) = id_to_index.get(&employee_id).copied() else {
            continue;
        };

        if let Some(text) = normalize_optional_text(value) {
            items[index].dynamic_fields.insert(field_key, text);
        }
    }

    Ok(())
}

fn upsert_dynamic_field_definitions_tx(
    tx: &Transaction<'_>,
    columns: &[DynamicImportColumn],
) -> Result<(), String> {
    for column in columns {
        tx.execute(
            r#"
            INSERT INTO employee_dynamic_fields(field_key, field_label, updated_at)
            VALUES(?, ?, datetime('now'))
            ON CONFLICT(field_key) DO UPDATE SET
              field_label = excluded.field_label,
              updated_at = datetime('now')
            "#,
            params![column.field_key.as_str(), column.field_label.as_str()],
        )
        .map_err(humanize_sqlite_error)?;
    }

    Ok(())
}

fn upsert_dynamic_field_definitions_for_map(
    tx: &Transaction<'_>,
    fields: &HashMap<String, String>,
) -> Result<(), String> {
    for key in fields.keys() {
        let label = dynamic_key_to_label(key);
        tx.execute(
            r#"
            INSERT INTO employee_dynamic_fields(field_key, field_label, updated_at)
            VALUES(?, ?, datetime('now'))
            ON CONFLICT(field_key) DO NOTHING
            "#,
            params![key.as_str(), label.as_str()],
        )
        .map_err(humanize_sqlite_error)?;
    }

    Ok(())
}

fn upsert_dynamic_fields_tx(
    tx: &Transaction<'_>,
    employee_id: i64,
    fields: &HashMap<String, String>,
) -> Result<(), String> {
    for (field_key, value) in fields {
        tx.execute(
            r#"
            INSERT INTO employee_dynamic_values(employee_id, field_key, value, updated_at)
            VALUES(?, ?, ?, datetime('now'))
            ON CONFLICT(employee_id, field_key) DO UPDATE SET
              value = excluded.value,
              updated_at = datetime('now')
            "#,
            params![employee_id, field_key.as_str(), value.as_str()],
        )
        .map_err(humanize_sqlite_error)?;
    }

    Ok(())
}

fn resolve_team_id_tx(
    tx: &Transaction<'_>,
    team_name: Option<&str>,
) -> Result<Option<i64>, String> {
    let Some(name) = team_name else {
        return Ok(None);
    };

    let Some(normalized_name) = normalize_optional_text(Some(name.to_string())) else {
        return Ok(None);
    };

    tx.execute(
        "INSERT INTO teams(name) VALUES (?) ON CONFLICT(name) DO NOTHING",
        params![normalized_name.as_str()],
    )
    .map_err(humanize_sqlite_error)?;

    tx.query_row(
        "SELECT id FROM teams WHERE name = ?",
        params![normalized_name.as_str()],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| format!("failed to resolve team id: {err}"))?
    .ok_or_else(|| "failed to resolve team id after insert".to_string())
    .map(Some)
}

fn upsert_employee_from_payload(
    tx: &Transaction<'_>,
    payload: EmployeePayload,
    staff_group: &str,
) -> Result<UpsertAction, String> {
    let normalized = NormalizedEmployeePayload::try_from(payload)?;
    let team_id = resolve_team_id_tx(tx, normalized.team_name.as_deref())?;

    let existing_id: Option<i64> = tx
        .query_row(
            "SELECT id FROM employees WHERE employee_id = ?",
            params![normalized.employee_id.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to check employee existence: {err}"))?;

    if let Some(id) = existing_id {
        tx.execute(
            r#"
            UPDATE employees
            SET
              full_name = ?,
              nick_name = ?,
              team_id = ?,
              project = ?,
              job_title = ?,
              email = ?,
              cellphone = ?,
              date_of_birth = ?,
              gender = ?,
              asw_start_date = ?,
              client_start_date = ?,
              contract_end_date = ?,
              client_year_of_services = ?,
              start_date = ?,
              computername = ?,
              notes = ?,
              staff_group = ?,
              updated_at = datetime('now')
            WHERE id = ?
            "#,
            params![
                normalized.full_name.as_str(),
                normalized.nick_name.as_deref(),
                team_id,
                normalized.project.as_deref(),
                normalized.job_title.as_deref(),
                normalized.email.as_deref(),
                normalized.cellphone.as_deref(),
                normalized.date_of_birth.as_deref(),
                normalized.gender.as_deref(),
                normalized.asw_start_date.as_deref(),
                normalized.client_start_date.as_deref(),
                normalized.contract_end_date.as_deref(),
                normalized.client_year_of_services.as_deref(),
                normalized.start_date.as_deref(),
                normalized.computer_name.as_deref(),
                normalized.notes.as_deref(),
                staff_group,
                id,
            ],
        )
        .map_err(humanize_sqlite_error)?;

        if !normalized.dynamic_fields.is_empty() {
            upsert_dynamic_field_definitions_for_map(tx, &normalized.dynamic_fields)?;
            upsert_dynamic_fields_tx(tx, id, &normalized.dynamic_fields)?;
        }

        return Ok(UpsertAction::Updated);
    }

    tx.execute(
        r#"
        INSERT INTO employees (
          employee_id,
          full_name,
          nick_name,
          team_id,
          project,
          job_title,
          email,
          cellphone,
          date_of_birth,
          gender,
          asw_start_date,
          client_start_date,
          contract_end_date,
          client_year_of_services,
          start_date,
          computername,
          notes,
          staff_group,
          updated_at
        ) VALUES (
          ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now')
        )
        "#,
        params![
            normalized.employee_id.as_str(),
            normalized.full_name.as_str(),
            normalized.nick_name.as_deref(),
            team_id,
            normalized.project.as_deref(),
            normalized.job_title.as_deref(),
            normalized.email.as_deref(),
            normalized.cellphone.as_deref(),
            normalized.date_of_birth.as_deref(),
            normalized.gender.as_deref(),
            normalized.asw_start_date.as_deref(),
            normalized.client_start_date.as_deref(),
            normalized.contract_end_date.as_deref(),
            normalized.client_year_of_services.as_deref(),
            normalized.start_date.as_deref(),
            normalized.computer_name.as_deref(),
            normalized.notes.as_deref(),
            staff_group,
        ],
    )
    .map_err(humanize_sqlite_error)?;

    let inserted_id = tx.last_insert_rowid();
    if !normalized.dynamic_fields.is_empty() {
        upsert_dynamic_field_definitions_for_map(tx, &normalized.dynamic_fields)?;
        upsert_dynamic_fields_tx(tx, inserted_id, &normalized.dynamic_fields)?;
    }

    Ok(UpsertAction::Inserted)
}

fn detect_import_columns(range: &calamine::Range<Data>) -> Result<(usize, ImportColumns), String> {
    const HEADER_EMPLOYEE_ID: &[&str] = &[
        "eeid",
        "emid",
        "employeeid",
        "employeecode",
        "staffid",
        "staffcode",
        "staffcodeid",
        "manhanvien",
        "m\u{00E3}nh\u{00E2}nvi\u{00EA}n",
        "m\u{00E3}nv",
        "manv",
    ];
    const HEADER_FULL_NAME: &[&str] = &[
        "vietnamesename",
        "fullname",
        "yourfullname",
        "hoten",
        "h\u{1ECD}t\u{00EA}n",
        "name",
        "englishname",
    ];
    const HEADER_ASW_START_DATE: &[&str] = &["aswstartdate", "aswigstartdate", "startdate"];
    const HEADER_CLIENT_START_DATE: &[&str] = &[
        "clientstartdate",
        "currentclientstartdate",
        "newclientpmdjoindate",
        "lastclientstartdate",
    ];
    const HEADER_NICK_NAME: &[&str] = &["nickname", "nick"];
    const HEADER_CLIENT_PMD: &[&str] = &[
        "clientpmd",
        "newclientpmd",
        "formerclientpmd",
        "formerclient",
        "newclient",
        "client",
        "team",
        "department",
    ];
    const HEADER_PROJECT: &[&str] = &["project", "projectdeprt", "projectdept"];
    const HEADER_JOB_TITLE: &[&str] = &[
        "currentjobtitle",
        "offeredjobtitle",
        "lastedjobtitle",
        "newjobtitle",
        "startingtitlee",
        "jobtitle",
        "title",
    ];
    const HEADER_EMAIL: &[&str] = &[
        "workingemail",
        "formerworkingemail",
        "newworkingemail",
        "personalemail",
        "email",
    ];
    const HEADER_CELLPHONE: &[&str] = &[
        "cellphone",
        "phone",
        "mobilenumber",
        "mobile",
        "phonenumber",
    ];
    const HEADER_DOB: &[&str] = &["dob", "dateofbirth", "birthday", "yob", "yearofbirth"];
    const HEADER_GENDER: &[&str] = &["gender", "sex"];
    const HEADER_CONTRACT_END: &[&str] = &[
        "contractenddate",
        "aswlwd",
        "aswenddate",
        "formerenddate",
        "enddate",
    ];
    const HEADER_CLIENT_YOS: &[&str] = &[
        "clientyearofservices",
        "yearofservices",
        "services",
        "formerservices",
    ];
    const HEADER_COMPUTER_NAME: &[&str] = &["computername", "computer", "tenmay"];
    const HEADER_NOTES: &[&str] = &["notes", "note", "ghichu", "remark", "remarkdetails"];

    for (row_index, row) in range.rows().enumerate().take(120) {
        let mut headers: HashMap<String, usize> = HashMap::new();
        let mut header_entries: Vec<(usize, String, String)> = Vec::new();

        for (column_index, cell) in row.iter().enumerate() {
            let raw_label = cell_to_string(cell);
            if raw_label.trim().is_empty() {
                continue;
            }

            let key = normalize_header_key(&raw_label);
            if key.is_empty() {
                continue;
            }

            headers.insert(key.clone(), column_index);
            header_entries.push((column_index, raw_label.trim().to_string(), key));
        }

        let Some(employee_id) = find_column_index(&headers, HEADER_EMPLOYEE_ID) else {
            continue;
        };

        let full_name = find_column_index(&headers, HEADER_FULL_NAME);

        let asw_start_date = find_column_index(&headers, HEADER_ASW_START_DATE);
        let client_start_date = find_column_index(&headers, HEADER_CLIENT_START_DATE);
        let nick_name = find_column_index(&headers, HEADER_NICK_NAME);
        let client_pmd = find_column_index(&headers, HEADER_CLIENT_PMD);
        let project = find_column_index(&headers, HEADER_PROJECT);
        let job_title = find_column_index(&headers, HEADER_JOB_TITLE);
        let email = find_column_index(&headers, HEADER_EMAIL);
        let cellphone = find_column_index(&headers, HEADER_CELLPHONE);
        let date_of_birth = find_column_index(&headers, HEADER_DOB);
        let gender = find_column_index(&headers, HEADER_GENDER);
        let contract_end_date = find_column_index(&headers, HEADER_CONTRACT_END);
        let client_year_of_services = find_column_index(&headers, HEADER_CLIENT_YOS);
        let computer_name = find_column_index(&headers, HEADER_COMPUTER_NAME);
        let notes = find_column_index(&headers, HEADER_NOTES);

        let known_indexes = [
            Some(employee_id),
            full_name,
            asw_start_date,
            client_start_date,
            nick_name,
            client_pmd,
            project,
            job_title,
            email,
            cellphone,
            date_of_birth,
            gender,
            contract_end_date,
            client_year_of_services,
            computer_name,
            notes,
        ]
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>();

        let mut seen_dynamic_keys = HashSet::new();
        let mut dynamic_columns = Vec::new();
        for (index, label, header_key) in &header_entries {
            if known_indexes.contains(index) {
                continue;
            }

            if should_skip_dynamic_import_column(header_key) {
                continue;
            }

            let key = normalize_dynamic_key(label);
            if key.is_empty() || !seen_dynamic_keys.insert(key.clone()) {
                continue;
            }

            dynamic_columns.push(DynamicImportColumn {
                index: *index,
                field_key: key,
                field_label: label.clone(),
            });
        }

        let columns = ImportColumns {
            asw_start_date,
            employee_id,
            client_start_date,
            full_name,
            nick_name,
            client_pmd,
            project,
            job_title,
            email,
            cellphone,
            date_of_birth,
            gender,
            contract_end_date,
            client_year_of_services,
            computer_name,
            notes,
            dynamic_columns,
        };

        return Ok((row_index, columns));
    }

    Err(
        "failed to detect import header row: required staff ID column not found (supported aliases include EE. ID, Em. ID, Staff ID, Ma nhan vien)"
            .to_string(),
    )
}

fn should_skip_dynamic_import_column(header_key: &str) -> bool {
    header_key == "question" || header_key.contains("ctyaswhitevn")
}

fn normalize_staff_group(value: &str) -> Option<&'static str> {
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

fn infer_staff_group_from_source(source_name: &str, sheet_name: &str) -> &'static str {
    let normalized = normalize_header_key(format!("{source_name} {sheet_name}").as_str());

    if normalized.contains("onboarding") {
        return STAFF_GROUP_ONBOARDING;
    }
    if normalized.contains("offboarding") {
        return STAFF_GROUP_OFFBOARDING;
    }
    if normalized.contains("internalmovement")
        || normalized.contains("internalmovent")
        || normalized.contains("internalmove")
    {
        return STAFF_GROUP_INTERNAL_MOVEMENT;
    }
    if normalized.contains("employeelist") || normalized.contains("eelist") {
        return STAFF_GROUP_EMPLOYEE_LIST;
    }

    STAFF_GROUP_EMPLOYEE_LIST
}

fn find_column_index(headers: &HashMap<String, usize>, aliases: &[&str]) -> Option<usize> {
    aliases
        .iter()
        .find_map(|alias| headers.get(*alias).copied())
}

fn normalize_header_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect::<String>()
}

fn normalize_dynamic_key(value: &str) -> String {
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

fn normalize_dynamic_fields(input: Option<HashMap<String, String>>) -> HashMap<String, String> {
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

fn dynamic_key_to_label(key: &str) -> String {
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

fn is_reserved_column_key(key: &str) -> bool {
    matches!(key, "rownumber" | "row_number" | "actions" | "action")
        || CORE_COLUMN_DEFINITIONS
            .iter()
            .any(|(core_key, _)| normalize_dynamic_key(core_key) == key)
}

fn dynamic_field_exists(conn: &Connection, key: &str) -> Result<bool, String> {
    let exists = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM employee_dynamic_fields WHERE field_key = ?)",
            params![key],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|err| format!("failed to check dynamic field existence: {err}"))?;

    Ok(exists > 0)
}

fn generate_dynamic_field_key(conn: &Connection, label: &str) -> Result<String, String> {
    let base_key = normalize_dynamic_key(label);
    if base_key.is_empty() {
        return Err("column title is invalid".to_string());
    }

    if !is_reserved_column_key(base_key.as_str()) && !dynamic_field_exists(conn, base_key.as_str())?
    {
        return Ok(base_key);
    }

    for index in 2..=9999 {
        let candidate = format!("{base_key}_{index}");
        if is_reserved_column_key(candidate.as_str()) {
            continue;
        }

        if !dynamic_field_exists(conn, candidate.as_str())? {
            return Ok(candidate);
        }
    }

    Err("failed to allocate a unique dynamic column key".to_string())
}

fn row_is_empty(row: &[Data]) -> bool {
    row.iter().all(|cell| cell_to_string(cell).is_empty())
}
fn extract_optional_value(row: &[Data], index: Option<usize>) -> Option<String> {
    index
        .and_then(|idx| row.get(idx))
        .map(cell_to_string)
        .and_then(|value| normalize_optional_text(Some(value)))
}

fn extract_date_value(row: &[Data], index: Option<usize>) -> Option<String> {
    let cell = index.and_then(|idx| row.get(idx))?;

    if let Some(number) = cell.get_float() {
        if let Some(date_value) = excel_serial_to_iso(number) {
            return Some(date_value);
        }
    }

    if let Some(number) = cell.get_int() {
        if let Some(date_value) = excel_serial_to_iso(number as f64) {
            return Some(date_value);
        }
    }

    normalize_date_value(Some(cell_to_string(cell)))
}

fn extract_optional_or_date_value(row: &[Data], index: Option<usize>) -> Option<String> {
    let value = extract_optional_value(row, index)?;
    normalize_date_text(&value).or(Some(value))
}

fn resolve_import_source_paths(
    requested: Option<String>,
    requested_many: Option<Vec<String>>,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    let mut seen_paths: HashSet<String> = HashSet::new();

    if let Some(raw_paths) = requested_many {
        for raw_path in raw_paths {
            let Some(path_text) = normalize_optional_text(Some(raw_path)) else {
                continue;
            };

            let candidate = PathBuf::from(path_text);
            if !candidate.exists() {
                return Err(format!(
                    "import source file does not exist: {}",
                    candidate.display()
                ));
            }

            if !is_excel_file(&candidate) {
                continue;
            }

            let key = candidate.to_string_lossy().to_string();
            if seen_paths.insert(key) {
                paths.push(candidate);
            }
        }
    }

    if let Some(path_text) = normalize_optional_text(requested) {
        let candidate = PathBuf::from(path_text);
        if !candidate.exists() {
            return Err(format!(
                "import source file does not exist: {}",
                candidate.display()
            ));
        }

        if is_excel_file(&candidate) {
            let key = candidate.to_string_lossy().to_string();
            if seen_paths.insert(key) {
                paths.push(candidate);
            }
        }
    }

    if !paths.is_empty() {
        return Ok(paths);
    }

    let cwd = std::env::current_dir()
        .map_err(|err| format!("failed to resolve current directory for import: {err}"))?;

    let mut candidate_dirs = vec![cwd.join("ExSource"), cwd.join("Exsource")];
    if let Some(parent) = cwd.parent() {
        candidate_dirs.push(parent.join("ExSource"));
        candidate_dirs.push(parent.join("Exsource"));
    }

    let mut excel_files: Vec<PathBuf> = Vec::new();

    for dir in candidate_dirs {
        if !dir.exists() {
            continue;
        }

        let entries = fs::read_dir(&dir)
            .map_err(|err| format!("failed to read import directory '{}': {err}", dir.display()))?;

        for entry in entries {
            let entry =
                entry.map_err(|err| format!("failed to read import directory entry: {err}"))?;
            let path = entry.path();
            if is_excel_file(&path) && seen_paths.insert(path.to_string_lossy().to_string()) {
                excel_files.push(path);
            }
        }
    }

    if excel_files.is_empty() {
        return Err(
            "no Excel source found under ExSource/ or Exsource/. Provide filePath(s) or place .xlsx in that folder"
                .to_string(),
        );
    }

    excel_files.sort_by_key(|path| {
        fs::metadata(path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
    });

    Ok(vec![excel_files.pop().ok_or_else(|| {
        "no Excel source found under ExSource/".to_string()
    })?])
}

fn is_excel_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };

    matches!(ext.to_ascii_lowercase().as_str(), "xlsx" | "xlsm" | "xls")
}

fn build_fts_query(raw: &str) -> Option<String> {
    let tokens = raw
        .split_whitespace()
        .map(|token| {
            token
                .chars()
                .filter(|ch| ch.is_alphanumeric() || *ch == '_')
                .collect::<String>()
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if tokens.is_empty() {
        return None;
    }

    Some(
        tokens
            .iter()
            .map(|token| format!("{token}*"))
            .collect::<Vec<_>>()
            .join(" AND "),
    )
}

fn normalize_employee_id(value: String) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "0" {
        return Err("employeeId is required".to_string());
    }

    Ok(trimmed.to_uppercase())
}

fn require_text(value: String, field_name: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_name} is required"));
    }

    Ok(trimmed.to_string())
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "0" {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_email(value: Option<String>) -> Option<String> {
    normalize_optional_text(value).map(|email| email.to_lowercase())
}

fn normalize_optional_or_date(value: Option<String>) -> Option<String> {
    let normalized = normalize_optional_text(value)?;
    normalize_date_text(&normalized).or(Some(normalized))
}

fn normalize_date_value(value: Option<String>) -> Option<String> {
    let normalized = normalize_optional_text(value)?;
    normalize_date_text(&normalized).or(Some(normalized))
}

fn normalize_date_text(value: &str) -> Option<String> {
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

    NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|date_time| date_time.date().format("%Y-%m-%d").to_string())
}

fn excel_serial_to_iso(serial: f64) -> Option<String> {
    if !serial.is_finite() {
        return None;
    }

    let days = serial.trunc();
    if days < 1.0 {
        return None;
    }

    let base = NaiveDate::from_ymd_opt(1899, 12, 30)?.and_hms_opt(0, 0, 0)?;
    let seconds = ((serial - days) * 86_400.0).round() as i64;
    let date_time = base + ChronoDuration::days(days as i64) + ChronoDuration::seconds(seconds);

    Some(date_time.date().format("%Y-%m-%d").to_string())
}

fn cell_to_string(cell: &Data) -> String {
    if let Some(value) = cell.get_string() {
        return value.trim().to_string();
    }

    if let Some(value) = cell.get_float() {
        return format_numeric(value);
    }

    if let Some(value) = cell.get_int() {
        return value.to_string();
    }

    if let Some(value) = cell.get_bool() {
        return if value {
            "true".to_string()
        } else {
            "false".to_string()
        };
    }

    cell.to_string().trim().to_string()
}

fn format_numeric(value: f64) -> String {
    let rounded = value.round();
    if (value - rounded).abs() < f64::EPSILON {
        return format!("{rounded:.0}");
    }

    let mut out = format!("{value}");
    if out.contains('.') {
        while out.ends_with('0') {
            out.pop();
        }
        if out.ends_with('.') {
            out.pop();
        }
    }

    out
}

fn humanize_sqlite_error(err: rusqlite::Error) -> String {
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
