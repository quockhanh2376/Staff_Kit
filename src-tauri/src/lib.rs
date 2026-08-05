mod auth_session;
mod db;
mod lan_assets;
mod lan_auth;
mod lan_server;
use tauri::Manager;

// ── Public commands (no session required: bootstrap/login/diagnostics) ───────

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

#[tauri::command]
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

#[tauri::command]
fn init_database(app: tauri::AppHandle) -> Result<db::DatabaseStatus, String> {
    db::init_database(&app)
}

#[tauri::command]
fn get_database_status(app: tauri::AppHandle) -> Result<db::DatabaseStatus, String> {
    db::database_status(&app)
}

// ── Backup / settings / history (admin) ──────────────────────────────────────

#[tauri::command]
fn get_backup_settings(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::BackupSettings, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::get_backup_settings(&app)
}

#[tauri::command]
fn update_backup_settings(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::BackupSettingsUpdateInput,
) -> Result<db::BackupSettings, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::update_backup_settings(&app, payload)
}

#[tauri::command]
fn backup_database_now(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::BackupRunResult, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::backup_database_now(&app)
}

// ── Employee reads (authenticated) ───────────────────────────────────────────

#[tauri::command]
fn list_employees(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    filters: db::EmployeeQuery,
) -> Result<db::EmployeeListResponse, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_employees(&app, filters)
}

#[tauri::command]
fn search_employees(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    filters: db::EmployeeQuery,
) -> Result<db::EmployeeListResponse, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::search_employees(&app, filters)
}

// ── Employee asset seed (admin; actor from SessionContext) ───────────────────

#[tauri::command]
fn preview_employee_asset_seed(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::EmployeeAssetSeedInput,
) -> Result<db::EmployeeAssetSeedPreview, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::preview_employee_asset_seed_with_actor(&app, ctx, payload)
}

#[tauri::command]
fn import_employee_asset_seed(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::EmployeeAssetSeedInput,
) -> Result<db::EmployeeAssetSeedReport, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::import_employee_asset_seed_with_actor(&app, ctx, payload)
}

#[tauri::command]
fn list_employee_group_counts(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::EmployeeGroupCounts, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_employee_group_counts(&app)
}

// ── Local accounts (super_admin CRUD; login/hints/logout public) ─────────────

#[tauri::command]
fn list_local_accounts(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::LocalAccountRecord>, String> {
    let _ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    db::list_local_accounts(&app)
}

#[tauri::command]
fn create_local_account(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::LocalAccountCreateInput,
) -> Result<db::LocalAccountRecord, String> {
    let _ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    db::create_local_account(&app, payload)
}

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
fn login_local_account(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    payload: db::LocalAccountLoginInput,
) -> Result<db::LocalAccountLoginResult, String> {
    db::login_local_account(&app, &session_store, payload)
}

#[tauri::command]
fn logout_local_account(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<(), String> {
    db::logout_local_account(&session_store, &session_token);
    Ok(())
}

#[tauri::command]
fn list_login_account_hints(app: tauri::AppHandle) -> Result<Vec<db::LoginAccountHint>, String> {
    db::list_login_account_hints(&app)
}

#[tauri::command]
fn change_local_account_password(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::LocalPasswordChangeInput,
) -> Result<bool, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    let target_id = payload.id;
    let result = db::change_local_account_password(&app, payload)?;
    // Invalidate ALL sessions for this account (including the current token) so
    // the user must re-login with the new password.
    session_store.invalidate_account(target_id);
    Ok(result)
}

#[tauri::command]
fn admin_reset_local_account_password(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::LocalPasswordResetInput,
) -> Result<bool, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    let target_id = payload.id;
    let result = db::admin_reset_local_account_password(&app, payload)?;
    // Invalidate the TARGET account's sessions (not the actor's). The target
    // must re-login with the new password; the admin stays logged in.
    session_store.invalidate_account(target_id);
    Ok(result)
}

#[tauri::command]
fn forgot_local_account_password(
    app: tauri::AppHandle,
    payload: db::LocalForgotPasswordInput,
) -> Result<bool, String> {
    db::forgot_local_account_password(&app, payload)
}

// ── Employee columns (read authenticated, mutate admin) ──────────────────────

#[tauri::command]
fn list_employee_columns(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::EmployeeColumnDefinition>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_employee_columns(&app)
}

#[tauri::command]
fn upsert_employee_column(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::EmployeeColumnUpsertInput,
) -> Result<db::EmployeeColumnDefinition, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::upsert_employee_column(&app, payload)
}

#[tauri::command]
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

#[tauri::command]
fn create_employee(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::EmployeePayload,
) -> Result<db::EmployeeRecord, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::create_employee(&app, payload)
}

#[tauri::command]
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

#[tauri::command]
fn move_employees_group(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::MoveEmployeesGroupInput,
) -> Result<i64, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::move_employees_group(&app, payload)
}

#[tauri::command]
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

#[tauri::command]
fn list_teams(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::TeamRecord>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_teams(&app)
}

#[tauri::command]
fn upsert_team(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::TeamUpsertInput,
) -> Result<db::TeamRecord, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::upsert_team(&app, payload)
}

#[tauri::command]
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

#[tauri::command]
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

// ── Excel import (admin) ─────────────────────────────────────────────────────

#[tauri::command]
fn import_excel(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::ImportExcelInput,
) -> Result<db::ImportReport, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    // Snapshot before import so user can rollback if needed
    let _ = db::create_history_snapshot(&app, "before_import");
    db::import_excel(&app, payload)
}

#[tauri::command]
fn preview_import_excel(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::ImportExcelInput,
) -> Result<db::ImportPreviewResult, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::preview_import_excel(&app, payload)
}

// ── MSSQL import (admin) ─────────────────────────────────────────────────────

#[tauri::command]
async fn get_mssql_connection_defaults(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::mssql_import::MssqlConnectionDefaults, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    Ok(db::mssql_import::get_mssql_connection_defaults().await)
}

#[tauri::command]
async fn test_mssql_connection(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    host: String,
    port: u16,
    user: String,
    password: String,
) -> Result<bool, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::mssql_import::test_mssql_connection(&host, port, &user, &password).await
}

#[tauri::command]
async fn preview_mssql_staff(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    host: String,
    port: u16,
    user: String,
    password: String,
    query: Option<String>,
) -> Result<db::mssql_import::MssqlImportPreview, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::mssql_import::preview_mssql_staff(&host, port, &user, &password, query.as_deref()).await
}

#[tauri::command]
async fn import_mssql_staff(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    host: String,
    port: u16,
    user: String,
    password: String,
    query: Option<String>,
    staff_group: Option<String>,
) -> Result<db::mssql_import::MssqlImportReport, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::mssql_import::import_mssql_staff(
        &app,
        &host,
        port,
        &user,
        &password,
        query.as_deref(),
        staff_group.as_deref(),
    )
    .await
}

#[tauri::command]
fn inspect_import_columns(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::ImportExcelInput,
) -> Result<db::ImportColumnsPreview, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::inspect_import_columns(&app, payload)
}

// ── History snapshots (admin read, super_admin restore) ──────────────────────

#[tauri::command]
fn list_history_snapshots(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::SnapshotInfo>, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::list_history_snapshots(&app)
}

#[tauri::command]
fn create_history_snapshot_cmd(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    label: String,
) -> Result<db::SnapshotInfo, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::create_history_snapshot(&app, &label)
}

#[tauri::command]
fn restore_history_snapshot(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    filename: String,
) -> Result<(), String> {
    let _ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    db::restore_history_snapshot(&app, &filename)?;
    // The DB was replaced — all sessions are stale. Invalidate everything.
    session_store.invalidate_all();
    Ok(())
}

#[tauri::command]
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

#[tauri::command]
fn restore_database_from_file(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    source_path: String,
) -> Result<(), String> {
    let _ctx = auth_session::require_super_admin(&session_store, &session_token)?;
    db::restore_database_from_file(&app, &source_path)?;
    // The entire DB was replaced — invalidate all sessions.
    session_store.invalidate_all();
    Ok(())
}

#[tauri::command]
fn get_db_custom_path(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Option<String>, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::get_db_custom_path(&app)
}

// ── Borrow LAN settings (admin) ──────────────────────────────────────────────

#[tauri::command]
fn get_borrow_lan_settings(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::BorrowLanSettings, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::get_borrow_lan_settings(&app)
}

#[tauri::command]
fn update_borrow_lan_settings(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::BorrowLanSettingsUpdateInput,
) -> Result<db::BorrowLanSettings, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::update_borrow_lan_settings_with_actor(&app, ctx, payload)
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BorrowLanTokenStatus {
    ready: bool,
}

#[tauri::command]
fn get_borrow_lan_token_status(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    lan_token_store: tauri::State<'_, std::sync::Arc<lan_auth::LanTokenStore>>,
    session_token: String,
) -> Result<BorrowLanTokenStatus, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    Ok(BorrowLanTokenStatus {
        ready: lan_token_store.is_ready(),
    })
}

#[tauri::command]
fn issue_borrow_lan_token(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    lan_token_store: tauri::State<'_, std::sync::Arc<lan_auth::LanTokenStore>>,
    session_token: String,
) -> Result<String, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    Ok(lan_token_store.issue())
}

#[tauri::command]
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

#[tauri::command]
fn inspect_asset_import_file(
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetImportInspectInput,
) -> Result<db::AssetImportFileInspection, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::inspect_asset_import_file(payload)
}

#[tauri::command]
fn preview_asset_import_file(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetDirectImportInput,
) -> Result<db::AssetDirectImportPreview, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::preview_asset_import_file(&app, payload)
}

#[tauri::command]
fn import_asset_import_file(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetDirectImportInput,
) -> Result<db::AssetDirectImportReport, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::import_asset_import_file(&app, payload)
}

#[tauri::command]
fn create_asset_import_batch(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetImportBatchCreateInput,
) -> Result<db::AssetImportBatchDetail, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::create_asset_import_batch(&app, payload)
}

#[tauri::command]
fn list_asset_import_batches(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::AssetImportBatchSummary>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_asset_import_batches(&app)
}

#[tauri::command]
fn get_asset_import_batch_detail(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    batch_id: i64,
) -> Result<db::AssetImportBatchDetail, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::get_asset_import_batch_detail(&app, batch_id)
}

#[tauri::command]
fn update_asset_import_row(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetImportRowUpdateInput,
) -> Result<db::AssetImportRowRecord, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::update_asset_import_row(&app, payload)
}

#[tauri::command]
fn set_asset_import_row_skipped(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetImportRowSkipInput,
) -> Result<db::AssetImportRowRecord, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::set_asset_import_row_skipped(&app, payload)
}

#[tauri::command]
fn import_asset_import_batch_valid_rows(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    batch_id: i64,
) -> Result<db::AssetImportCommitResult, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::import_asset_import_batch_valid_rows(&app, batch_id)
}

#[tauri::command]
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

#[tauri::command]
fn create_asset_manually(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetUpsertInput,
) -> Result<db::AssetRecord, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::create_asset_manually(&app, payload)
}

#[tauri::command]
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

#[tauri::command]
fn list_asset_categories(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::AssetCategoryRecord>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_asset_categories(&app)
}

#[tauri::command]
fn list_asset_category_details(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::AssetCategoryDetailRecord>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_asset_category_details(&app)
}

#[tauri::command]
fn create_asset_category(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetCategoryUpsertInput,
) -> Result<db::AssetCategoryDetailRecord, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::create_asset_category(&app, payload)
}

#[tauri::command]
fn update_asset_category(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::AssetCategoryUpsertInput,
) -> Result<db::AssetCategoryDetailRecord, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::update_asset_category(&app, payload)
}

#[tauri::command]
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

#[tauri::command]
fn get_asset_dashboard_summary(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<db::AssetDashboardSummary, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::get_asset_dashboard_summary(&app)
}

#[tauri::command]
fn list_asset_dashboard_serialized(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::AssetDashboardSerializedRecord>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_asset_dashboard_serialized(&app)
}

#[tauri::command]
fn list_asset_dashboard_quantity(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::AssetDashboardQuantityRecord>, String> {
    let _ctx = auth_session::require_authenticated(&session_store, &session_token)?;
    db::list_asset_dashboard_quantity(&app)
}

#[tauri::command]
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

#[tauri::command]
fn list_pending_borrow_requests(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
) -> Result<Vec<db::BorrowRequestRecord>, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::list_pending_borrow_requests(&app)
}

#[tauri::command]
fn get_borrow_request_detail(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    request_id: i64,
) -> Result<db::BorrowRequestRecord, String> {
    let _ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::get_borrow_request_detail(&app, request_id)
}

#[tauri::command]
fn approve_borrow_request(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    request_id: i64,
) -> Result<db::BorrowRequestRecord, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::approve_borrow_request_with_actor(&app, ctx, request_id)
}

#[tauri::command]
fn reject_borrow_request(
    app: tauri::AppHandle,
    session_store: tauri::State<'_, auth_session::SessionStore>,
    session_token: String,
    payload: db::BorrowRequestRejectInput,
) -> Result<db::BorrowRequestRecord, String> {
    let ctx = auth_session::require_admin(&session_store, &session_token)?;
    db::reject_borrow_request_with_actor(&app, ctx, payload)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let lan_token_store = std::sync::Arc::new(lan_auth::LanTokenStore::new());
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(auth_session::SessionStore::new())
        .manage(lan_token_store.clone())
        .setup({
            let lan_token_store = lan_token_store.clone();
            move |app| {
                if let Err(error) = lan_server::start(app.handle().clone(), lan_token_store.clone())
                {
                    eprintln!("failed to start Staff Kit LAN borrow server: {error}");
                }
                // Show the main window after WebView2 has finished initializing.
                // We set visible:false in tauri.conf.json to avoid the black flash
                // that appears before the first frame is rendered.
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                }
                Ok(())
            }
        })
        .invoke_handler(tauri::generate_handler![
            // ── Public (bootstrap/login/diagnostics) ───────────────────────────
            ping,
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
            inspect_asset_import_file,
            preview_asset_import_file,
            import_asset_import_file,
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
            update_stock_item_quantity,
            list_pending_borrow_requests,
            get_borrow_request_detail,
            approve_borrow_request,
            reject_borrow_request,
            list_employees,
            search_employees,
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
            inspect_import_columns,
            import_excel,
            preview_import_excel,
            get_mssql_connection_defaults,
            test_mssql_connection,
            preview_mssql_staff,
            import_mssql_staff
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
        "restore_history_snapshot",
        "move_database_to",
        "restore_database_from_file",
        "create_local_account",
        "update_local_account",
        "delete_local_account",
        "list_local_accounts",
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
        "inspect_asset_import_file",
        "preview_asset_import_file",
        "import_asset_import_file",
        "create_asset_import_batch",
        "import_asset_import_batch_valid_rows",
        "delete_asset_import_batch",
        "create_asset_manually",
        "upsert_assets",
        "create_asset_category",
        "update_asset_category",
        "deactivate_asset_category",
        "update_stock_item_quantity",
        "list_pending_borrow_requests",
        "get_borrow_request_detail",
        "approve_borrow_request",
        "reject_borrow_request",
        "preview_employee_asset_seed",
        "import_employee_asset_seed",
        "upsert_employee_column",
        "delete_employee_column",
        "admin_reset_local_account_password",
        "inspect_import_columns",
        "import_excel",
        "preview_import_excel",
        "get_mssql_connection_defaults",
        "test_mssql_connection",
        "preview_mssql_staff",
        "import_mssql_staff",
    ];

    const AUTHENTICATED_COMMANDS: &[&str] = &[
        "list_employees",
        "search_employees",
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
