use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::schema::{
    ACTIVE_LOCAL_ACCOUNT_SETTING_KEY, DEFAULT_ADMIN_SEED_SETTING_KEY, DEFAULT_LOCAL_ACCOUNT_KEY,
    DEFAULT_LOCAL_ACCOUNT_NAME, DEFAULT_LOCAL_ACCOUNT_PASSWORD,
    DEFAULT_LOCAL_ACCOUNT_RECOVERY_CODE, DEFAULT_LOCAL_ACCOUNT_USERNAME,
    DEFAULT_NEW_LOCAL_ACCOUNT_PASSWORD, LOCAL_ACCOUNT_ROLE_ADMIN, LOCAL_ACCOUNT_ROLE_SUPER_ADMIN,
    LOCAL_ACCOUNT_ROLE_USER,
};
use super::{
    humanize_sqlite_error, normalize_dynamic_key, normalize_optional_text, open_runtime_connection,
    require_text,
};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAccountRecord {
    pub id: i64,
    pub account_key: String,
    pub display_name: String,
    pub username: String,
    pub role: String,
    pub is_super_admin: bool,
    pub is_active: bool,
    pub force_password_reset: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAccountCreateInput {
    pub display_name: String,
    pub username: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAccountUpdateInput {
    pub id: i64,
    pub display_name: Option<String>,
    pub username: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAccountLoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPasswordChangeInput {
    pub id: i64,
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPasswordResetInput {
    pub id: i64,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalForgotPasswordInput {
    pub username: String,
    pub recovery_code: String,
    pub new_password: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn list_local_accounts(app: &AppHandle) -> Result<Vec<LocalAccountRecord>, String> {
    let conn = open_runtime_connection(app)?;
    query_local_accounts(&conn)
}

pub fn create_local_account(
    app: &AppHandle,
    payload: LocalAccountCreateInput,
) -> Result<LocalAccountRecord, String> {
    let conn = open_runtime_connection(app)?;
    let display_name = require_text(payload.display_name, "displayName")?;
    let role = normalize_local_account_role(payload.role);

    let username = if let Some(raw_username) = payload.username {
        require_local_account_username(raw_username)?
    } else {
        generate_local_account_username(&conn, display_name.as_str())?
    };

    if local_account_username_exists(&conn, username.as_str())? {
        return Err("username already exists".to_string());
    }

    let account_key = generate_local_account_key(&conn, display_name.as_str())?;
    let password_hash = hash_password(DEFAULT_NEW_LOCAL_ACCOUNT_PASSWORD)?;
    let recovery_code_hash = hash_password(DEFAULT_LOCAL_ACCOUNT_RECOVERY_CODE)?;

    conn.execute(
        r#"
        INSERT INTO app_local_accounts(
          account_key,
          display_name,
          username,
          password_hash,
          recovery_code_hash,
          force_password_reset,
          role,
          created_at,
          updated_at
        )
        VALUES(?, ?, ?, ?, ?, 1, ?, datetime('now'), datetime('now'))
        "#,
        params![
            account_key.as_str(),
            display_name.as_str(),
            username.as_str(),
            password_hash.as_str(),
            recovery_code_hash.as_str(),
            role.as_str()
        ],
    )
    .map_err(humanize_sqlite_error)?;

    let inserted_id = conn.last_insert_rowid();
    load_local_account_by_id(&conn, inserted_id)
}

pub fn update_local_account(
    app: &AppHandle,
    payload: LocalAccountUpdateInput,
) -> Result<LocalAccountRecord, String> {
    let conn = open_runtime_connection(app)?;
    let existing = load_local_account_by_id(&conn, payload.id)?;

    let display_name = if let Some(raw) = payload.display_name {
        require_text(raw, "displayName")?
    } else {
        existing.display_name.clone()
    };

    let role = normalize_local_account_role(payload.role);

    let username = if let Some(raw_username) = payload.username {
        let new_username = require_local_account_username(raw_username)?;
        if local_account_username_exists_excluding(&conn, new_username.as_str(), payload.id)? {
            return Err("username already taken by another account".to_string());
        }
        new_username
    } else {
        existing.username.clone()
    };

    conn.execute(
        r#"
        UPDATE app_local_accounts
        SET display_name = ?, username = ?, role = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
        params![
            display_name.as_str(),
            username.as_str(),
            role.as_str(),
            payload.id
        ],
    )
    .map_err(humanize_sqlite_error)?;

    load_local_account_by_id(&conn, payload.id)
}

pub fn delete_local_account(app: &AppHandle, id: i64) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_local_accounts", [], |row| {
            row.get(0)
        })
        .map_err(|err| format!("failed to count local accounts: {err}"))?;

    if total <= 1 {
        return Err("cannot delete the last remaining account".to_string());
    }

    let active_id = get_active_local_account_id(&conn)?;
    let changed = conn
        .execute("DELETE FROM app_local_accounts WHERE id = ?", params![id])
        .map_err(humanize_sqlite_error)?;

    if changed > 0 && active_id == Some(id) {
        let fallback_id: i64 = conn
            .query_row(
                "SELECT id FROM app_local_accounts ORDER BY created_at ASC, id ASC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|err| format!("failed to resolve fallback account after deletion: {err}"))?;
        set_active_local_account_id(&conn, fallback_id)?;
    }

    Ok(changed > 0)
}

pub fn set_active_local_account(app: &AppHandle, id: i64) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM app_local_accounts WHERE id = ?",
            params![id],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to verify account: {err}"))?;

    if exists == 0 {
        return Err(format!("local account with id {id} was not found"));
    }

    set_active_local_account_id(&conn, id)?;
    Ok(true)
}

pub fn login_local_account(
    app: &AppHandle,
    payload: LocalAccountLoginInput,
) -> Result<LocalAccountRecord, String> {
    let conn = open_runtime_connection(app)?;
    let username_normalized = normalize_local_account_username(payload.username)
        .ok_or_else(|| "invalid username".to_string())?;

    let maybe = conn
        .query_row(
            "SELECT id, password_hash FROM app_local_accounts WHERE username = ? COLLATE NOCASE",
            params![username_normalized.as_str()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|err| format!("failed to query account for login: {err}"))?;

    let Some((id, password_hash)) = maybe else {
        return Err("incorrect username or password".to_string());
    };

    if !verify_password(payload.password.as_str(), password_hash.as_str())? {
        return Err("incorrect username or password".to_string());
    }

    set_active_local_account_id(&conn, id)?;
    load_local_account_by_id(&conn, id)
}

pub fn change_local_account_password(
    app: &AppHandle,
    payload: LocalPasswordChangeInput,
) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    let new_password = require_local_account_password(payload.new_password)?;

    let maybe = conn
        .query_row(
            "SELECT password_hash FROM app_local_accounts WHERE id = ?",
            params![payload.id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("failed to load password for change: {err}"))?;

    let Some(stored_hash) = maybe else {
        return Err(format!(
            "local account with id {} was not found",
            payload.id
        ));
    };

    if !verify_password(payload.current_password.as_str(), stored_hash.as_str())? {
        return Err("current password is incorrect".to_string());
    }

    let new_hash = hash_password(new_password.as_str())?;
    conn.execute(
        "UPDATE app_local_accounts SET password_hash = ?, force_password_reset = 0, updated_at = datetime('now') WHERE id = ?",
        params![new_hash.as_str(), payload.id],
    )
    .map_err(humanize_sqlite_error)?;

    Ok(true)
}

pub fn admin_reset_local_account_password(
    app: &AppHandle,
    payload: LocalPasswordResetInput,
) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    let new_password = require_local_account_password(payload.new_password)?;
    let new_hash = hash_password(new_password.as_str())?;

    let changed = conn
        .execute(
            "UPDATE app_local_accounts SET password_hash = ?, force_password_reset = 0, updated_at = datetime('now') WHERE id = ?",
            params![new_hash.as_str(), payload.id],
        )
        .map_err(humanize_sqlite_error)?;

    if changed == 0 {
        return Err(format!(
            "local account with id {} was not found",
            payload.id
        ));
    }

    Ok(true)
}

pub fn forgot_local_account_password(
    app: &AppHandle,
    payload: LocalForgotPasswordInput,
) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    let new_password = require_local_account_password(payload.new_password)?;
    let recovery_code = normalize_recovery_code_input(Some(payload.recovery_code))
        .ok_or_else(|| "recovery code is required".to_string())?;

    let username_normalized = normalize_local_account_username(payload.username)
        .ok_or_else(|| "invalid username".to_string())?;

    let maybe = conn
        .query_row(
            "SELECT id, recovery_code_hash FROM app_local_accounts WHERE username = ? COLLATE NOCASE",
            params![username_normalized.as_str()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(|err| format!("failed to query account for recovery: {err}"))?;

    let Some((id, stored_code_hash)) = maybe else {
        return Err("incorrect username or recovery code".to_string());
    };

    let Some(stored_hash) = stored_code_hash else {
        return Err("no recovery code set for this account".to_string());
    };

    if !verify_password(recovery_code.as_str(), stored_hash.as_str())? {
        return Err("incorrect username or recovery code".to_string());
    }

    let new_hash = hash_password(new_password.as_str())?;
    conn.execute(
        "UPDATE app_local_accounts SET password_hash = ?, force_password_reset = 0, updated_at = datetime('now') WHERE id = ?",
        params![new_hash.as_str(), id],
    )
    .map_err(humanize_sqlite_error)?;

    set_active_local_account_id(&conn, id)?;
    Ok(true)
}

// ── pub(super) helpers called by mod.rs during migrations ─────────────────────

pub(super) fn ensure_local_accounts_seed(conn: &Connection) -> Result<(), String> {
    let total_accounts: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_local_accounts", [], |row| {
            row.get(0)
        })
        .map_err(|err| format!("failed to count local accounts: {err}"))?;

    if total_accounts <= 0 {
        let password_hash = hash_password(DEFAULT_LOCAL_ACCOUNT_PASSWORD)?;
        let recovery_code_hash = hash_password(DEFAULT_LOCAL_ACCOUNT_RECOVERY_CODE)?;
        conn.execute(
            r#"
            INSERT INTO app_local_accounts(
              account_key,
              display_name,
              username,
              password_hash,
              recovery_code_hash,
              force_password_reset,
              role,
              created_at,
              updated_at
            )
            VALUES(?, ?, ?, ?, ?, 1, ?, datetime('now'), datetime('now'))
            "#,
            params![
                DEFAULT_LOCAL_ACCOUNT_KEY,
                DEFAULT_LOCAL_ACCOUNT_NAME,
                DEFAULT_LOCAL_ACCOUNT_USERNAME,
                password_hash,
                recovery_code_hash,
                LOCAL_ACCOUNT_ROLE_ADMIN
            ],
        )
        .map_err(humanize_sqlite_error)?;
    } else {
        backfill_local_account_auth_fields(conn)?;
    }
    ensure_default_admin_account(conn)?;

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

// ── Private helpers ───────────────────────────────────────────────────────────

fn query_local_accounts(conn: &Connection) -> Result<Vec<LocalAccountRecord>, String> {
    let mut active_id = get_active_local_account_id(conn)?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              id,
              account_key,
              display_name,
              username,
              role,
              force_password_reset,
              created_at,
              updated_at
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
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .map_err(|err| format!("failed to query local accounts: {err}"))?;

    let mut items = Vec::new();
    for row in rows {
        let (
            id,
            account_key,
            display_name,
            username,
            role_raw,
            force_password_reset,
            created_at,
            updated_at,
        ) = row.map_err(|err| format!("failed to read local account row: {err}"))?;
        let role = parse_account_role(&role_raw);

        items.push(LocalAccountRecord {
            id,
            account_key,
            display_name,
            username,
            is_super_admin: role == LOCAL_ACCOUNT_ROLE_SUPER_ADMIN,
            role,
            is_active: false,
            force_password_reset: force_password_reset > 0,
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
        SELECT id, account_key, display_name, username, role, force_password_reset, created_at, updated_at
        FROM app_local_accounts
        WHERE id = ?
        "#,
        params![id],
        |row| {
            let role_raw: String = row.get(4)?;
            let role = parse_account_role(&role_raw);
            Ok(LocalAccountRecord {
                id: row.get(0)?,
                account_key: row.get(1)?,
                display_name: row.get(2)?,
                username: row.get(3)?,
                is_super_admin: role == LOCAL_ACCOUNT_ROLE_SUPER_ADMIN,
                role,
                is_active: active_id == Some(id),
                force_password_reset: row.get::<_, i64>(5)? > 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to load local account: {err}"))?
    .ok_or_else(|| format!("local account with id {id} was not found"))
}

pub(crate) fn get_active_local_account_id(conn: &Connection) -> Result<Option<i64>, String> {
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

fn normalize_local_account_role(value: Option<String>) -> String {
    let normalized =
        normalize_optional_text(value).unwrap_or_else(|| LOCAL_ACCOUNT_ROLE_USER.to_string());
    if normalized.eq_ignore_ascii_case(LOCAL_ACCOUNT_ROLE_SUPER_ADMIN) {
        LOCAL_ACCOUNT_ROLE_SUPER_ADMIN.to_string()
    } else if normalized.eq_ignore_ascii_case(LOCAL_ACCOUNT_ROLE_ADMIN) {
        LOCAL_ACCOUNT_ROLE_ADMIN.to_string()
    } else {
        LOCAL_ACCOUNT_ROLE_USER.to_string()
    }
}

fn parse_account_role(raw: &str) -> String {
    if raw.eq_ignore_ascii_case(LOCAL_ACCOUNT_ROLE_SUPER_ADMIN) {
        LOCAL_ACCOUNT_ROLE_SUPER_ADMIN.to_string()
    } else if raw.eq_ignore_ascii_case(LOCAL_ACCOUNT_ROLE_ADMIN) {
        LOCAL_ACCOUNT_ROLE_ADMIN.to_string()
    } else {
        LOCAL_ACCOUNT_ROLE_USER.to_string()
    }
}

fn normalize_local_account_username(value: String) -> Option<String> {
    let candidate = value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-' || *ch == '.')
        .collect::<String>();

    if candidate.len() < 3 || candidate.len() > 48 {
        return None;
    }

    Some(candidate)
}

fn require_local_account_username(value: String) -> Result<String, String> {
    normalize_local_account_username(value).ok_or_else(|| {
        "username must be 3-48 chars and contain only letters, numbers, ., _, -".to_string()
    })
}

fn require_local_account_password(value: String) -> Result<String, String> {
    let password = value.trim().to_string();
    if password.len() < 6 {
        return Err("password must be at least 6 characters".to_string());
    }
    Ok(password)
}

fn normalize_recovery_code_input(value: Option<String>) -> Option<String> {
    value
        .and_then(|item| normalize_optional_text(Some(item)))
        .map(|item| item.to_uppercase())
}

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| format!("failed to hash password: {err}"))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, password_hash: &str) -> Result<bool, String> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|err| format!("stored password hash is invalid: {err}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

fn local_account_username_exists(conn: &Connection, username: &str) -> Result<bool, String> {
    let exists: i64 = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM app_local_accounts WHERE username = ? COLLATE NOCASE)",
            params![username],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to check username existence: {err}"))?;
    Ok(exists > 0)
}

fn local_account_username_exists_excluding(
    conn: &Connection,
    username: &str,
    excluded_id: i64,
) -> Result<bool, String> {
    let exists: i64 = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM app_local_accounts WHERE username = ? COLLATE NOCASE AND id <> ?)",
            params![username, excluded_id],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to check username existence: {err}"))?;
    Ok(exists > 0)
}

fn generate_local_account_username(conn: &Connection, seed: &str) -> Result<String, String> {
    let base_normalized = normalize_local_account_username(seed.to_string())
        .or_else(|| Some(normalize_dynamic_key(seed)))
        .unwrap_or_else(|| "user".to_string());
    let base = if base_normalized.is_empty() {
        "user".to_string()
    } else {
        base_normalized
    };

    if !local_account_username_exists(conn, base.as_str())? {
        return Ok(base);
    }

    for index in 2..=9999 {
        let candidate = format!("{base}_{index}");
        if !local_account_username_exists(conn, candidate.as_str())? {
            return Ok(candidate);
        }
    }

    Err("failed to allocate local account username".to_string())
}

fn backfill_local_account_auth_fields(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, display_name, account_key, username, password_hash FROM app_local_accounts ORDER BY id ASC",
        )
        .map_err(|err| format!("failed to prepare account auth backfill query: {err}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|err| format!("failed to query account auth backfill rows: {err}"))?;

    for row in rows {
        let (id, display_name, account_key, username, password_hash) =
            row.map_err(|err| format!("failed to read account auth backfill row: {err}"))?;

        let next_username = username
            .and_then(|value| normalize_local_account_username(value))
            .unwrap_or_else(|| {
                generate_local_account_username(conn, account_key.as_str())
                    .unwrap_or_else(|_| normalize_dynamic_key(display_name.as_str()))
            });

        let needs_password = password_hash
            .as_ref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true);

        if needs_password {
            let seeded_hash = hash_password(DEFAULT_NEW_LOCAL_ACCOUNT_PASSWORD)?;
            conn.execute(
                r#"
                UPDATE app_local_accounts
                SET username = ?, password_hash = ?, force_password_reset = 1, updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![next_username.as_str(), seeded_hash.as_str(), id],
            )
            .map_err(humanize_sqlite_error)?;
        } else {
            conn.execute(
                r#"
                UPDATE app_local_accounts
                SET username = ?, updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![next_username.as_str(), id],
            )
            .map_err(humanize_sqlite_error)?;
        }
    }

    Ok(())
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
    let base = if base.is_empty() {
        "user".to_string()
    } else {
        base
    };

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

fn ensure_default_admin_account(conn: &Connection) -> Result<(), String> {
    let default_admin_seeded = is_default_admin_seeded(conn)?;
    let existing = conn
        .query_row(
            "SELECT id FROM app_local_accounts WHERE username = ? COLLATE NOCASE",
            params![DEFAULT_LOCAL_ACCOUNT_USERNAME],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|err| format!("failed to lookup default admin account: {err}"))?;

    match existing {
        Some(id) => {
            if default_admin_seeded {
                conn.execute(
                    "UPDATE app_local_accounts SET role = ?, updated_at = datetime('now') WHERE id = ?",
                    params![LOCAL_ACCOUNT_ROLE_ADMIN, id],
                )
                .map_err(humanize_sqlite_error)?;
            } else {
                let password_hash = hash_password(DEFAULT_LOCAL_ACCOUNT_PASSWORD)?;
                let recovery_code_hash = hash_password(DEFAULT_LOCAL_ACCOUNT_RECOVERY_CODE)?;
                conn.execute(
                    r#"
                    UPDATE app_local_accounts
                    SET
                      display_name = ?,
                      password_hash = ?,
                      recovery_code_hash = ?,
                      force_password_reset = 1,
                      role = ?,
                      updated_at = datetime('now')
                    WHERE id = ?
                    "#,
                    params![
                        DEFAULT_LOCAL_ACCOUNT_NAME,
                        password_hash.as_str(),
                        recovery_code_hash.as_str(),
                        LOCAL_ACCOUNT_ROLE_ADMIN,
                        id
                    ],
                )
                .map_err(humanize_sqlite_error)?;
            }
        }
        None => {
            let password_hash = hash_password(DEFAULT_LOCAL_ACCOUNT_PASSWORD)?;
            let recovery_code_hash = hash_password(DEFAULT_LOCAL_ACCOUNT_RECOVERY_CODE)?;
            let account_key = if local_account_key_exists(conn, DEFAULT_LOCAL_ACCOUNT_KEY)? {
                generate_local_account_key(conn, DEFAULT_LOCAL_ACCOUNT_NAME)?
            } else {
                DEFAULT_LOCAL_ACCOUNT_KEY.to_string()
            };

            conn.execute(
                r#"
                INSERT INTO app_local_accounts(
                  account_key,
                  display_name,
                  username,
                  password_hash,
                  recovery_code_hash,
                  force_password_reset,
                  role,
                  created_at,
                  updated_at
                )
                VALUES(?, ?, ?, ?, ?, 1, ?, datetime('now'), datetime('now'))
                "#,
                params![
                    account_key.as_str(),
                    DEFAULT_LOCAL_ACCOUNT_NAME,
                    DEFAULT_LOCAL_ACCOUNT_USERNAME,
                    password_hash.as_str(),
                    recovery_code_hash.as_str(),
                    LOCAL_ACCOUNT_ROLE_ADMIN
                ],
            )
            .map_err(humanize_sqlite_error)?;
        }
    }

    // Always ensure the default admin 'adman' is super_admin, even on migration from older versions
    conn.execute(
        "UPDATE app_local_accounts SET role = ?, updated_at = datetime('now') WHERE username = ? COLLATE NOCASE AND role != ?",
        params![LOCAL_ACCOUNT_ROLE_SUPER_ADMIN, DEFAULT_LOCAL_ACCOUNT_USERNAME, LOCAL_ACCOUNT_ROLE_SUPER_ADMIN],
    )
    .map_err(humanize_sqlite_error)?;

    mark_default_admin_seeded(conn)?;
    Ok(())
}

fn is_default_admin_seeded(conn: &Connection) -> Result<bool, String> {
    let value: Option<String> = conn
        .query_row(
            "SELECT setting_value FROM app_settings WHERE setting_key = ?",
            params![DEFAULT_ADMIN_SEED_SETTING_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to read default admin seed flag: {err}"))?;

    Ok(value.map(|item| item.trim() == "1").unwrap_or(false))
}

fn mark_default_admin_seeded(conn: &Connection) -> Result<(), String> {
    conn.execute(
        r#"
        INSERT INTO app_settings(setting_key, setting_value, updated_at)
        VALUES(?, '1', datetime('now'))
        ON CONFLICT(setting_key) DO UPDATE SET
          setting_value = excluded.setting_value,
          updated_at = datetime('now')
        "#,
        params![DEFAULT_ADMIN_SEED_SETTING_KEY],
    )
    .map_err(humanize_sqlite_error)?;
    Ok(())
}
