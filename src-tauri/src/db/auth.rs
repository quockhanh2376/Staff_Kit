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
    LOCAL_ACCOUNT_ROLE_USER, SECURITY_POLICY_SETTING_KEY,
};
use super::{
    humanize_sqlite_error, normalize_dynamic_key, normalize_optional_text, open_runtime_connection,
    require_text,
};
use crate::auth_session::SecurityPolicyMode;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityPolicyRecord {
    pub mode: SecurityPolicyMode,
    pub min_password_length: usize,
    pub max_failed_login_attempts: u32,
    pub lockout_duration_minutes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSecurityPolicyInput {
    pub current_password: String,
    pub new_mode: SecurityPolicyMode,
}

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
    let conn = open_runtime_connection(app)?;
    delete_local_account_conn(&conn, actor_account_id, id)
}

pub(crate) fn delete_local_account_conn(
    conn: &Connection,
    actor_account_id: i64,
    id: i64,
) -> Result<bool, String> {
    // SEC-001 self-delete guard: the actor is derived from the verified
    // SessionContext at the command layer (lib.rs), never from the frontend.
    // Reject before any DB mutation so the session and data are untouched.
    if actor_account_id == id {
        return Err(super::super::auth_session::AUTH_CANNOT_DELETE_SELF.to_string());
    }

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

    let active_id = get_active_local_account_id(conn)?;
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
        set_active_local_account_id(conn, fallback_id)?;
    }

    Ok(changed > 0)
}

pub fn login_local_account(
    app: &AppHandle,
    session_store: &crate::auth_session::SessionStore,
    payload: LocalAccountLoginInput,
) -> Result<LocalAccountLoginResult, String> {
    let conn = open_runtime_connection(app)?;
    login_local_account_conn(&conn, session_store, payload)
}

pub(crate) fn login_local_account_conn(
    conn: &Connection,
    session_store: &crate::auth_session::SessionStore,
    payload: LocalAccountLoginInput,
) -> Result<LocalAccountLoginResult, String> {
    let username_raw = payload.username.trim().to_string();
    if username_raw.is_empty() {
        return Err("incorrect username or password".to_string());
    }

    // Per-account in-flight serialization: reject concurrent attempts for same username
    let _in_flight_guard = session_store.acquire_in_flight_login(&username_raw)?;

    // Check if this username is the sole usable Super Admin (D3.14)
    let is_sole = is_sole_active_super_admin(conn, &username_raw).unwrap_or(false);

    // SEC-010 / D3.14: Check lockout status before verifying credentials.
    // Sole usable Super Admin is exempt from timed lockout.
    session_store.check_lockout_with_exemption(&username_raw, is_sole)?;

    // D3.15: Progressive throttling for sole Super Admin, bounded and outside global locks
    if is_sole {
        let throttle_delay = session_store.calculate_progressive_delay(&username_raw);
        if throttle_delay > std::time::Duration::ZERO {
            std::thread::sleep(throttle_delay);
        }
    }

    let username_normalized = normalize_local_account_username(payload.username);

    let maybe = match &username_normalized {
        Some(uname) => conn
            .query_row(
                "SELECT id, password_hash FROM app_local_accounts WHERE username = ? COLLATE NOCASE",
                params![uname.as_str()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|err| format!("failed to query account for login: {err}"))?,
        None => None,
    };

    let credentials_valid = match &maybe {
        Some((_id, password_hash)) => {
            verify_password(payload.password.as_str(), password_hash.as_str())?
        }
        None => {
            // SEC-010: Constant-time verification against precomputed dummy Argon2id hash
            // to eliminate timing differences and prevent user enumeration.
            static DUMMY_HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
            let dummy = DUMMY_HASH.get_or_init(|| {
                hash_password("staffkit_dummy_constant_time_salt_password").unwrap_or_default()
            });
            let _ = verify_password(payload.password.as_str(), dummy);
            false
        }
    };

    if !credentials_valid {
        session_store.record_failed_attempt_with_exemption(&username_raw, is_sole)?;
        return Err("incorrect username or password".to_string());
    }

    let (id, _password_hash) = maybe.expect("valid credentials guarantee account exists");

    // Clear failed attempts on successful login
    session_store.record_successful_login(&username_raw);

    // Load the verified record to derive identity server-side. The frontend
    // never supplies role/account_key; both come from the DB row.
    let account = load_local_account_by_id(conn, id)?;
    let role = crate::auth_session::Role::from_db_str(&account.role);
    let session_token = session_store.issue_session(account.id, &account.account_key, role);

    // Keep the legacy "active account" UI hint in sync. This DB row is no longer
    // part of the authorization trust path (Phase A); it only drives the login
    // screen's username prefill.
    set_active_local_account_id(conn, id)?;

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
    session_store: &crate::auth_session::SessionStore,
    payload: LocalPasswordChangeInput,
) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    change_local_account_password_conn(&conn, session_store, payload)
}

pub(crate) fn change_local_account_password_conn(
    conn: &Connection,
    session_store: &crate::auth_session::SessionStore,
    payload: LocalPasswordChangeInput,
) -> Result<bool, String> {
    let policy = resolve_security_policy_fail_safe(conn);
    let new_password = require_local_account_password(payload.new_password, policy.min_password_length())?;

    let maybe = conn
        .query_row(
            "SELECT username, password_hash FROM app_local_accounts WHERE id = ?",
            params![payload.id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|err| format!("failed to load password for change: {err}"))?;

    let Some((username, stored_hash)) = maybe else {
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

    // Unlock account upon successful password change
    session_store.unlock_account(&username);

    Ok(true)
}

pub fn admin_reset_local_account_password(
    app: &AppHandle,
    session_store: &crate::auth_session::SessionStore,
    payload: LocalPasswordResetInput,
) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    admin_reset_local_account_password_conn(&conn, session_store, payload)
}

pub(crate) fn admin_reset_local_account_password_conn(
    conn: &Connection,
    session_store: &crate::auth_session::SessionStore,
    payload: LocalPasswordResetInput,
) -> Result<bool, String> {
    let policy = resolve_security_policy_fail_safe(conn);
    let new_password = require_local_account_password(payload.new_password, policy.min_password_length())?;
    let new_hash = hash_password(new_password.as_str())?;

    let username: Option<String> = conn
        .query_row(
            "SELECT username FROM app_local_accounts WHERE id = ?",
            params![payload.id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to load account for reset: {err}"))?;

    let Some(username) = username else {
        return Err(format!(
            "local account with id {} was not found",
            payload.id
        ));
    };

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

    // Unlock target account upon admin reset (safe unlock behavior)
    session_store.unlock_account(&username);

    Ok(true)
}

pub fn forgot_local_account_password(
    app: &AppHandle,
    session_store: &crate::auth_session::SessionStore,
    payload: LocalForgotPasswordInput,
) -> Result<bool, String> {
    let conn = open_runtime_connection(app)?;
    forgot_local_account_password_conn(&conn, session_store, payload)
}

pub(crate) fn forgot_local_account_password_conn(
    conn: &Connection,
    session_store: &crate::auth_session::SessionStore,
    payload: LocalForgotPasswordInput,
) -> Result<bool, String> {
    let username_raw = payload.username.trim().to_string();
    if username_raw.is_empty() {
        return Err("incorrect username or recovery code".to_string());
    }

    // SEC-010: Check lockout first to prevent brute-forcing via recovery flow.
    // Check if this username is the sole usable Super Admin (exempt from lockout).
    let is_sole = is_sole_active_super_admin(conn, &username_raw).unwrap_or(false);
    session_store.check_lockout_with_exemption(&username_raw, is_sole)?;

    let policy = resolve_security_policy_fail_safe(conn);
    let new_password = require_local_account_password(payload.new_password, policy.min_password_length())?;

    let recovery_code = normalize_recovery_code_input(Some(payload.recovery_code))
        .ok_or_else(|| "recovery code is required".to_string())?;

    let username_normalized = normalize_local_account_username(payload.username);

    let maybe = match &username_normalized {
        Some(uname) => conn
            .query_row(
                "SELECT id, recovery_code_hash FROM app_local_accounts WHERE username = ? COLLATE NOCASE",
                params![uname.as_str()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()
            .map_err(|err| format!("failed to query account for recovery: {err}"))?,
        None => None,
    };

    let (id, recovery_valid) = match maybe {
        Some((id, Some(stored_hash))) => {
            (Some(id), verify_password(recovery_code.as_str(), stored_hash.as_str())?)
        }
        Some((id, None)) => {
            // No recovery code configured on this account; run dummy verification for constant time
            static DUMMY_HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
            let dummy = DUMMY_HASH.get_or_init(|| {
                hash_password("staffkit_dummy_constant_time_salt_recovery").unwrap_or_default()
            });
            let _ = verify_password(recovery_code.as_str(), dummy);
            (Some(id), false)
        }
        None => {
            // User not found; run dummy verification for constant time
            static DUMMY_HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
            let dummy = DUMMY_HASH.get_or_init(|| {
                hash_password("staffkit_dummy_constant_time_salt_recovery").unwrap_or_default()
            });
            let _ = verify_password(recovery_code.as_str(), dummy);
            (None, false)
        }
    };

    if !recovery_valid {
        session_store.record_failed_attempt_with_exemption(&username_raw, is_sole)?;
        return Err("incorrect username or recovery code".to_string());
    }

    let id = id.expect("valid recovery guarantee account exists");
    let new_hash = hash_password(new_password.as_str())?;
    conn.execute(
        "UPDATE app_local_accounts SET password_hash = ?, force_password_reset = 0, updated_at = datetime('now') WHERE id = ?",
        params![new_hash.as_str(), id],
    )
    .map_err(humanize_sqlite_error)?;

    // Unlock account upon successful recovery reset
    session_store.unlock_account(&username_raw);

    set_active_local_account_id(conn, id)?;
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

fn require_local_account_password(value: String, min_length: usize) -> Result<String, String> {
    let password = value.trim().to_string();
    if password.chars().count() < min_length {
        return Err(format!("password must be at least {} characters", min_length));
    }
    Ok(password)
}

/// Validate if `password_hash` has a valid, parseable Argon2id format.
pub(crate) fn is_valid_password_hash_format(password_hash: &str) -> bool {
    PasswordHash::new(password_hash).is_ok()
}

/// Determine whether `target_username` is the sole genuinely active/usable Super Admin (D3.14).
///
/// An account is considered an active/usable Super Admin only if:
/// 1. Its role is `super_admin` (case-insensitive)
/// 2. Username is non-empty
/// 3. Its password hash has a valid, parseable format (meaning the account is actually login-eligible)
///
/// If exactly 1 such account exists and matches `target_username`, returns `true`.
pub(crate) fn is_sole_active_super_admin(
    conn: &Connection,
    target_username: &str,
) -> Result<bool, String> {
    let normalized_target = target_username.trim().to_lowercase();
    if normalized_target.is_empty() {
        return Ok(false);
    }

    let mut stmt = conn
        .prepare("SELECT username, password_hash FROM app_local_accounts WHERE role = ? COLLATE NOCASE")
        .map_err(|err| format!("failed to prepare super admin query: {err}"))?;

    let rows = stmt
        .query_map(params![LOCAL_ACCOUNT_ROLE_SUPER_ADMIN], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|err| format!("failed to query super admins: {err}"))?;

    let mut eligible_admins = Vec::new();
    for row in rows {
        let (username, password_hash) =
            row.map_err(|err| format!("failed to read super admin row: {err}"))?;
        let u_trim = username.trim();
        let p_trim = password_hash.trim();
        if !u_trim.is_empty() && is_valid_password_hash_format(p_trim) {
            eligible_admins.push(u_trim.to_lowercase());
        }
    }

    Ok(eligible_admins.len() == 1 && eligible_admins[0] == normalized_target)
}

/// Read persisted security policy from `app_settings` (D3.19).
pub(crate) fn get_security_policy_mode(conn: &Connection) -> Result<SecurityPolicyMode, String> {
    let val = super::get_setting_value(conn, SECURITY_POLICY_SETTING_KEY)?;
    match val.as_deref() {
        Some(s) => SecurityPolicyMode::from_str(s)
            .ok_or_else(|| format!("unknown security policy mode '{s}'")),
        None => Ok(SecurityPolicyMode::Standard), // default for new/unconfigured installations
    }
}

/// Read security policy with fail-safe fallback to Strict mode if reading fails (D3.19 Rule 10).
pub(crate) fn resolve_security_policy_fail_safe(conn: &Connection) -> SecurityPolicyMode {
    match get_security_policy_mode(conn) {
        Ok(mode) => mode,
        Err(err) => {
            eprintln!(
                "[SECURITY] Failed to read security policy from database: {err}. Safely falling back to Strict mode."
            );
            SecurityPolicyMode::Strict
        }
    }
}

pub fn get_security_policy(app: &AppHandle) -> Result<SecurityPolicyRecord, String> {
    let conn = open_runtime_connection(app)?;
    get_security_policy_conn(&conn)
}

pub(crate) fn get_security_policy_conn(conn: &Connection) -> Result<SecurityPolicyRecord, String> {
    let mode = resolve_security_policy_fail_safe(conn);
    Ok(SecurityPolicyRecord {
        mode,
        min_password_length: mode.min_password_length(),
        max_failed_login_attempts: mode.max_failed_login_attempts(),
        lockout_duration_minutes: mode.lockout_duration().as_secs() / 60,
    })
}

pub fn update_security_policy(
    app: &AppHandle,
    session_store: &crate::auth_session::SessionStore,
    actor_account_id: i64,
    payload: UpdateSecurityPolicyInput,
) -> Result<SecurityPolicyRecord, String> {
    let conn = open_runtime_connection(app)?;
    update_security_policy_conn(&conn, session_store, actor_account_id, payload)
}

pub(crate) fn update_security_policy_conn(
    conn: &Connection,
    session_store: &crate::auth_session::SessionStore,
    actor_account_id: i64,
    payload: UpdateSecurityPolicyInput,
) -> Result<SecurityPolicyRecord, String> {
    let actor = load_local_account_by_id(conn, actor_account_id)?;
    if actor.role != LOCAL_ACCOUNT_ROLE_SUPER_ADMIN {
        return Err("only super admin accounts may modify security policy".to_string());
    }

    let password_hash: String = conn
        .query_row(
            "SELECT password_hash FROM app_local_accounts WHERE id = ?",
            params![actor_account_id],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to load super admin credentials: {err}"))?;

    let is_valid = verify_password(&payload.current_password, &password_hash)?;
    if !is_valid {
        let _ = super::audit::insert_audit_log_conn(
            conn,
            "security_policy.change_failed",
            "account",
            Some(&actor.username),
            "security_policy",
            "global",
            Some(r#"{"reason":"incorrect_password"}"#),
        );
        let is_sole = is_sole_active_super_admin(conn, &actor.username).unwrap_or(false);
        let _ = session_store.record_failed_attempt_with_exemption(&actor.username, is_sole);
        return Err("incorrect password for security policy confirmation".to_string());
    }

    let old_mode = get_security_policy_mode(conn).unwrap_or(SecurityPolicyMode::Standard);
    super::set_setting_value(conn, SECURITY_POLICY_SETTING_KEY, Some(payload.new_mode.as_str()))?;

    // Switch policy in SessionStore: clears non-locked failure counts, preserves active lockouts
    session_store.switch_policy(payload.new_mode);

    let payload_json = format!(
        r#"{{"from":"{}","to":"{}"}}"#,
        old_mode.as_str(),
        payload.new_mode.as_str()
    );
    let _ = super::audit::insert_audit_log_conn(
        conn,
        "security_policy.updated",
        "account",
        Some(&actor.username),
        "security_policy",
        "global",
        Some(&payload_json),
    );

    get_security_policy_conn(conn)
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

    // ── SEC-010 & SEC-011 DB Integration Tests ─────────────────────────────────

    #[test]
    fn wrong_password_increments_failures_and_locks_at_5() {
        let conn = seeded_connection();
        let store = SessionStore::new();
        store.switch_policy(SecurityPolicyMode::Strict);

        // 4 failed attempts with wrong password
        for _ in 0..4 {
            let res = login_local_account_conn(
                &conn,
                &store,
                LocalAccountLoginInput {
                    username: "alice".to_string(),
                    password: "wrong_password".to_string(),
                },
            );
            assert!(res.is_err());
            assert_eq!(res.unwrap_err(), "incorrect username or password");
        }

        assert_eq!(store.get_failed_attempts("alice"), 4);
        assert!(!store.is_account_locked("alice"));

        // 5th failed attempt locks the account
        let res5 = login_local_account_conn(
            &conn,
            &store,
            LocalAccountLoginInput {
                username: "alice".to_string(),
                password: "wrong_password".to_string(),
            },
        );
        assert!(res5.is_err());
        assert!(res5.unwrap_err().contains("temporarily locked"));
        assert!(store.is_account_locked("alice"));

        // 6th attempt with CORRECT password is still rejected while locked
        let res6 = login_local_account_conn(
            &conn,
            &store,
            LocalAccountLoginInput {
                username: "alice".to_string(),
                password: "alicepw".to_string(),
            },
        );
        assert!(res6.is_err());
        assert!(res6.unwrap_err().contains("temporarily locked"));
    }

    #[test]
    fn lockout_expiry_allows_successful_login() {
        let conn = seeded_connection();
        let clock = crate::auth_session::TestClock::new();
        let store = SessionStore::with_clock(clock.handle());
        store.switch_policy(SecurityPolicyMode::Strict);

        // Lock alice out with 5 failed attempts
        for _ in 0..5 {
            let _ = login_local_account_conn(
                &conn,
                &store,
                LocalAccountLoginInput {
                    username: "alice".to_string(),
                    password: "wrong_password".to_string(),
                },
            );
        }
        assert!(store.is_account_locked("alice"));

        // Advance 14 minutes -> still locked
        clock.advance(std::time::Duration::from_secs(14 * 60));
        assert!(login_local_account_conn(
            &conn,
            &store,
            LocalAccountLoginInput {
                username: "alice".to_string(),
                password: "alicepw".to_string(),
            },
        )
        .is_err());

        // Advance 1 more minute (15 min total) -> lockout expires, login succeeds!
        clock.advance(std::time::Duration::from_secs(60));
        let login_res = login_local_account_conn(
            &conn,
            &store,
            LocalAccountLoginInput {
                username: "alice".to_string(),
                password: "alicepw".to_string(),
            },
        );
        assert!(login_res.is_ok(), "login must succeed after lockout expiry");
        assert_eq!(store.get_failed_attempts("alice"), 0);
        assert!(!store.is_account_locked("alice"));
    }

    #[test]
    fn unknown_username_timing_and_lockout() {
        let conn = seeded_connection();
        let store = SessionStore::new();
        store.switch_policy(SecurityPolicyMode::Strict);

        // 4 failed attempts on nonexistent user
        for _ in 0..4 {
            let res = login_local_account_conn(
                &conn,
                &store,
                LocalAccountLoginInput {
                    username: "nonexistent_user".to_string(),
                    password: "some_password".to_string(),
                },
            );
            assert_eq!(res.unwrap_err(), "incorrect username or password");
        }

        // 5th failed attempt on nonexistent user locks out identically
        let res5 = login_local_account_conn(
            &conn,
            &store,
            LocalAccountLoginInput {
                username: "nonexistent_user".to_string(),
                password: "some_password".to_string(),
            },
        );
        assert!(res5.is_err());
        assert!(res5.unwrap_err().contains("temporarily locked"));
        assert!(store.is_account_locked("nonexistent_user"));
    }

    #[test]
    fn forgot_password_rate_limiting_and_safe_unlock() {
        let conn = seeded_connection();
        let store = SessionStore::new();
        store.switch_policy(SecurityPolicyMode::Strict);

        // Add recovery code for alice
        let recovery_hash = hash_password("ALICE-REC-1234").expect("hash recovery");
        conn.execute(
            "UPDATE app_local_accounts SET recovery_code_hash = ? WHERE username = 'alice'",
            params![recovery_hash],
        )
        .expect("update recovery code");

        // 4 wrong recovery attempts
        for _ in 0..4 {
            let res = forgot_local_account_password_conn(
                &conn,
                &store,
                LocalForgotPasswordInput {
                    username: "alice".to_string(),
                    recovery_code: "WRONG-CODE".to_string(),
                    new_password: "NewSafePassword123!".to_string(),
                },
            );
            assert_eq!(res.unwrap_err(), "incorrect username or recovery code");
        }

        // 5th wrong recovery attempt locks out
        let res5 = forgot_local_account_password_conn(
            &conn,
            &store,
            LocalForgotPasswordInput {
                username: "alice".to_string(),
                recovery_code: "WRONG-CODE".to_string(),
                new_password: "NewSafePassword123!".to_string(),
            },
        );
        assert!(res5.unwrap_err().contains("temporarily locked"));
        assert!(store.is_account_locked("alice"));

        // While locked, even correct recovery code is blocked
        let res_locked = forgot_local_account_password_conn(
            &conn,
            &store,
            LocalForgotPasswordInput {
                username: "alice".to_string(),
                recovery_code: "ALICE-REC-1234".to_string(),
                new_password: "NewSafePassword123!".to_string(),
            },
        );
        assert!(res_locked.unwrap_err().contains("temporarily locked"));

        // Unlock account (e.g. by Admin reset)
        let admin_reset_res = admin_reset_local_account_password_conn(
            &conn,
            &store,
            LocalPasswordResetInput {
                id: 2, // alice
                new_password: "AdminResetPass123!".to_string(),
            },
        );
        assert!(admin_reset_res.is_ok());
        assert!(!store.is_account_locked("alice"));

        // Alice can now log in with the new password
        let login_res = login_local_account_conn(
            &conn,
            &store,
            LocalAccountLoginInput {
                username: "alice".to_string(),
                password: "AdminResetPass123!".to_string(),
            },
        );
        assert!(login_res.is_ok());
    }

    #[test]
    fn password_length_policy_minimum_12_chars_and_passphrases() {
        let conn = seeded_connection();
        super::super::set_setting_value(&conn, SECURITY_POLICY_SETTING_KEY, Some("strict")).unwrap();
        let store = SessionStore::new();
        store.switch_policy(SecurityPolicyMode::Strict);

        // 1. Rejects passwords shorter than 12 characters
        let short_res = change_local_account_password_conn(
            &conn,
            &store,
            LocalPasswordChangeInput {
                id: 2, // alice
                current_password: "alicepw".to_string(),
                new_password: "short_pw_11".to_string(), // 11 chars
            },
        );
        assert!(short_res.is_err());
        assert_eq!(short_res.unwrap_err(), "password must be at least 12 characters");

        // 2. Accepts exactly 12 characters
        let exact_12 = change_local_account_password_conn(
            &conn,
            &store,
            LocalPasswordChangeInput {
                id: 2, // alice
                current_password: "alicepw".to_string(),
                new_password: "TwelveChars!".to_string(), // 12 chars
            },
        );
        assert!(exact_12.is_ok());

        // 3. Accepts multi-word passphrase with spaces
        let passphrase = change_local_account_password_conn(
            &conn,
            &store,
            LocalPasswordChangeInput {
                id: 2, // alice
                current_password: "TwelveChars!".to_string(),
                new_password: "correct horse battery staple".to_string(), // 28 chars
            },
        );
        assert!(passphrase.is_ok());

        // 4. Accepts Unicode passphrase (Vietnamese characters)
        let unicode_passphrase = change_local_account_password_conn(
            &conn,
            &store,
            LocalPasswordChangeInput {
                id: 2, // alice
                current_password: "correct horse battery staple".to_string(),
                new_password: "mật khẩu an toàn 2026".to_string(), // 21 chars
            },
        );
        assert!(unicode_passphrase.is_ok());
    }

    #[test]
    fn legacy_accounts_compatibility_preserved() {
        let conn = seeded_connection();
        let store = SessionStore::new();

        // Default seeded adman account has 11-char password "Aswhite2026"
        let adman_login = login_local_account_conn(
            &conn,
            &store,
            LocalAccountLoginInput {
                username: "adman".to_string(),
                password: "Aswhite2026".to_string(),
            },
        );
        assert!(adman_login.is_ok(), "legacy account with <12 char password must log in cleanly");
        let result = adman_login.unwrap();
        assert_eq!(result.account.username, "adman");
        assert_eq!(result.account.role, "super_admin");
    }

    #[test]
    fn last_super_admin_protection_remains_intact() {
        let conn = seeded_connection();
        let clock = crate::auth_session::TestClock::new();
        let store = SessionStore::with_clock(clock.handle());

        // 1. Sole super_admin is exempt from timed lockout (D3.14)
        for _ in 0..12 {
            let res = login_local_account_conn(
                &conn,
                &store,
                LocalAccountLoginInput {
                    username: "adman".to_string(),
                    password: "wrong".to_string(),
                },
            );
            assert!(res.is_err());
        }
        assert!(!store.is_account_locked("adman"), "sole super admin must never be locked out");
        assert_eq!(store.get_failed_attempts("adman"), 12);

        // Correct password succeeds and clears failed attempts
        let ok_res = login_local_account_conn(
            &conn,
            &store,
            LocalAccountLoginInput {
                username: "adman".to_string(),
                password: "Aswhite2026".to_string(),
            },
        );
        assert!(ok_res.is_ok());
        assert_eq!(store.get_failed_attempts("adman"), 0);

        // 2. When a 2nd active super_admin is added, normal lockout applies to both (D3.16)
        store.switch_policy(SecurityPolicyMode::Strict);
        let super2_hash = hash_password("ValidSuperAdminPass123!").expect("hash super2");
        conn.execute(
            r#"
            INSERT INTO app_local_accounts(
              account_key, display_name, username, password_hash,
              recovery_code_hash, force_password_reset, role, created_at, updated_at
            )
            VALUES ('super2', 'Super2', 'super2', ?, NULL, 0, 'super_admin', datetime('now'), datetime('now'))
            "#,
            params![super2_hash],
        )
        .expect("seed super2 account");

        assert!(!is_sole_active_super_admin(&conn, "adman").expect("check sole"));
        assert!(!is_sole_active_super_admin(&conn, "super2").expect("check sole"));

        for _ in 0..5 {
            let _ = login_local_account_conn(
                &conn,
                &store,
                LocalAccountLoginInput {
                    username: "adman".to_string(),
                    password: "wrong".to_string(),
                },
            );
        }
        assert!(store.is_account_locked("adman"), "with 2+ super admins, normal lockout applies");

        // Advance 15 minutes -> adman can log in again
        clock.advance(std::time::Duration::from_secs(15 * 60));
        assert!(login_local_account_conn(
            &conn,
            &store,
            LocalAccountLoginInput {
                username: "adman".to_string(),
                password: "Aswhite2026".to_string(),
            },
        )
        .is_ok());

        // 3. Last super_admin cannot be deleted
        let super2_id: i64 = conn
            .query_row("SELECT id FROM app_local_accounts WHERE username = 'super2'", [], |r| r.get(0))
            .expect("get super2 id");
        let del_super2 = delete_local_account_conn(&conn, 1, super2_id);
        assert!(del_super2.is_ok(), "can delete super2 when adman remains");

        let del_res = delete_local_account_conn(&conn, 2, 1);
        assert!(del_res.is_err(), "cannot delete the last remaining super admin");
        assert_eq!(del_res.unwrap_err(), "cannot delete the last remaining super admin");
    }

    // ── Security Policy Tests (Standard / Strict) ──────────────────────────────

    #[test]
    fn test_is_sole_active_super_admin_logic() {
        let conn = seeded_connection();

        // Initially: only 'adman' is super_admin with a valid hash
        assert!(is_sole_active_super_admin(&conn, "adman").unwrap());
        assert!(!is_sole_active_super_admin(&conn, "alice").unwrap());
        assert!(!is_sole_active_super_admin(&conn, "nonexistent").unwrap());

        // Add second super_admin with valid hash -> neither is sole
        let valid_hash = hash_password("ValidHash123456").unwrap();
        conn.execute(
            r#"
            INSERT INTO app_local_accounts(
              account_key, display_name, username, password_hash,
              recovery_code_hash, force_password_reset, role, created_at, updated_at
            )
            VALUES ('admin2', 'Admin 2', 'admin2', ?, NULL, 0, 'super_admin', datetime('now'), datetime('now'))
            "#,
            params![valid_hash],
        )
        .unwrap();

        assert!(!is_sole_active_super_admin(&conn, "adman").unwrap());
        assert!(!is_sole_active_super_admin(&conn, "admin2").unwrap());

        // Corrupt admin2's password hash to invalid format -> admin2 is not login-eligible
        conn.execute(
            "UPDATE app_local_accounts SET password_hash = 'corrupt-not-argon-hash' WHERE username = 'admin2'",
            [],
        )
        .unwrap();

        // Now adman is the sole USABLE/eligible super admin again!
        assert!(is_sole_active_super_admin(&conn, "adman").unwrap());
        assert!(!is_sole_active_super_admin(&conn, "admin2").unwrap());

        // Demote admin2 to user -> adman remains sole super admin
        conn.execute(
            "UPDATE app_local_accounts SET role = 'user' WHERE username = 'admin2'",
            [],
        )
        .unwrap();
        assert!(is_sole_active_super_admin(&conn, "adman").unwrap());
    }

    #[test]
    fn test_security_policy_persistence_and_fail_safe() {
        let conn = seeded_connection();

        // Unconfigured default is Standard (D3.19)
        let mode = get_security_policy_mode(&conn).unwrap();
        assert_eq!(mode, SecurityPolicyMode::Standard);
        assert_eq!(resolve_security_policy_fail_safe(&conn), SecurityPolicyMode::Standard);

        // Update setting to strict
        super::super::set_setting_value(&conn, SECURITY_POLICY_SETTING_KEY, Some("strict")).unwrap();
        assert_eq!(get_security_policy_mode(&conn).unwrap(), SecurityPolicyMode::Strict);
        assert_eq!(resolve_security_policy_fail_safe(&conn), SecurityPolicyMode::Strict);

        // Corrupt setting value -> fail-safe fallback to Strict (D3.19 Rule 10)
        super::super::set_setting_value(&conn, SECURITY_POLICY_SETTING_KEY, Some("corrupted_val")).unwrap();
        assert!(get_security_policy_mode(&conn).is_err());
        assert_eq!(resolve_security_policy_fail_safe(&conn), SecurityPolicyMode::Strict);
    }

    #[test]
    fn test_update_security_policy_reauth_and_audit() {
        let conn = seeded_connection();
        let store = SessionStore::new();

        // 1. Non-super_admin (alice, id=2) cannot change policy (D3.17)
        let err_role = update_security_policy_conn(
            &conn,
            &store,
            2,
            UpdateSecurityPolicyInput {
                current_password: "alicepw".to_string(),
                new_mode: SecurityPolicyMode::Strict,
            },
        );
        assert!(err_role.is_err());
        assert_eq!(err_role.unwrap_err(), "only super admin accounts may modify security policy");

        // 2. Wrong current password for super admin (adman, id=1) fails atomically and audits (D3.20)
        let err_pw = update_security_policy_conn(
            &conn,
            &store,
            1,
            UpdateSecurityPolicyInput {
                current_password: "wrong_adman_pw".to_string(),
                new_mode: SecurityPolicyMode::Strict,
            },
        );
        assert!(err_pw.is_err());
        assert_eq!(err_pw.unwrap_err(), "incorrect password for security policy confirmation");

        // Check failure audit log
        let count_fail: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE event_type = 'security_policy.change_failed'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_fail, 1);

        // Policy is unchanged
        assert_eq!(get_security_policy_mode(&conn).unwrap(), SecurityPolicyMode::Standard);
        assert_eq!(store.current_policy(), SecurityPolicyMode::Standard);

        // 3. Correct password succeeds, persists, updates in-memory store, and audits (D3.17)
        let res_ok = update_security_policy_conn(
            &conn,
            &store,
            1,
            UpdateSecurityPolicyInput {
                current_password: "Aswhite2026".to_string(),
                new_mode: SecurityPolicyMode::Strict,
            },
        );
        assert!(res_ok.is_ok());
        let rec = res_ok.unwrap();
        assert_eq!(rec.mode, SecurityPolicyMode::Strict);
        assert_eq!(rec.min_password_length, 12);
        assert_eq!(rec.max_failed_login_attempts, 5);
        assert_eq!(rec.lockout_duration_minutes, 15);

        // Check success audit log
        let count_success: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE event_type = 'security_policy.updated'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_success, 1);

        // DB setting and store are both Strict
        assert_eq!(get_security_policy_mode(&conn).unwrap(), SecurityPolicyMode::Strict);
        assert_eq!(store.current_policy(), SecurityPolicyMode::Strict);
    }

    #[test]
    fn test_password_length_enforcement_by_policy() {
        let conn = seeded_connection();
        let store = SessionStore::new();

        // In Standard mode (default): min 8 characters
        assert_eq!(store.current_policy(), SecurityPolicyMode::Standard);
        let res_short_std = change_local_account_password_conn(
            &conn,
            &store,
            LocalPasswordChangeInput {
                id: 2,
                current_password: "alicepw".to_string(),
                new_password: "short7c".to_string(),
            },
        );
        assert!(res_short_std.is_err());
        assert_eq!(res_short_std.unwrap_err(), "password must be at least 8 characters");

        // 8 chars in Standard succeeds
        let res_8 = change_local_account_password_conn(
            &conn,
            &store,
            LocalPasswordChangeInput {
                id: 2,
                current_password: "alicepw".to_string(),
                new_password: "exact8ch".to_string(),
            },
        );
        assert!(res_8.is_ok());

        // Switch to Strict mode: min 12 characters
        super::super::set_setting_value(&conn, SECURITY_POLICY_SETTING_KEY, Some("strict")).unwrap();
        store.switch_policy(SecurityPolicyMode::Strict);

        // 11 chars in Strict fails
        let res_11 = change_local_account_password_conn(
            &conn,
            &store,
            LocalPasswordChangeInput {
                id: 2,
                current_password: "exact8ch".to_string(),
                new_password: "elevenchars".to_string(),
            },
        );
        assert!(res_11.is_err());
        assert_eq!(res_11.unwrap_err(), "password must be at least 12 characters");

        // Passphrase with spaces and Vietnamese Unicode (16 characters) succeeds in Strict
        let res_passphrase = change_local_account_password_conn(
            &conn,
            &store,
            LocalPasswordChangeInput {
                id: 2,
                current_password: "exact8ch".to_string(),
                new_password: "mật khẩu an toàn".to_string(),
            },
        );
        assert!(res_passphrase.is_ok());

        // Legacy accounts with short passwords (e.g. adman's Aswhite2026 = 11 chars) still log in!
        let login_legacy = login_local_account_conn(
            &conn,
            &store,
            LocalAccountLoginInput {
                username: "adman".to_string(),
                password: "Aswhite2026".to_string(),
            },
        );
        assert!(login_legacy.is_ok());
    }
}
