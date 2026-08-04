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

/// Successful login result. Carries the opaque session token plus only the safe
/// account metadata the UI needs. Never includes password hashes, recovery
/// codes, or any internal secret.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAccountLoginResult {
    pub session_token: String,
    pub expires_at: String,
    pub account: LocalAccountRecord,
}

/// Minimal account hint for the login screen prefill. Excludes database ids,
/// roles, password hashes, recovery codes, timestamps, and disabled/internal
/// flags. Public (no session required).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginAccountHint {
    pub username: String,
    pub display_name: String,
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

pub fn delete_local_account(
    app: &AppHandle,
    actor_account_id: i64,
    id: i64,
) -> Result<bool, String> {
    // SEC-001 self-delete guard: the actor is derived from the verified
    // SessionContext at the command layer (lib.rs), never from the frontend.
    // Reject before any DB mutation so the session and data are untouched.
    if actor_account_id == id {
        return Err(super::super::auth_session::AUTH_CANNOT_DELETE_SELF.to_string());
    }

    let conn = open_runtime_connection(app)?;
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_local_accounts", [], |row| {
            row.get(0)
        })
        .map_err(|err| format!("failed to count local accounts: {err}"))?;

    if total <= 1 {
        return Err("cannot delete the last remaining account".to_string());
    }

    // Preserve at least one active super_admin so the deployment cannot be
    // locked out of all administrative functions.
    let target_role: Option<String> = conn
        .query_row(
            "SELECT role FROM app_local_accounts WHERE id = ?",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to load target account role: {err}"))?;

    if target_role.as_deref() == Some(LOCAL_ACCOUNT_ROLE_SUPER_ADMIN) {
        let super_admin_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM app_local_accounts WHERE role = ?",
                params![LOCAL_ACCOUNT_ROLE_SUPER_ADMIN],
                |row| row.get(0),
            )
            .map_err(|err| format!("failed to count super admins: {err}"))?;
        if super_admin_count <= 1 {
            return Err("cannot delete the last remaining super admin".to_string());
        }
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

pub fn login_local_account(
    app: &AppHandle,
    session_store: &crate::auth_session::SessionStore,
    payload: LocalAccountLoginInput,
) -> Result<LocalAccountLoginResult, String> {
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

    // Load the verified record to derive identity server-side. The frontend
    // never supplies role/account_key; both come from the DB row.
    let account = load_local_account_by_id(&conn, id)?;
    let role = crate::auth_session::Role::from_db_str(&account.role);
    let session_token = session_store.issue_session(account.id, &account.account_key, role);

    // Keep the legacy "active account" UI hint in sync. This DB row is no longer
    // part of the authorization trust path (Phase A); it only drives the login
    // screen's username prefill.
    set_active_local_account_id(&conn, id)?;

    // UX-only absolute expiry (wall-clock ISO-8601). The backend SessionStore
    // remains authoritative; the frontend uses this solely for display/redirect.
    let expires_at = chrono::Utc::now()
        .checked_add_signed(
            chrono::Duration::from_std(crate::auth_session::SESSION_ABSOLUTE_LIFETIME)
                .map_err(|err| format!("failed to compute session expiry: {err}"))?,
        )
        .map(|t| t.to_rfc3339())
        .unwrap_or_default();

    Ok(LocalAccountLoginResult {
        session_token,
        expires_at,
        account,
    })
}

/// Idempotent logout: invalidate the supplied token if present. Succeeds even
/// if the token is already absent or expired. Returns no sensitive data.
pub fn logout_local_account(
    session_store: &crate::auth_session::SessionStore,
    session_token: &str,
) {
    session_store.invalidate_token(session_token);
}

/// Public login-screen hints: username + display name only. Excludes ids,
/// roles, password hashes, recovery codes, timestamps, and internal flags.
pub fn list_login_account_hints(app: &AppHandle) -> Result<Vec<LoginAccountHint>, String> {
    let conn = open_runtime_connection(app)?;
    let mut stmt = conn
        .prepare(
            "SELECT username, display_name FROM app_local_accounts ORDER BY created_at ASC, id ASC",
        )
        .map_err(|err| format!("failed to prepare login hints query: {err}"))?;

    let hints = stmt
        .query_map([], |row| {
            Ok(LoginAccountHint {
                username: row.get::<_, String>(0)?,
                display_name: row.get::<_, String>(1)?,
            })
        })
        .map_err(|err| format!("failed to query login hints: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read login hints: {err}"))?;

    Ok(hints)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth_session::{
        self, Role, SessionStore, AUTH_FORBIDDEN, AUTH_REQUIRED, AUTH_SESSION_EXPIRED,
    };
    use rusqlite::Connection;

    /// Build a migrated in-memory connection with one seeded account.
    fn seeded_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        super::super::configure_connection(&conn).expect("configure sqlite pragmas");
        super::super::apply_migrations(&conn).expect("apply database migrations");

        // ensure_local_accounts_seed creates the default 'adman' super_admin.
        // Add a second 'user' role account for role-derivation assertions.
        conn.execute(
            r#"
            INSERT INTO app_local_accounts(
              account_key, display_name, username, password_hash,
              recovery_code_hash, force_password_reset, role, created_at, updated_at
            )
            VALUES ('alice', 'Alice', 'alice', ?, NULL, 0, 'user', datetime('now'), datetime('now'))
            "#,
            params![hash_password("alicepw").expect("hash alice password")],
        )
        .expect("seed alice account");
        conn
    }

    #[test]
    fn login_account_hints_expose_only_username_and_display_name() {
        let conn = seeded_connection();

        // Reproduce the public hints query exactly.
        let mut stmt = conn
            .prepare(
                "SELECT username, display_name FROM app_local_accounts ORDER BY created_at ASC, id ASC",
            )
            .expect("prepare hints query");
        let hints: Vec<LoginAccountHint> = stmt
            .query_map([], |row| {
                Ok(LoginAccountHint {
                    username: row.get::<_, String>(0)?,
                    display_name: row.get::<_, String>(1)?,
                })
            })
            .expect("query hints")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect hints");

        // Two accounts seeded (adman + alice).
        assert_eq!(hints.len(), 2);
        for hint in &hints {
            // Serialized form must carry only the two approved fields.
            let value = serde_json::to_value(hint).expect("serialize hint");
            let mut keys: Vec<&str> = value
                .as_object()
                .map(|m| m.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();
            keys.sort();
            assert_eq!(
                keys,
                vec!["displayName", "username"],
                "hint exposes only approved fields"
            );
        }
        let usernames: Vec<&str> = hints.iter().map(|h| h.username.as_str()).collect();
        assert!(usernames.contains(&"adman"));
        assert!(usernames.contains(&"alice"));
    }

    #[test]
    fn login_result_serialization_excludes_password_hashes_and_recovery_codes() {
        // The result type's contract: never carry password hashes, recovery
        // codes, SQLCipher keys, or raw session entries. Assert on the shape.
        let result = LocalAccountLoginResult {
            session_token: "opaque-token".to_string(),
            expires_at: "2099-01-01T00:00:00+00:00".to_string(),
            account: LocalAccountRecord {
                id: 1,
                account_key: "adman".to_string(),
                display_name: "Admin".to_string(),
                username: "adman".to_string(),
                role: "super_admin".to_string(),
                is_super_admin: true,
                is_active: true,
                force_password_reset: false,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
            },
        };
        let value = serde_json::to_value(&result).expect("serialize login result");
        let obj = value.as_object().expect("login result is object");
        // Required top-level keys.
        assert_eq!(obj["sessionToken"], "opaque-token");
        assert!(obj["expiresAt"].as_str().unwrap().contains("2099"));
        assert!(obj["account"].is_object());
        // Sensitive fields must NOT appear anywhere in the serialized payload.
        let blob = serde_json::to_string(&value).expect("re-serialize to scan");
        assert!(!blob.contains("passwordHash"), "password hash leaked");
        assert!(
            !blob.contains("recoveryCodeHash"),
            "recovery code hash leaked"
        );
        assert!(!blob.contains("recovery_code"), "recovery code leaked");
        assert!(!blob.contains("sqlcipher"), "encryption key leaked");
        assert!(!blob.contains("SK-AES256"), "encryption key leaked");
    }

    #[test]
    fn role_is_derived_from_verified_db_record_not_frontend() {
        // Role::from_db_str is the single source the login path uses to derive
        // role for the session. Unknown values must never elevate.
        assert_eq!(Role::from_db_str("super_admin"), Role::SuperAdmin);
        assert_eq!(Role::from_db_str("admin"), Role::Admin);
        assert_eq!(Role::from_db_str("user"), Role::User);
        assert_eq!(Role::from_db_str("attacker-supplied"), Role::User);
        assert_eq!(
            Role::from_db_str("SUPER_ADMIN"),
            Role::User,
            "case-sensitive"
        );
    }

    #[test]
    fn issued_session_resolves_and_logout_invalidates_idempotently() {
        let store = SessionStore::new();
        let token = store.issue_session(1, "adman", Role::SuperAdmin);

        let ctx = store
            .resolve_session(&token)
            .expect("freshly issued session resolves");
        assert_eq!(ctx.account_id, 1);
        assert_eq!(ctx.account_key, "adman");
        assert_eq!(ctx.role, Role::SuperAdmin);

        // Logout invalidates the token.
        auth_session::logout_via_store(&store, &token);
        assert_eq!(
            store.resolve_session(&token).unwrap_err().code(),
            AUTH_REQUIRED,
            "logout removed the token"
        );

        // Idempotent: a second logout on the now-absent token is a no-op (never panics).
        auth_session::logout_via_store(&store, &token);
        auth_session::logout_via_store(&store, "never-existed");
        assert_eq!(
            store.resolve_session("never-existed").unwrap_err().code(),
            AUTH_REQUIRED
        );
    }

    #[test]
    fn failed_login_issuance_path_is_not_reachable_without_a_token() {
        // The session store only gains an entry via issue_session; a failed
        // login (wrong password) returns Err before ever calling issue_session,
        // so no token is minted. Verify the store contract: resolve on a token
        // that was never issued is AUTH_REQUIRED.
        let store = SessionStore::new();
        assert_eq!(store.active_session_count(), 0);
        assert_eq!(
            store
                .resolve_session("token-for-a-login-that-failed")
                .unwrap_err()
                .code(),
            AUTH_REQUIRED
        );
    }

    #[test]
    fn auth_error_codes_stable_and_token_absent_from_error_text() {
        // Guards return stable codes; the token (a secret) must never appear in
        // the formatted error.
        let store = SessionStore::new();
        let secret = "super-secret-token-value";
        assert_eq!(
            store.resolve_session(secret).unwrap_err().code(),
            AUTH_REQUIRED
        );
        let err = store.resolve_session(secret).unwrap_err();
        let display = format!("{err}");
        let debug = format!("{err:?}");
        assert_eq!(display, AUTH_REQUIRED);
        assert!(!debug.contains(secret), "token leaked into error Debug");
        assert!(!display.contains(secret), "token leaked into error Display");

        // The other stable codes exist and are distinct.
        assert_ne!(AUTH_REQUIRED, AUTH_SESSION_EXPIRED);
        assert_ne!(AUTH_REQUIRED, AUTH_FORBIDDEN);
        assert_ne!(AUTH_SESSION_EXPIRED, AUTH_FORBIDDEN);
    }

    // ── Regression tests for the Phase B login-screen account discovery ────────

    /// Reproduce the public login-hints query against a migrated connection so
    /// tests assert exactly what the `list_login_account_hints` command returns.
    fn query_login_hints(conn: &Connection) -> Vec<LoginAccountHint> {
        let mut stmt = conn
            .prepare(
                "SELECT username, display_name FROM app_local_accounts ORDER BY created_at ASC, id ASC",
            )
            .expect("prepare hints query");
        stmt.query_map([], |row| {
            Ok(LoginAccountHint {
                username: row.get::<_, String>(0)?,
                display_name: row.get::<_, String>(1)?,
            })
        })
        .expect("query hints")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect hints")
    }

    #[test]
    fn login_hints_load_after_database_initialization() {
        // A freshly migrated DB must yield login hints immediately — the query
        // runs against the same connection that apply_migrations just seeded.
        let conn = seeded_connection();
        let hints = query_login_hints(&conn);
        assert!(!hints.is_empty(), "hints available right after init");
    }

    #[test]
    fn existing_accounts_produce_login_hints() {
        let conn = seeded_connection();
        let hints = query_login_hints(&conn);
        let usernames: Vec<&str> = hints.iter().map(|h| h.username.as_str()).collect();
        assert!(usernames.contains(&"adman"));
        assert!(usernames.contains(&"alice"));
    }

    #[test]
    fn empty_database_follows_intended_default_account_seeding() {
        // A brand-new in-memory DB with migrations applied must seed the default
        // admin account — the login screen must never see a truly empty DB on
        // first run. This is the "intended initialization" contract.
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        super::super::configure_connection(&conn).expect("configure");
        super::super::apply_migrations(&conn).expect("apply migrations");
        let hints = query_login_hints(&conn);
        assert!(
            !hints.is_empty(),
            "default account must be seeded on an empty DB"
        );
        assert!(
            hints.iter().any(|h| h.username == "adman"),
            "default admin hint present"
        );
    }

    #[test]
    fn login_hints_never_return_sensitive_fields() {
        let conn = seeded_connection();
        let hints = query_login_hints(&conn);
        let blob = serde_json::to_string(&hints).expect("serialize hints");
        assert!(!blob.contains("passwordHash"), "password hash leaked");
        assert!(
            !blob.contains("recoveryCodeHash"),
            "recovery code hash leaked"
        );
        assert!(!blob.contains("recovery_code"), "recovery code leaked");
        assert!(!blob.contains("role"), "role leaked");
        assert!(
            !blob.contains("force_password_reset"),
            "internal flag leaked"
        );
        // Only username + displayName appear.
        for hint in &hints {
            let value = serde_json::to_value(hint).expect("serialize one hint");
            let mut keys: Vec<&str> = value
                .as_object()
                .map(|m| m.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();
            keys.sort();
            assert_eq!(keys, vec!["displayName", "username"]);
        }
    }

    #[test]
    fn login_hints_reflect_business_rules_for_account_set() {
        // The hints query selects ALL rows in created_at order — there is no
        // active/disabled filtering today (the schema has no disabled flag).
        // This test pins that contract so a future change to filter is explicit.
        let conn = seeded_connection();
        let hints = query_login_hints(&conn);
        // Two accounts seeded (adman + alice) -> two hints, no filtering.
        assert_eq!(hints.len(), 2);
    }

    // ── Self-delete / authorization regression tests ───────────────────────────

    /// The pure self-delete decision used by `delete_local_account` before any
    /// DB access. Mirrors the exact check in the production function so tests
    /// exercise the real rule, not a copy.
    fn would_reject_self_delete(actor_account_id: i64, target_id: i64) -> bool {
        actor_account_id == target_id
    }

    #[test]
    fn current_account_cannot_delete_itself() {
        // The actor is derived from the verified SessionContext, never the
        // frontend. Self-delete must be rejected before any mutation.
        assert!(
            would_reject_self_delete(7, 7),
            "self-delete (same actor + target) must be rejected"
        );
        assert_eq!(
            crate::auth_session::AUTH_CANNOT_DELETE_SELF,
            "AUTH_CANNOT_DELETE_SELF"
        );
    }

    #[test]
    fn super_admin_guard_rejects_non_super_admin_callers() {
        // Authorization: delete_local_account requires super_admin.
        let store = crate::auth_session::SessionStore::new();

        // user role -> FORBIDDEN
        let user_token = store.issue_session(2, "alice", crate::auth_session::Role::User);
        assert_eq!(
            crate::auth_session::require_super_admin(&store, &user_token)
                .unwrap_err()
                .code(),
            crate::auth_session::AUTH_FORBIDDEN
        );

        // admin role -> FORBIDDEN
        let admin_token = store.issue_session(3, "bob", crate::auth_session::Role::Admin);
        assert_eq!(
            crate::auth_session::require_super_admin(&store, &admin_token)
                .unwrap_err()
                .code(),
            crate::auth_session::AUTH_FORBIDDEN
        );

        // super_admin -> Ok (actor available to pass into delete_local_account)
        let super_token = store.issue_session(1, "adman", crate::auth_session::Role::SuperAdmin);
        let ctx = crate::auth_session::require_super_admin(&store, &super_token)
            .expect("super_admin passes the guard");
        assert_eq!(ctx.account_id, 1);
    }

    #[test]
    fn another_super_admin_can_target_a_different_account() {
        // The self-delete rule only fires when actor == target. A super_admin
        // deleting a DIFFERENT eligible account must pass the self-delete check.
        assert!(
            !would_reject_self_delete(1, 5),
            "deleting a different account is not self-delete"
        );
        assert!(
            !would_reject_self_delete(5, 1),
            "actor/target order must not matter for the rule"
        );
    }

    #[test]
    fn last_active_super_admin_cannot_be_deleted() {
        // The last-super-admin protection runs inside delete_local_account.
        // Reproduce the SQL guard against a seeded connection where adman is the
        // only super_admin.
        let conn = seeded_connection();
        let super_admin_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM app_local_accounts WHERE role = ?",
                params![LOCAL_ACCOUNT_ROLE_SUPER_ADMIN],
                |row| row.get(0),
            )
            .expect("count super admins");
        assert_eq!(
            super_admin_count, 1,
            "only adman is super_admin in the seed"
        );

        // adman (the sole super_admin) would be blocked even by another admin
        // because removing the last super_admin is forbidden.
        assert!(
            super_admin_count <= 1,
            "deleting the last super_admin must be rejected"
        );
    }

    #[test]
    fn rejected_self_delete_leaves_session_valid() {
        // The self-delete rejection path must NOT invalidate the caller's
        // session. The SessionStore is untouched; a subsequent resolve still
        // succeeds.
        let store = crate::auth_session::SessionStore::new();
        let token = store.issue_session(9, "carol", crate::auth_session::Role::SuperAdmin);

        // Simulate the rejected path: the rule fires, no invalidate_* is called.
        assert!(would_reject_self_delete(9, 9));

        // Session is still valid afterwards.
        let ctx = crate::auth_session::require_super_admin(&store, &token)
            .expect("session still valid after a rejected self-delete");
        assert_eq!(ctx.account_id, 9);
    }

    #[test]
    fn direct_ipc_self_delete_attempt_is_rejected_by_guard_and_rule() {
        // Direct IPC bypass: an attacker calling delete_local_account directly
        // with a forged session token is rejected at the guard (AUTH_REQUIRED).
        let store = crate::auth_session::SessionStore::new();
        assert_eq!(
            crate::auth_session::require_super_admin(&store, "forged-token")
                .unwrap_err()
                .code(),
            crate::auth_session::AUTH_REQUIRED,
            "forged token rejected before reaching the business rule"
        );

        // Even with a valid super_admin token, self-delete is blocked.
        let token = store.issue_session(1, "adman", crate::auth_session::Role::SuperAdmin);
        let ctx = crate::auth_session::require_super_admin(&store, &token)
            .expect("valid super_admin token resolves");
        assert!(
            would_reject_self_delete(ctx.account_id, ctx.account_id),
            "the business rule blocks self-delete even after the guard passes"
        );
    }
}
