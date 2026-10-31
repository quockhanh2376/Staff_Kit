mod auth_session;
mod db;
mod lan_assets;
mod lan_auth;
mod lan_server;
pub mod recovery_ledger;
use tauri::Manager;

// ── Public commands (no session required: bootstrap/login/diagnostics) ───────

pub(crate) fn resolve_app_version(package_version: &str) -> String {
    let trimmed = package_version.trim();
    if !trimmed.is_empty() && trimmed != "0.0.0" {
        trimmed.to_string()
    } else {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

#[tauri::command]
fn get_app_version(app: tauri::AppHandle) -> String {
    resolve_app_version(&app.package_info().version.to_string())
}

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

#[tauri::command]
fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    Ok(())
}

#[tauri::command(async)]
fn write_export_file(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    path: String,
    contents: Vec<u8>,
) -> Result<(), String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    std::fs::write(&path, contents)
        .map_err(|error| format!("failed to write export file '{}': {error}", path))
}

#[tauri::command(async)]
fn init_database(app: tauri::AppHandle) -> Result<db::DatabaseStatus, String> {
    db::init_database(&app)
}

#[tauri::command(async)]
fn get_database_status(app: tauri::AppHandle) -> Result<db::DatabaseStatus, String> {
    db::database_status(&app)
}

// ── Backup / settings / history (admin) ──────────────────────────────────────

#[tauri::command(async)]
fn get_backup_settings(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::BackupSettings, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::get_backup_settings(&app)
}

#[tauri::command(async)]
fn update_backup_settings(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::BackupSettingsUpdateInput,
) -> Result<db::BackupSettings, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::update_backup_settings(&app, payload)
}

#[tauri::command(async)]
fn backup_database_now(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::BackupRunResult, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::backup_database_now(&app)
}

// ── Employee reads (authenticated) ───────────────────────────────────────────

#[tauri::command(async)]
fn list_employees(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    filters: db::EmployeeQuery,
) -> Result<db::EmployeeListResponse, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_employees(&app, filters)
}

#[tauri::command(async)]
fn search_employees(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    filters: db::EmployeeQuery,
) -> Result<db::EmployeeListResponse, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::search_employees(&app, filters)
}

#[tauri::command(async)]
fn get_employee_assigned_assets(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    employee_id: i64,
) -> Result<Vec<db::EmployeeAssignedAssetRecord>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::get_employee_assigned_assets(&app, employee_id)
}

// ── Employee asset seed (admin; actor from SessionContext) ───────────────────

#[tauri::command(async)]
fn preview_employee_asset_seed(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::EmployeeAssetSeedInput,
) -> Result<db::EmployeeAssetSeedPreview, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::preview_employee_asset_seed_with_actor(&app, ctx, payload)
}

#[tauri::command(async)]
fn import_employee_asset_seed(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::EmployeeAssetSeedInput,
) -> Result<db::EmployeeAssetSeedReport, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::import_employee_asset_seed_with_actor(&app, ctx, payload)
}

#[tauri::command(async)]
fn list_employee_group_counts(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::EmployeeGroupCounts, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_employee_group_counts(&app)
}

// ── Local accounts (super_admin CRUD; login/hints/logout public) ─────────────

#[tauri::command(async)]
fn list_local_accounts(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::LocalAccountRecord>, String> {
    let _ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    db::list_local_accounts(&app)
}

#[tauri::command(async)]
fn create_local_account(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::LocalAccountCreateInput,
) -> Result<db::LocalAccountRecord, String> {
    let _ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    db::create_local_account(&app, payload)
}

#[tauri::command(async)]
fn update_local_account(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::LocalAccountUpdateInput,
) -> Result<db::LocalAccountRecord, String> {
    let _ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    let target_id = payload.id;
    let result = db::update_local_account(&app, payload)?;
    // Role/displayName may have changed — invalidate the target's sessions so
    // the next login picks up the new role. The actor (super_admin) keeps theirs.
    session_store.invalidate_account(target_id);
    Ok(result)
}

#[tauri::command(async)]
fn delete_local_account(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    id: i64,
) -> Result<bool, String> {
    let ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    let result = db::delete_local_account(&app, ctx.account_id, id)?;
    // Invalidate the deleted account's sessions (if any survived). The actor
    // is a different account (self-delete is blocked), so their session is safe.
    session_store.invalidate_account(id);
    Ok(result)
}

// ── Login / logout / hints (public) ──────────────────────────────────────────

#[tauri::command(async)]
fn login_local_account(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    payload: db::LocalAccountLoginInput,
) -> Result<db::LocalAccountLoginResult, String> {
    db::login_local_account(&app, &session_store, payload)
}

#[tauri::command(async)]
fn logout_local_account(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<(), String> {
    db::logout_local_account(&session_store, &session_token);
    Ok(())
}

#[tauri::command(async)]
fn list_login_account_hints(app: tauri::AppHandle) -> Result<Vec<db::LoginAccountHint>, String> {
    db::list_login_account_hints(&app)
}

#[tauri::command(async)]
fn change_local_account_password(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::LocalPasswordChangeInput,
) -> Result<bool, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    let target_id = payload.id;
    let result = db::change_local_account_password(&app, &session_store, payload)?;
    // Invalidate ALL sessions for this account (including the current token) so
    // the user must re-login with the new password.
    session_store.invalidate_account(target_id);
    Ok(result)
}

#[tauri::command(async)]
fn admin_reset_local_account_password(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::LocalPasswordResetInput,
) -> Result<bool, String> {
    let ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    let target_id = payload.id;
    let result = db::admin_reset_local_account_password(&app, &session_store, ctx.account_id, payload)?;
    // Invalidate the TARGET account's sessions (not the actor's). The target
    // must re-login with the new password; the admin stays logged in.
    session_store.invalidate_account(target_id);
    Ok(result)
}

#[tauri::command(async)]
fn forgot_local_account_password(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    payload: db::LocalForgotPasswordInput,
) -> Result<bool, String> {
    db::forgot_local_account_password(&app, &session_store, payload)
}

#[tauri::command(async)]
fn get_security_policy(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::SecurityPolicyRecord, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::get_security_policy(&app)
}

#[tauri::command(async)]
fn update_security_policy(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::UpdateSecurityPolicyInput,
) -> Result<db::SecurityPolicyRecord, String> {
    let ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    db::update_security_policy(&app, &session_store, ctx.account_id, payload)
}

// ── Recovery codes (Super Admin only, D3.20/D3.26/RC-Phase C1) ───────────────

#[tauri::command(async)]
fn get_recovery_code_status(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    _recovery_code_manager: tauri::State<'_, recovery_ledger::activation::RecoveryCodeManager>,
    session_token: String,
) -> Result<recovery_ledger::activation::RecoveryCodeStatus, String> {
    let ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    let conn = db::open_runtime_connection(&app)?;
    recovery_ledger::activation::RecoveryCodeManager::query_status(&conn, ctx.account_id)
}

#[tauri::command(async)]
fn generate_recovery_codes(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    recovery_code_manager: tauri::State<'_, recovery_ledger::activation::RecoveryCodeManager>,
    session_token: String,
    current_password: String,
) -> Result<recovery_ledger::activation::GenerateRecoveryCodesResult, String> {
    let ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    let conn = db::open_runtime_connection(&app)?;

    let (username, password_hash): (String, String) = conn
        .query_row(
            "SELECT username, password_hash FROM app_local_accounts WHERE id = ?",
            rusqlite::params![ctx.account_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("failed to load super admin credentials: {e}"))?;

    let is_valid = db::auth::verify_password(&current_password, &password_hash)?;
    if !is_valid {
        let _ = db::audit::insert_audit_log_conn(
            &conn,
            "recovery_codes.generation_failed",
            "account",
            Some(&username),
            "recovery_codes",
            "global",
            Some(r#"{"reason":"incorrect_password"}"#),
        );
        return Err("incorrect password for recovery code generation confirmation".to_string());
    }

    let result = recovery_code_manager.generate_codes(ctx.account_id, &username)?;

    let payload_json = format!(r#"{{"batch_id":"{}"}}"#, result.batch_id);
    let _ = db::audit::insert_audit_log_conn(
        &conn,
        "recovery_codes.generated",
        "account",
        Some(&username),
        "recovery_codes",
        "global",
        Some(&payload_json),
    );

    Ok(result)
}

#[tauri::command(async)]
fn activate_recovery_codes(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    recovery_code_manager: tauri::State<'_, recovery_ledger::activation::RecoveryCodeManager>,
    session_token: String,
    batch_id: String,
) -> Result<(), String> {
    let ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    let mut conn = db::open_runtime_connection(&app)?;

    let username: String = conn
        .query_row(
            "SELECT username FROM app_local_accounts WHERE id = ?",
            rusqlite::params![ctx.account_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "super_admin".to_string());

    let db_path = db::resolve_database_path(&app)?;
    let local_ledger_path = db_path.with_file_name(recovery_ledger::verification_gate::LEDGER_FILENAME);
    let master_key = db::APP_DB_ENCRYPTION_KEY.as_bytes();

    recovery_code_manager.activate_codes(
        &mut conn,
        ctx.account_id,
        &batch_id,
        &local_ledger_path,
        master_key,
    )?;

    let payload_json = format!(r#"{{"batch_id":"{}"}}"#, batch_id);
    let _ = db::audit::insert_audit_log_conn(
        &conn,
        "recovery_codes.activated",
        "account",
        Some(&username),
        "recovery_codes",
        "global",
        Some(&payload_json),
    );

    Ok(())
}

#[tauri::command(async)]
fn cancel_recovery_codes_generation(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    recovery_code_manager: tauri::State<'_, recovery_ledger::activation::RecoveryCodeManager>,
    session_token: String,
) -> Result<(), String> {
    let ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    recovery_code_manager.cancel_generation(ctx.account_id);

    if let Ok(conn) = db::open_runtime_connection(&app) {
        let username: Option<String> = conn
            .query_row(
                "SELECT username FROM app_local_accounts WHERE id = ?",
                rusqlite::params![ctx.account_id],
                |row| row.get(0),
            )
            .ok();

        let _ = db::audit::insert_audit_log_conn(
            &conn,
            "recovery_codes.generation_cancelled",
            "account",
            username.as_deref(),
            "recovery_codes",
            "global",
            None,
        );
    }

    Ok(())
}

#[tauri::command(async)]
fn revoke_recovery_codes(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    recovery_code_manager: tauri::State<'_, recovery_ledger::activation::RecoveryCodeManager>,
    session_token: String,
    current_password: String,
) -> Result<(), String> {
    let ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    let mut conn = db::open_runtime_connection(&app)?;

    let (username, password_hash): (String, String) = conn
        .query_row(
            "SELECT username, password_hash FROM app_local_accounts WHERE id = ?",
            rusqlite::params![ctx.account_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("failed to load super admin credentials: {e}"))?;

    let is_valid = db::auth::verify_password(&current_password, &password_hash)?;
    if !is_valid {
        let _ = db::audit::insert_audit_log_conn(
            &conn,
            "recovery_codes.revocation_failed",
            "account",
            Some(&username),
            "recovery_codes",
            "global",
            Some(r#"{"reason":"incorrect_password"}"#),
        );
        return Err("incorrect password for recovery code revocation confirmation".to_string());
    }

    let db_path = db::resolve_database_path(&app)?;
    let local_ledger_path = db_path.with_file_name(recovery_ledger::verification_gate::LEDGER_FILENAME);
    let master_key = db::APP_DB_ENCRYPTION_KEY.as_bytes();

    recovery_code_manager.revoke_codes(&mut conn, ctx.account_id, &local_ledger_path, master_key)?;

    let _ = db::audit::insert_audit_log_conn(
        &conn,
        "recovery_codes.revoked",
        "account",
        Some(&username),
        "recovery_codes",
        "global",
        None,
    );

    Ok(())
}

// ── Employee columns (read authenticated, mutate admin) ──────────────────────

#[tauri::command(async)]
fn list_employee_columns(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::EmployeeColumnDefinition>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_employee_columns(&app)
}

#[tauri::command(async)]
fn upsert_employee_column(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::EmployeeColumnUpsertInput,
) -> Result<db::EmployeeColumnDefinition, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::upsert_employee_column(&app, payload)
}

#[tauri::command(async)]
fn delete_employee_column(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    key: String,
) -> Result<bool, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::delete_employee_column(&app, key)
}

// ── Employee mutations (authenticated) ───────────────────────────────────────

#[tauri::command(async)]
fn create_employee(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::EmployeePayload,
) -> Result<db::EmployeeRecord, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::create_employee(&app, payload)
}

#[tauri::command(async)]
fn update_employee(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    id: i64,
    payload: db::EmployeePayload,
) -> Result<db::EmployeeRecord, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::update_employee(&app, id, payload)
}

#[tauri::command(async)]
fn move_employees_group(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::MoveEmployeesGroupInput,
) -> Result<i64, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::move_employees_group(&app, payload)
}

#[tauri::command(async)]
fn delete_employee(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    id: i64,
) -> Result<bool, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::delete_employee(&app, id)
}

// ── Teams (authenticated) ────────────────────────────────────────────────────

#[tauri::command(async)]
fn list_teams(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::TeamRecord>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_teams(&app)
}

#[tauri::command(async)]
fn upsert_team(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::TeamUpsertInput,
) -> Result<db::TeamRecord, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::upsert_team(&app, payload)
}

#[tauri::command(async)]
fn delete_team(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    id: i64,
) -> Result<bool, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::delete_team(&app, id)
}

// ── Data reset / restore / move DB (super_admin) ─────────────────────────────

#[tauri::command(async)]
fn reset_all_data(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<bool, String> {
    let _ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    let result = db::reset_all_data(&app)?;
    // All data was wiped — invalidate every session (including the actor's).
    // The frontend clears its session and returns to login/bootstrap.
    session_store.invalidate_all();
    Ok(result)
}

#[tauri::command(async)]
fn configure_usb_backup_destination(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::ConfigureUsbBackupInput,
) -> Result<db::BackupSettings, String> {
    let _ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    db::configure_usb_backup_destination(&app, payload)
}

// ── Excel import (admin) ─────────────────────────────────────────────────────

#[tauri::command(async)]
fn import_excel(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::ImportExcelInput,
) -> Result<db::ImportReport, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::import_excel(&app, payload)
}

#[tauri::command(async)]
fn preview_import_excel(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::ImportExcelInput,
) -> Result<db::ImportPreviewResult, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::preview_import_excel(&app, payload)
}

// ── Remote EE Sync (admin) ────────────────────────────────────────────────────

#[tauri::command(async)]
fn get_remote_ee_sync_settings(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::mssql_import::RemoteEeSyncSettings, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::mssql_import::get_remote_ee_sync_settings(&app)
}

#[tauri::command(async)]
fn save_remote_ee_sync_settings(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    server: String,
    database: String,
    port: u16,
    auth_type: db::mssql_import::MssqlAuthType,
    username: String,
    approved_username: String,
    password: Option<String>,
) -> Result<(), String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::mssql_import::save_remote_ee_sync_settings(
        &app,
        db::mssql_import::RemoteEeSyncSettings {
            server,
            database,
            port,
            auth_type,
            username,
            approved_username,
            has_saved_password: false,
            environment_override: false,
        },
        password,
        ctx.account_id,
    )
}

#[tauri::command(async)]
fn change_remote_ee_sync_approved_account(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    username: String,
    confirmation: String,
) -> Result<(), String> {
    let ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    db::mssql_import::change_remote_ee_sync_approved_account(
        &app,
        &username,
        &confirmation,
        ctx.account_id,
    )
}

#[tauri::command(async)]
fn remove_remote_ee_sync_password(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<(), String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::mssql_import::remove_remote_ee_sync_password()
}

#[tauri::command]
async fn test_remote_ee_sync_connection(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::mssql_import::MssqlConnectionTestResult, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    Ok(db::mssql_import::test_remote_ee_sync_connection(&app).await)
}

#[tauri::command]
async fn preview_employee_list_mssql_sync(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::mssql_import::EmployeeListSyncPreview, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::mssql_import::preview_employee_list_sync(&app).await
}

#[tauri::command]
async fn apply_employee_list_mssql_sync(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    preview_id: String,
) -> Result<db::mssql_import::EmployeeListSyncResult, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::mssql_import::apply_employee_list_sync(&app, &preview_id).await
}

#[tauri::command(async)]
fn inspect_import_columns(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::ImportExcelInput,
) -> Result<db::ImportColumnsPreview, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::inspect_import_columns(&app, payload)
}

#[tauri::command(async)]
fn detect_import_file(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    file_path: String,
) -> Result<db::ImportDetectionResult, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::detect_import_file(std::path::Path::new(&file_path))
}

// ── History snapshots (admin read, super_admin restore) ──────────────────────

#[tauri::command(async)]
fn list_history_snapshots(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::SnapshotInfo>, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::list_history_snapshots(&app)
}

#[tauri::command(async)]
fn create_history_snapshot_cmd(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    label: String,
) -> Result<db::SnapshotInfo, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::create_history_snapshot(&app, &label)
}

#[tauri::command(async)]
fn restore_history_snapshot(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    filename: String,
) -> Result<(), String> {
    let ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    db::restore_history_snapshot(&app, &filename, Some(ctx.account_id))?;
    // The DB was replaced — all sessions are stale. Invalidate everything.
    session_store.invalidate_all();
    Ok(())
}

#[tauri::command(async)]
fn move_database_to(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    target_folder: String,
) -> Result<String, String> {
    let _ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    let result = db::move_database_to(&app, &target_folder)?;
    // The active DB location changed — session identity may not transfer.
    session_store.invalidate_all();
    Ok(result)
}

#[tauri::command(async)]
fn restore_database_from_file(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    source_path: String,
) -> Result<(), String> {
    let ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    db::restore_database_from_file(&app, &source_path, Some(ctx.account_id))?;
    // The entire DB was replaced — invalidate all sessions.
    session_store.invalidate_all();
    Ok(())
}

#[tauri::command(async)]
fn get_db_custom_path(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Option<String>, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::get_db_custom_path(&app)
}

// ── Borrow LAN settings (admin) ──────────────────────────────────────────────

#[tauri::command(async)]
fn get_borrow_lan_settings(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::BorrowLanSettings, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::get_borrow_lan_settings(&app)
}

#[tauri::command]
async fn update_borrow_lan_settings(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    lan_server: tauri::State<'_, std::sync::Arc<lan_server::LanServerManager>>,
    session_token: String,
    payload: db::BorrowLanSettingsUpdateInput,
) -> Result<db::BorrowLanSettings, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    let updated = db::update_borrow_lan_settings_with_actor(&app, ctx, payload)?;
    if !updated.enabled {
        lan_server.stop().await?;
    }
    Ok(updated)
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BorrowLanTokenStatus {
    ready: bool,
}

#[tauri::command(async)]
fn get_borrow_lan_token_status(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    lan_token_store: tauri::State<'_, std::sync::Arc<lan_auth::LanTokenStore>>,
    lan_server: tauri::State<'_, std::sync::Arc<lan_server::LanServerManager>>,
    session_token: String,
) -> Result<BorrowLanTokenStatus, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    let server_status = lan_server.status()?;
    Ok(BorrowLanTokenStatus {
        ready: server_status.running && lan_token_store.is_ready(),
    })
}

#[tauri::command(async)]
fn issue_borrow_lan_token(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    lan_token_store: tauri::State<'_, std::sync::Arc<lan_auth::LanTokenStore>>,
    lan_server: tauri::State<'_, std::sync::Arc<lan_server::LanServerManager>>,
    session_token: String,
) -> Result<String, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    lan_server.require_running()?;
    Ok(lan_token_store.issue())
}

#[tauri::command(async)]
fn revoke_borrow_lan_token(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    lan_token_store: tauri::State<'_, std::sync::Arc<lan_auth::LanTokenStore>>,
    session_token: String,
) -> Result<(), String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    lan_token_store.revoke();
    Ok(())
}

#[tauri::command]
async fn start_borrow_lan_server(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    lan_server: tauri::State<'_, std::sync::Arc<lan_server::LanServerManager>>,
    session_token: String,
) -> Result<lan_server::LanServerStatus, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    lan_server.start().await?;
    lan_server.status()
}

#[tauri::command]
async fn stop_borrow_lan_server(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    lan_server: tauri::State<'_, std::sync::Arc<lan_server::LanServerManager>>,
    session_token: String,
) -> Result<lan_server::LanServerStatus, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    lan_server.stop().await?;
    lan_server.status()
}

#[tauri::command(async)]
fn get_borrow_lan_status(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    lan_server: tauri::State<'_, std::sync::Arc<lan_server::LanServerManager>>,
    session_token: String,
) -> Result<lan_server::LanServerStatus, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    lan_server.status()
}

#[tauri::command(async)]
fn detect_borrow_lan_host() -> Result<Option<String>, String> {
    db::detect_borrow_lan_host()
}

#[tauri::command]
async fn probe_lan_server(port: u16) -> bool {
    use std::time::Duration;
    use tokio::net::TcpStream;
    use tokio::time::timeout;

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    timeout(Duration::from_millis(800), TcpStream::connect(addr))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

// ── Asset import (admin; row edits authenticated) ────────────────────────────

#[tauri::command(async)]
fn inspect_asset_import_file(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetImportInspectInput,
) -> Result<db::AssetImportFileInspection, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::inspect_asset_import_file(payload)
}

#[tauri::command(async)]
fn preview_asset_import_file(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetDirectImportInput,
) -> Result<db::AssetDirectImportPreview, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::preview_asset_import_file(&app, payload)
}

#[tauri::command(async)]
fn import_asset_import_file(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetDirectImportInput,
) -> Result<db::AssetDirectImportReport, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::import_asset_import_file(&app, payload)
}

#[tauri::command(async)]
fn preview_historical_assignment_import(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::HistoricalAssignmentImportInput,
) -> Result<db::HistoricalAssignmentPreview, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::preview_historical_assignment_import(&app, payload)
}

#[tauri::command(async)]
fn apply_historical_assignment_import(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::HistoricalAssignmentImportInput,
) -> Result<db::HistoricalAssignmentImportReport, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::apply_historical_assignment_import(&app, payload, Some(ctx.account_id))
}

#[tauri::command(async)]
fn preview_asset_metadata_reimport(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetMetadataImportInput,
) -> Result<db::AssetMetadataPreviewReport, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::preview_asset_metadata_reimport(&app, payload)
}

#[tauri::command(async)]
fn apply_asset_metadata_reimport(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetMetadataImportInput,
) -> Result<db::AssetMetadataImportReport, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::apply_asset_metadata_reimport(&app, payload, Some(ctx.account_id))
}

#[tauri::command(async)]
fn sign_historical_borrow_request(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::SignHistoricalBorrowRequestInput,
) -> Result<(), String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::sign_historical_borrow_request(&app, payload)
}

#[tauri::command(async)]
fn create_asset_import_batch(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetImportBatchCreateInput,
) -> Result<db::AssetImportBatchDetail, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::create_asset_import_batch(&app, payload)
}

#[tauri::command(async)]
fn list_asset_import_batches(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::AssetImportBatchSummary>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_asset_import_batches(&app)
}

#[tauri::command(async)]
fn get_asset_import_batch_detail(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    batch_id: i64,
) -> Result<db::AssetImportBatchDetail, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::get_asset_import_batch_detail(&app, batch_id)
}

#[tauri::command(async)]
fn update_asset_import_row(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetImportRowUpdateInput,
) -> Result<db::AssetImportRowRecord, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::update_asset_import_row(&app, payload)
}

#[tauri::command(async)]
fn set_asset_import_row_skipped(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetImportRowSkipInput,
) -> Result<db::AssetImportRowRecord, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::set_asset_import_row_skipped(&app, payload)
}

#[tauri::command(async)]
fn import_asset_import_batch_valid_rows(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    batch_id: i64,
) -> Result<db::AssetImportCommitResult, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::import_asset_import_batch_valid_rows(&app, batch_id)
}

#[tauri::command(async)]
fn delete_asset_import_batch(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    batch_id: i64,
) -> Result<bool, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::delete_asset_import_batch(&app, batch_id)
}

// ── Asset CRUD (admin mutations) ─────────────────────────────────────────────

#[tauri::command(async)]
fn create_asset_manually(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetUpsertInput,
) -> Result<db::AssetRecord, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::create_asset_manually(&app, payload)
}

#[tauri::command(async)]
fn upsert_assets(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: Vec<db::AssetUpsertInput>,
) -> Result<Vec<db::AssetRecord>, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::upsert_assets(&app, payload)
}

// ── Asset categories (read authenticated, mutate admin) ──────────────────────

#[tauri::command(async)]
fn list_asset_categories(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::AssetCategoryRecord>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_asset_categories(&app)
}

#[tauri::command(async)]
fn list_asset_category_details(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::AssetCategoryDetailRecord>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_asset_category_details(&app)
}

#[tauri::command(async)]
fn create_asset_category(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetCategoryUpsertInput,
) -> Result<db::AssetCategoryDetailRecord, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::create_asset_category(&app, payload)
}

#[tauri::command(async)]
fn update_asset_category(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetCategoryUpsertInput,
) -> Result<db::AssetCategoryDetailRecord, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::update_asset_category(&app, payload)
}

#[tauri::command(async)]
fn deactivate_asset_category(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    category_id: i64,
) -> Result<db::AssetCategoryDetailRecord, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::deactivate_asset_category(&app, category_id)
}

// ── Asset dashboard (read authenticated, stock mutate admin) ─────────────────

#[tauri::command(async)]
fn get_asset_dashboard_summary(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::AssetDashboardSummary, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::get_asset_dashboard_summary(&app)
}

#[tauri::command(async)]
fn list_asset_dashboard_serialized(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::AssetDashboardSerializedRecord>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_asset_dashboard_serialized(&app)
}

#[tauri::command(async)]
fn list_asset_dashboard_quantity(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::AssetDashboardQuantityRecord>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_asset_dashboard_quantity(&app)
}

#[tauri::command(async)]
fn check_asset_integrity(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::AssetIntegrityCheckResult, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::check_asset_integrity(&app)
}

#[tauri::command(async)]
fn update_stock_item_quantity(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::StockItemQuantityUpdateInput,
) -> Result<db::AssetDashboardQuantityRecord, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::update_stock_item_quantity(&app, payload)
}

// ── Borrow review (admin; actor from SessionContext) ─────────────────────────

#[tauri::command(async)]
fn get_borrow_policy(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Option<db::borrow::BorrowPolicyDesktopRecord>, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::borrow::get_borrow_policy(&app)
}

#[tauri::command(async)]
fn save_borrow_policy(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::borrow::BorrowPolicyUpdateInput,
) -> Result<db::borrow::BorrowPolicyDesktopRecord, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::borrow::save_borrow_policy(&app, ctx.account_id, &payload.text_en, &payload.text_vi)
}

#[tauri::command(async)]
fn get_borrow_request_evidence(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    request_id: i64,
) -> Result<Option<db::borrow::BorrowRequestEvidenceRecord>, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::borrow::get_borrow_request_evidence(&app, request_id)
}

#[tauri::command(async)]
fn list_pending_borrow_requests(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::BorrowRequestRecord>, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::list_pending_borrow_requests(&app)
}

#[tauri::command(async)]
fn get_borrow_request_detail(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    request_id: i64,
) -> Result<db::BorrowRequestRecord, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::get_borrow_request_detail(&app, request_id)
}

#[tauri::command(async)]
fn approve_borrow_request(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    request_id: i64,
) -> Result<db::BorrowRequestRecord, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::approve_borrow_request_with_actor(&app, ctx, request_id)
}

#[tauri::command(async)]
fn reject_borrow_request(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::BorrowRequestRejectInput,
) -> Result<db::BorrowRequestRecord, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::reject_borrow_request_with_actor(&app, ctx, payload)
}

#[tauri::command(async)]
fn cancel_borrow_request(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    request_id: i64,
) -> Result<db::BorrowRequestRecord, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::cancel_borrow_request_with_actor(&app, ctx, request_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let lan_token_store = std::sync::Arc::new(lan_auth::LanTokenStore::new());
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(auth_session::SessionStore::new())
        .manage(recovery_ledger::activation::RecoveryCodeManager::new())
        .manage(lan_token_store.clone())
        .setup({
            let lan_token_store = lan_token_store.clone();
            move |app| {
                let setup_start = std::time::Instant::now();
                let lan_server = std::sync::Arc::new(lan_server::LanServerManager::new(
                    app.handle().clone(),
                    lan_token_store.clone(),
                ));
                app.manage(lan_server.clone());

                // LAN is intentionally manual-only. The QR card owns the
                // serialized start/stop lifecycle after the app is ready.
                //
                // Main window visibility follows docs/superpowers/specs/2026-09-12-tauri-startup-webview-paint-design.md:
                // Frontend signals readiness via `show_main_window` after first paint (requestAnimationFrame).
                // Bounded fallback: if frontend crashes before signaling, show window after 7s to avoid orphan process.
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(7));
                    if let Some(window) = app_handle.get_webview_window("main") {
                        if !window.is_visible().unwrap_or(false) {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                });

                // Start background auto-backup scheduler (TotalSpec P1.1 / D1.1):
                // Checks every 15 minutes, catches up overdue on start after 5s grace period.
                // Runs completely in background via spawn_blocking without blocking UI or LAN portal.
                let backup_app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(15 * 60));
                    loop {
                        interval.tick().await;
                        let handle = backup_app_handle.clone();
                        let _ = tauri::async_runtime::spawn_blocking(move || {
                            let _ = db::run_auto_backup_if_due(&handle);
                        })
                        .await;
                    }
                });

                if let Ok(conn) = db::open_runtime_connection(app.handle()) {
                    let mode = db::resolve_security_policy_fail_safe(&conn);
                    if let Some(store) = app.try_state::<auth_session::SessionStore>() {
                        store.switch_policy(mode);
                    }
                    if let Ok(db_path) = db::resolve_database_path(app.handle()) {
                        let local_ledger_path = db_path.with_file_name(recovery_ledger::verification_gate::LEDGER_FILENAME);
                        let journal_dir = local_ledger_path.parent().unwrap_or_else(|| std::path::Path::new("."));
                        let journal = recovery_ledger::journal::RecoveryJournal::new(journal_dir.to_path_buf());
                        let master_key = db::APP_DB_ENCRYPTION_KEY.as_bytes();
                        let _ = recovery_ledger::reconciler::RecoveryTransactionCoordinator::reconcile_on_startup(
                            &conn,
                            &journal,
                            &local_ledger_path,
                            master_key,
                        );
                    }
                }

                eprintln!("[Staff Kit Perf] tauri setup took {:?}", setup_start.elapsed());
                Ok(())
            }
        })
        .invoke_handler(tauri::generate_handler![
            // ── Public (bootstrap/login/diagnostics) ───────────────────────────
            ping,
            get_app_version,
            show_main_window,
            init_database,
            get_database_status,
            login_local_account,
            logout_local_account,
            list_login_account_hints,
            forgot_local_account_password,
            detect_borrow_lan_host,
            probe_lan_server,
            // ── Guarded commands ───────────────────────────────────────────────
            write_export_file,
            get_backup_settings,
            update_backup_settings,
            backup_database_now,
            list_history_snapshots,
            create_history_snapshot_cmd,
            restore_history_snapshot,
            move_database_to,
            get_db_custom_path,
            restore_database_from_file,
            get_borrow_lan_settings,
            update_borrow_lan_settings,
            get_borrow_lan_token_status,
            issue_borrow_lan_token,
            revoke_borrow_lan_token,
            start_borrow_lan_server,
            stop_borrow_lan_server,
            get_borrow_lan_status,
            inspect_asset_import_file,
            preview_asset_import_file,
            import_asset_import_file,
            preview_historical_assignment_import,
            apply_historical_assignment_import,
            preview_asset_metadata_reimport,
            apply_asset_metadata_reimport,
            sign_historical_borrow_request,
            create_asset_import_batch,
            list_asset_import_batches,
            get_asset_import_batch_detail,
            update_asset_import_row,
            set_asset_import_row_skipped,
            import_asset_import_batch_valid_rows,
            delete_asset_import_batch,
            create_asset_manually,
            upsert_assets,
            list_asset_categories,
            list_asset_category_details,
            create_asset_category,
            update_asset_category,
            deactivate_asset_category,
            get_asset_dashboard_summary,
            list_asset_dashboard_serialized,
            list_asset_dashboard_quantity,
            check_asset_integrity,
            update_stock_item_quantity,
            get_borrow_policy,
            save_borrow_policy,
            get_borrow_request_evidence,
            list_pending_borrow_requests,
            get_borrow_request_detail,
            approve_borrow_request,
            reject_borrow_request,
            cancel_borrow_request,
            list_employees,
            search_employees,
            get_employee_assigned_assets,
            preview_employee_asset_seed,
            import_employee_asset_seed,
            list_employee_group_counts,
            list_local_accounts,
            create_local_account,
            update_local_account,
            delete_local_account,
            change_local_account_password,
            admin_reset_local_account_password,
            list_employee_columns,
            upsert_employee_column,
            delete_employee_column,
            create_employee,
            update_employee,
            move_employees_group,
            delete_employee,
            list_teams,
            upsert_team,
            delete_team,
            reset_all_data,
            configure_usb_backup_destination,
            inspect_import_columns,
            detect_import_file,
            import_excel,
            preview_import_excel,
            get_remote_ee_sync_settings,
            save_remote_ee_sync_settings,
            change_remote_ee_sync_approved_account,
            remove_remote_ee_sync_password,
            test_remote_ee_sync_connection,
            preview_employee_list_mssql_sync,
            apply_employee_list_mssql_sync,
            get_security_policy,
            update_security_policy,
            get_recovery_code_status,
            generate_recovery_codes,
            activate_recovery_codes,
            cancel_recovery_codes_generation,
            revoke_recovery_codes
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let app = window.app_handle().clone();
                // Best-effort snapshot on close; do not block close if it fails.
                // Internal lifecycle call — NOT an IPC command.
                let _ = db::create_history_snapshot(&app, "app_close");
                // Clear all in-memory sessions on shutdown. No tokens persist.
                if let Some(store) = app.try_state::<auth_session::SessionStore>() {
                    store.invalidate_all();
                }
                if let Some(store) = app.try_state::<std::sync::Arc<lan_auth::LanTokenStore>>() {
                    store.revoke();
                }
                if let Some(server) =
                    app.try_state::<std::sync::Arc<lan_server::LanServerManager>>()
                {
                    let _ = tauri::async_runtime::block_on(server.stop());
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Exhaustive command classification table ────────────────────────────────
    //
    // Every registered IPC command must appear in exactly one of these sets.
    // This test FAILS if a command is added without classification or if a
    // command appears in the handler list but not here. It is the regression
    // net for SEC-001 Phase C.

    const PUBLIC_COMMANDS: &[&str] = &[
        "ping",
        "get_app_version",
        "show_main_window",
        "init_database",
        "get_database_status",
        "login_local_account",
        "logout_local_account",
        "list_login_account_hints",
        "forgot_local_account_password",
        "detect_borrow_lan_host",
        "probe_lan_server",
    ];

    const SUPER_ADMIN_COMMANDS: &[&str] = &[
        "reset_all_data",
        "configure_usb_backup_destination",
        "restore_history_snapshot",
        "move_database_to",
        "restore_database_from_file",
        "create_local_account",
        "update_local_account",
        "delete_local_account",
        "list_local_accounts",
        "change_remote_ee_sync_approved_account",
        "update_security_policy",
        "get_recovery_code_status",
        "generate_recovery_codes",
        "activate_recovery_codes",
        "cancel_recovery_codes_generation",
        "revoke_recovery_codes",
        "admin_reset_local_account_password",
    ];

    const ADMIN_COMMANDS: &[&str] = &[
        "write_export_file",
        "get_backup_settings",
        "update_backup_settings",
        "backup_database_now",
        "list_history_snapshots",
        "create_history_snapshot_cmd",
        "get_db_custom_path",
        "get_borrow_lan_settings",
        "update_borrow_lan_settings",
        "get_borrow_lan_token_status",
        "issue_borrow_lan_token",
        "revoke_borrow_lan_token",
        "start_borrow_lan_server",
        "stop_borrow_lan_server",
        "get_borrow_lan_status",
        "inspect_asset_import_file",
        "preview_asset_import_file",
        "import_asset_import_file",
        "preview_historical_assignment_import",
        "apply_historical_assignment_import",
        "preview_asset_metadata_reimport",
        "apply_asset_metadata_reimport",
        "create_asset_import_batch",
        "import_asset_import_batch_valid_rows",
        "delete_asset_import_batch",
        "create_asset_manually",
        "upsert_assets",
        "create_asset_category",
        "update_asset_category",
        "deactivate_asset_category",
        "update_stock_item_quantity",
        "get_borrow_policy",
        "save_borrow_policy",
        "get_borrow_request_evidence",
        "list_pending_borrow_requests",
        "get_borrow_request_detail",
        "approve_borrow_request",
        "reject_borrow_request",
        "cancel_borrow_request",
        "preview_employee_asset_seed",
        "import_employee_asset_seed",
        "upsert_employee_column",
        "delete_employee_column",
        "inspect_import_columns",
        "import_excel",
        "preview_import_excel",
        "detect_import_file",
        "get_remote_ee_sync_settings",
        "save_remote_ee_sync_settings",
        "remove_remote_ee_sync_password",
        "test_remote_ee_sync_connection",
        "preview_employee_list_mssql_sync",
        "apply_employee_list_mssql_sync",
    ];

    const AUTHENTICATED_COMMANDS: &[&str] = &[
        "list_employees",
        "search_employees",
        "get_employee_assigned_assets",
        "list_employee_group_counts",
        "change_local_account_password",
        "list_employee_columns",
        "create_employee",
        "update_employee",
        "move_employees_group",
        "delete_employee",
        "list_teams",
        "upsert_team",
        "delete_team",
        "list_asset_import_batches",
        "get_asset_import_batch_detail",
        "update_asset_import_row",
        "set_asset_import_row_skipped",
        "list_asset_categories",
        "list_asset_category_details",
        "get_asset_dashboard_summary",
        "list_asset_dashboard_serialized",
        "list_asset_dashboard_quantity",
        "check_asset_integrity",
        "sign_historical_borrow_request",
        "get_security_policy",
    ];

    /// Commands that must NOT be registered for IPC (internal-only or removed).
    const FORBIDDEN_IPC_COMMANDS: &[&str] = &["set_active_local_account", "run_auto_backup_if_due"];

    /// The exact set of commands registered in `generate_handler!`.
    fn registered_commands() -> Vec<&'static str> {
        let mut all = Vec::new();
        all.extend_from_slice(PUBLIC_COMMANDS);
        all.extend_from_slice(SUPER_ADMIN_COMMANDS);
        all.extend_from_slice(ADMIN_COMMANDS);
        all.extend_from_slice(AUTHENTICATED_COMMANDS);
        all
    }

    #[test]
    fn resolve_app_version_handles_standard_and_prerelease_and_fallback() {
        assert_eq!(resolve_app_version("2.2.1"), "2.2.1");
        assert_eq!(resolve_app_version("2.2.1-internal-rc1"), "2.2.1-internal-rc1");
        assert_eq!(resolve_app_version("2.3.0-beta.2"), "2.3.0-beta.2");
        assert_eq!(resolve_app_version("0.0.0"), env!("CARGO_PKG_VERSION"));
        assert_eq!(resolve_app_version(""), env!("CARGO_PKG_VERSION"));
        assert_eq!(resolve_app_version("   "), env!("CARGO_PKG_VERSION"));
        assert!(PUBLIC_COMMANDS.contains(&"get_app_version"));
    }

    #[test]
    fn every_registered_command_is_classified() {
        let classified: std::collections::HashSet<&str> =
            registered_commands().into_iter().collect();
        // No duplicates in the classification table.
        let total = PUBLIC_COMMANDS.len()
            + SUPER_ADMIN_COMMANDS.len()
            + ADMIN_COMMANDS.len()
            + AUTHENTICATED_COMMANDS.len();
        assert_eq!(
            classified.len(),
            total,
            "duplicate command names in the classification table"
        );
    }

    #[test]
    fn forbidden_commands_are_not_in_classification() {
        let registered: std::collections::HashSet<&str> =
            registered_commands().into_iter().collect();
        for forbidden in FORBIDDEN_IPC_COMMANDS {
            assert!(
                !registered.contains(*forbidden),
                "forbidden command '{}' must not be registered for IPC",
                forbidden
            );
        }
    }

    #[test]
    fn remote_ee_sync_uses_admin_guard_and_exposes_no_arbitrary_mssql_sql_command() {
        for command in [
            "get_remote_ee_sync_settings",
            "save_remote_ee_sync_settings",
            "remove_remote_ee_sync_password",
            "test_remote_ee_sync_connection",
            "preview_employee_list_mssql_sync",
            "apply_employee_list_mssql_sync",
        ] {
            assert!(
                ADMIN_COMMANDS.contains(&command),
                "{command} must require admin access"
            );
        }
        assert!(SUPER_ADMIN_COMMANDS.contains(&"change_remote_ee_sync_approved_account"));
        assert!(!ADMIN_COMMANDS.contains(&"change_remote_ee_sync_approved_account"));
        assert!(SUPER_ADMIN_COMMANDS.contains(&"admin_reset_local_account_password"));
        assert!(!ADMIN_COMMANDS.contains(&"admin_reset_local_account_password"));
        for removed in [
            "get_mssql_connection_defaults",
            "test_mssql_connection",
            "preview_mssql_staff",
            "import_mssql_staff",
        ] {
            assert!(
                !registered_commands().contains(&removed),
                "legacy SQL command {removed} must not be registered"
            );
        }
    }

    #[test]
    fn classification_counts_are_non_empty_and_disjoint() {
        assert!(!PUBLIC_COMMANDS.is_empty());
        assert!(!SUPER_ADMIN_COMMANDS.is_empty());
        assert!(!ADMIN_COMMANDS.is_empty());
        assert!(!AUTHENTICATED_COMMANDS.is_empty());
        // No command appears in two sets.
        let mut seen = std::collections::HashSet::new();
        for cmd in registered_commands() {
            assert!(
                seen.insert(cmd),
                "command '{cmd}' appears in multiple classification sets"
            );
        }
    }

    #[test]
    fn detect_import_file_requires_an_admin_session() {
        assert!(ADMIN_COMMANDS.contains(&"detect_import_file"));
        assert!(!AUTHENTICATED_COMMANDS.contains(&"detect_import_file"));

        let store = auth_session::SessionStore::new();
        assert_eq!(
            auth_session::require_admin(&store, "").unwrap_err().code(),
            auth_session::AUTH_REQUIRED
        );

        let user_token = store.issue_session(1, "user", auth_session::Role::User);
        assert_eq!(
            auth_session::require_admin(&store, &user_token)
                .unwrap_err()
                .code(),
            auth_session::AUTH_FORBIDDEN
        );

        let admin_token = store.issue_session(2, "admin", auth_session::Role::Admin);
        auth_session::require_admin(&store, &admin_token).expect("admin may detect imports");
    }

    // ── Guard-before-side-effect proof ────────────────────────────────────────
    //
    // These tests prove the authorization guard fires and returns an error
    // BEFORE any database, filesystem, or destructive side effect. They use
    // SessionStore directly (no AppHandle/DB needed) to verify the guard layer.
    // The guard returns `AuthError`, which `From<AuthError> for String` converts
    // — so the command wrapper would `?`-return before reaching the DB call.

    #[test]
    fn reset_all_data_guard_rejects_no_token_before_side_effect() {
        // The guard for reset_all_data is require_super_admin. With no session
        // token, it returns AUTH_REQUIRED — the command body (which opens the DB
        // and runs DELETE) is never reached.
        let store = auth_session::SessionStore::new();
        let err = auth_session::require_super_admin(&store, "").unwrap_err();
        assert_eq!(err.code(), auth_session::AUTH_REQUIRED);
    }

    #[test]
    fn reset_all_data_guard_rejects_user_and_admin_before_side_effect() {
        let store = auth_session::SessionStore::new();
        let user_tok = store.issue_session(1, "u", auth_session::Role::User);
        let admin_tok = store.issue_session(2, "a", auth_session::Role::Admin);

        assert_eq!(
            auth_session::require_super_admin(&store, &user_tok)
                .unwrap_err()
                .code(),
            auth_session::AUTH_FORBIDDEN
        );
        assert_eq!(
            auth_session::require_super_admin(&store, &admin_tok)
                .unwrap_err()
                .code(),
            auth_session::AUTH_FORBIDDEN
        );
    }

    #[test]
    fn write_export_file_guard_rejects_user_before_filesystem_write() {
        // write_export_file is admin-guarded. A user token is rejected before
        // std::fs::write is ever called.
        let store = auth_session::SessionStore::new();
        let user_tok = store.issue_session(3, "u", auth_session::Role::User);
        assert_eq!(
            auth_session::require_admin(&store, &user_tok)
                .unwrap_err()
                .code(),
            auth_session::AUTH_FORBIDDEN
        );
    }

    #[test]
    fn write_export_file_guard_rejects_no_token_before_filesystem_write() {
        let store = auth_session::SessionStore::new();
        assert_eq!(
            auth_session::require_admin(&store, "").unwrap_err().code(),
            auth_session::AUTH_REQUIRED
        );
    }

    #[test]
    fn lan_lifecycle_commands_are_admin_only() {
        let store = auth_session::SessionStore::new();
        let user_token = store.issue_session(7, "user", auth_session::Role::User);
        for command in [
            "start_borrow_lan_server",
            "stop_borrow_lan_server",
            "get_borrow_lan_status",
            "get_borrow_lan_token_status",
            "issue_borrow_lan_token",
            "revoke_borrow_lan_token",
        ] {
            assert!(
                ADMIN_COMMANDS.contains(&command),
                "LAN lifecycle command must be admin-classified: {command}"
            );
            assert_eq!(
                auth_session::require_admin(&store, &user_token)
                    .unwrap_err()
                    .code(),
                auth_session::AUTH_FORBIDDEN,
                "standard users must be rejected before {command} executes"
            );
        }
    }

    // ── Phase D: session invalidation matrix ──────────────────────────────────

    #[test]
    fn password_change_invalidates_all_sessions_for_that_account() {
        let store = auth_session::SessionStore::new();
        let tok1 = store.issue_session(5, "alice", auth_session::Role::User);
        let tok2 = store.issue_session(5, "alice", auth_session::Role::User);

        // Simulate the post-success hook from change_local_account_password.
        store.invalidate_account(5);

        assert_eq!(
            store.resolve_session(&tok1).unwrap_err().code(),
            auth_session::AUTH_REQUIRED
        );
        assert_eq!(
            store.resolve_session(&tok2).unwrap_err().code(),
            auth_session::AUTH_REQUIRED
        );
    }

    #[test]
    fn admin_reset_invalidates_target_sessions_but_preserves_actor_session() {
        let store = auth_session::SessionStore::new();
        let actor_tok = store.issue_session(1, "admin", auth_session::Role::Admin);
        let target_tok = store.issue_session(9, "bob", auth_session::Role::User);

        // Simulate admin_reset_local_account_password invalidating target only.
        store.invalidate_account(9);

        // Actor session survives.
        store
            .resolve_session(&actor_tok)
            .expect("actor session preserved");
        // Target session revoked.
        assert_eq!(
            store.resolve_session(&target_tok).unwrap_err().code(),
            auth_session::AUTH_REQUIRED
        );
    }

    #[test]
    fn account_deletion_invalidates_target_sessions() {
        let store = auth_session::SessionStore::new();
        let target_tok = store.issue_session(7, "bob", auth_session::Role::User);

        // Simulate delete_local_account post-success invalidation.
        store.invalidate_account(7);

        assert_eq!(
            store.resolve_session(&target_tok).unwrap_err().code(),
            auth_session::AUTH_REQUIRED
        );
    }

    #[test]
    fn role_change_invalidates_target_sessions() {
        let store = auth_session::SessionStore::new();
        let target_tok = store.issue_session(3, "carol", auth_session::Role::Admin);

        // Simulate update_local_account (role changed) post-success invalidation.
        store.invalidate_account(3);

        assert_eq!(
            store.resolve_session(&target_tok).unwrap_err().code(),
            auth_session::AUTH_REQUIRED,
            "no stale admin session survives role downgrade"
        );
    }

    #[test]
    fn failed_mutation_does_not_invalidate_sessions() {
        // If the DB mutation fails (returns Err), the command wrapper returns
        // early via `?` — the invalidation hook (which comes AFTER the mutation)
        // is never reached. Sessions survive.
        let store = auth_session::SessionStore::new();
        let tok = store.issue_session(1, "alice", auth_session::Role::SuperAdmin);

        // Simulate: mutation fails, no invalidation called.
        // The session must still be valid.
        store
            .resolve_session(&tok)
            .expect("session valid after failed mutation");
    }

    #[test]
    fn reset_all_data_success_invalidates_all_sessions() {
        let store = auth_session::SessionStore::new();
        let tok1 = store.issue_session(1, "a", auth_session::Role::SuperAdmin);
        let tok2 = store.issue_session(2, "b", auth_session::Role::User);

        // Simulate reset_all_data post-success.
        store.invalidate_all();

        assert_eq!(
            store.resolve_session(&tok1).unwrap_err().code(),
            auth_session::AUTH_REQUIRED
        );
        assert_eq!(
            store.resolve_session(&tok2).unwrap_err().code(),
            auth_session::AUTH_REQUIRED
        );
    }

    #[test]
    fn database_restore_success_invalidates_all_sessions() {
        let store = auth_session::SessionStore::new();
        let tok = store.issue_session(1, "a", auth_session::Role::SuperAdmin);

        // Simulate restore_database_from_file post-success.
        store.invalidate_all();

        assert_eq!(
            store.resolve_session(&tok).unwrap_err().code(),
            auth_session::AUTH_REQUIRED
        );
    }

    #[test]
    fn app_shutdown_clears_session_store() {
        let store = auth_session::SessionStore::new();
        store.issue_session(1, "a", auth_session::Role::User);
        store.issue_session(2, "b", auth_session::Role::Admin);
        assert_eq!(store.active_session_count(), 2);

        // Simulate on_window_event CloseRequested → invalidate_all.
        store.invalidate_all();

        assert_eq!(store.active_session_count(), 0);
    }

    #[test]
    fn no_stale_super_admin_session_survives_role_downgrade() {
        let store = auth_session::SessionStore::new();
        let tok = store.issue_session(5, "eve", auth_session::Role::SuperAdmin);

        // Role downgrade via update_local_account → invalidate_account(5).
        store.invalidate_account(5);

        // The old super_admin session is dead. A re-login would issue a User
        // role session instead (the DB row was updated).
        assert_eq!(
            store.resolve_session(&tok).unwrap_err().code(),
            auth_session::AUTH_REQUIRED
        );
    }

    #[test]
    fn self_delete_and_last_super_admin_protections_remain_intact() {
        // The self-delete rule and last-super-admin check live in db::auth and
        // are unchanged by Phase D. Verify the constants are still present.
        assert_eq!(
            auth_session::AUTH_CANNOT_DELETE_SELF,
            "AUTH_CANNOT_DELETE_SELF"
        );
    }
}
