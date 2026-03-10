// ──────────────────────────────────────────────────────────────────────────────
// Database constants: file name, SQL schema, FTS, column definitions
// ──────────────────────────────────────────────────────────────────────────────

pub(super) const DB_FILE_NAME: &str = "staff_kit.sqlite3";

// ── Staff groups ───────────────────────────────────────────────────────────────
pub(super) const STAFF_GROUP_EMPLOYEE_LIST: &str = "employee_list";
pub(super) const STAFF_GROUP_ONBOARDING: &str = "onboarding";
pub(super) const STAFF_GROUP_OFFBOARDING: &str = "offboarding";
pub(super) const STAFF_GROUP_INTERNAL_MOVEMENT: &str = "internal_movement";

// ── Local account roles / defaults ────────────────────────────────────────────
pub(super) const LOCAL_ACCOUNT_ROLE_SUPER_ADMIN: &str = "super_admin";
pub(super) const LOCAL_ACCOUNT_ROLE_ADMIN: &str = "admin";
pub(super) const LOCAL_ACCOUNT_ROLE_USER: &str = "user";
pub(super) const DEFAULT_LOCAL_ACCOUNT_NAME: &str = "adman";
pub(super) const DEFAULT_LOCAL_ACCOUNT_KEY: &str = "adman";
pub(super) const DEFAULT_LOCAL_ACCOUNT_USERNAME: &str = "adman";
pub(super) const DEFAULT_LOCAL_ACCOUNT_PASSWORD: &str = "Aswhite2026";
pub(super) const DEFAULT_LOCAL_ACCOUNT_RECOVERY_CODE: &str = "SK-RECOVERY-2026";
pub(super) const DEFAULT_NEW_LOCAL_ACCOUNT_PASSWORD: &str = "Welcome!";

// ── Database encryption ───────────────────────────────────────────────────────
/// AES-256 encryption key applied to every SQLite connection via PRAGMA key.
/// This is an app-level key stored in the binary — not a user password.
/// Changing this value requires migrating all existing databases.
pub(super) const APP_DB_ENCRYPTION_KEY: &str =
    "SK-AES256-staffkit-2026-io.staffkit.app";
pub(super) const DB_ENCRYPTION_MIGRATION_SETTING_KEY: &str = "db_encrypted_v1";

// ── Settings keys ─────────────────────────────────────────────────────────────
pub(super) const ACTIVE_LOCAL_ACCOUNT_SETTING_KEY: &str = "active_local_account_id";
pub(super) const DEFAULT_ADMIN_SEED_SETTING_KEY: &str = "default_admin_seed_v1";
pub(super) const BACKUP_DIRECTORY_SETTING_KEY: &str = "backup_directory_path";
pub(super) const AUTO_BACKUP_ENABLED_SETTING_KEY: &str = "backup_auto_enabled_v1";
pub(super) const AUTO_BACKUP_LAST_DATE_SETTING_KEY: &str = "backup_auto_last_date_v1";

// ── Backup ────────────────────────────────────────────────────────────────────
pub(super) const AUTO_BACKUP_RETENTION_FILES: usize = 7;
pub(super) const AUTO_BACKUP_INTERVAL_DAYS: i64 = 7;
pub(super) const AUTO_BACKUP_RETENTION_DAYS: i64 = 400;
pub(super) const BACKUP_FILE_PREFIX: &str = "staff_kit_backup";

// ── History snapshots ─────────────────────────────────────────────────────────
pub(super) const HISTORY_FOLDER_NAME: &str = "staff_kit_history";
pub(super) const HISTORY_RETENTION_COUNT: usize = 7;
pub(super) const HISTORY_FILE_PREFIX: &str = "snap";

// ── Configurable DB path ──────────────────────────────────────────────────────
pub(super) const DB_SETTINGS_FILE_NAME: &str = "db_settings.json";
#[allow(dead_code)]
pub(super) const DB_CUSTOM_PATH_KEY: &str = "custom_path";

// ── Computer Name dynamic field ───────────────────────────────────────────────
pub(crate) const COMPUTER_NAME_2_FIELD_KEY: &str = "computer_2";
pub(crate) const COMPUTER_NAME_2_FIELD_LABEL: &str = "Computer (2)";

// ── FTS columns ───────────────────────────────────────────────────────────────
pub(super) const FTS_COLUMNS: &[&str] = &[
    "employee_id",
    "full_name",
    "nick_name",
    "email",
    "project",
    "job_title",
    "notes",
    "team_name",
    "computername",
];

// ── Core column definitions (key, label) ──────────────────────────────────────
pub(crate) const CORE_COLUMN_DEFINITIONS: &[(&str, &str)] = &[
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

// ── SELECT columns for employee queries ───────────────────────────────────────
pub(super) const EMPLOYEE_SELECT_COLUMNS: &str = r#"
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

// ── Additional columns added via migration ────────────────────────────────────
pub(super) const EMPLOYEE_ADDITIONAL_COLUMNS: &[(&str, &str)] = &[
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

// ── Base schema SQL ───────────────────────────────────────────────────────────
pub(super) const BASE_SCHEMA_SQL: &str = r#"
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS teams (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  parent_id INTEGER REFERENCES teams(id) ON DELETE SET NULL
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
  username TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  recovery_code_hash TEXT,
  force_password_reset INTEGER NOT NULL DEFAULT 0,
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

// ── FTS virtual table ─────────────────────────────────────────────────────────
pub(super) const FTS_TABLE_SQL: &str = r#"
CREATE VIRTUAL TABLE employees_fts USING fts5(
  employee_id,
  full_name,
  nick_name,
  email,
  project,
  job_title,
  notes,
  team_name,
  computername,
  content='',
  tokenize='unicode61 remove_diacritics 2'
);
"#;

// ── FTS triggers ──────────────────────────────────────────────────────────────
pub(super) const FTS_TRIGGERS_SQL: &str = r#"
CREATE TRIGGER IF NOT EXISTS trg_employees_ai AFTER INSERT ON employees BEGIN
  INSERT INTO employees_fts(rowid, employee_id, full_name, nick_name, email, project, job_title, notes, team_name, computername)
  VALUES (
    NEW.id,
    NEW.employee_id,
    NEW.full_name,
    COALESCE(NEW.nick_name, ''),
    COALESCE(NEW.email, ''),
    COALESCE(NEW.project, ''),
    COALESCE(NEW.job_title, ''),
    COALESCE(NEW.notes, ''),
    COALESCE((SELECT name FROM teams WHERE id = NEW.team_id), ''),
    COALESCE(NEW.computername, '')
  );
END;

CREATE TRIGGER IF NOT EXISTS trg_employees_au AFTER UPDATE ON employees BEGIN
  INSERT INTO employees_fts(
    employees_fts,
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
  VALUES (
    'delete',
    OLD.id,
    OLD.employee_id,
    OLD.full_name,
    COALESCE(OLD.nick_name, ''),
    COALESCE(OLD.email, ''),
    COALESCE(OLD.project, ''),
    COALESCE(OLD.job_title, ''),
    COALESCE(OLD.notes, ''),
    COALESCE((SELECT name FROM teams WHERE id = OLD.team_id), ''),
    COALESCE(OLD.computername, '')
  );

  INSERT INTO employees_fts(rowid, employee_id, full_name, nick_name, email, project, job_title, notes, team_name, computername)
  VALUES (
    NEW.id,
    NEW.employee_id,
    NEW.full_name,
    COALESCE(NEW.nick_name, ''),
    COALESCE(NEW.email, ''),
    COALESCE(NEW.project, ''),
    COALESCE(NEW.job_title, ''),
    COALESCE(NEW.notes, ''),
    COALESCE((SELECT name FROM teams WHERE id = NEW.team_id), ''),
    COALESCE(NEW.computername, '')
  );
END;

CREATE TRIGGER IF NOT EXISTS trg_employees_ad AFTER DELETE ON employees BEGIN
  INSERT INTO employees_fts(
    employees_fts,
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
  VALUES (
    'delete',
    OLD.id,
    OLD.employee_id,
    OLD.full_name,
    COALESCE(OLD.nick_name, ''),
    COALESCE(OLD.email, ''),
    COALESCE(OLD.project, ''),
    COALESCE(OLD.job_title, ''),
    COALESCE(OLD.notes, ''),
    COALESCE((SELECT name FROM teams WHERE id = OLD.team_id), ''),
    COALESCE(OLD.computername, '')
  );
END;

CREATE TRIGGER IF NOT EXISTS trg_teams_au AFTER UPDATE OF name ON teams BEGIN
  INSERT INTO employees_fts(
    employees_fts,
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
    'delete',
    e.id,
    e.employee_id,
    e.full_name,
    COALESCE(e.nick_name, ''),
    COALESCE(e.email, ''),
    COALESCE(e.project, ''),
    COALESCE(e.job_title, ''),
    COALESCE(e.notes, ''),
    COALESCE(OLD.name, ''),
    COALESCE(e.computername, '')
  FROM employees e
  WHERE e.team_id = NEW.id;

  INSERT INTO employees_fts(rowid, employee_id, full_name, nick_name, email, project, job_title, notes, team_name, computername)
  SELECT
    e.id,
    e.employee_id,
    e.full_name,
    COALESCE(e.nick_name, ''),
    COALESCE(e.email, ''),
    COALESCE(e.project, ''),
    COALESCE(e.job_title, ''),
    COALESCE(e.notes, ''),
    COALESCE(NEW.name, ''),
    COALESCE(e.computername, '')
  FROM employees e
  WHERE e.team_id = NEW.id;
END;
"#;
