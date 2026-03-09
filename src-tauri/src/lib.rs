mod db;
use tauri::Manager;

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

#[tauri::command]
fn init_database(app: tauri::AppHandle) -> Result<db::DatabaseStatus, String> {
    db::init_database(&app)
}

#[tauri::command]
fn get_database_status(app: tauri::AppHandle) -> Result<db::DatabaseStatus, String> {
    db::database_status(&app)
}

#[tauri::command]
fn get_backup_settings(app: tauri::AppHandle) -> Result<db::BackupSettings, String> {
    db::get_backup_settings(&app)
}

#[tauri::command]
fn update_backup_settings(
    app: tauri::AppHandle,
    payload: db::BackupSettingsUpdateInput,
) -> Result<db::BackupSettings, String> {
    db::update_backup_settings(&app, payload)
}

#[tauri::command]
fn backup_database_now(app: tauri::AppHandle) -> Result<db::BackupRunResult, String> {
    db::backup_database_now(&app)
}

#[tauri::command]
fn run_auto_backup_if_due(app: tauri::AppHandle) -> Result<db::BackupRunResult, String> {
    db::run_auto_backup_if_due(&app)
}

#[tauri::command]
fn list_employees(
    app: tauri::AppHandle,
    filters: db::EmployeeQuery,
) -> Result<db::EmployeeListResponse, String> {
    db::list_employees(&app, filters)
}

#[tauri::command]
fn search_employees(
    app: tauri::AppHandle,
    filters: db::EmployeeQuery,
) -> Result<db::EmployeeListResponse, String> {
    db::search_employees(&app, filters)
}

#[tauri::command]
fn list_employee_group_counts(app: tauri::AppHandle) -> Result<db::EmployeeGroupCounts, String> {
    db::list_employee_group_counts(&app)
}

#[tauri::command]
fn list_local_accounts(app: tauri::AppHandle) -> Result<Vec<db::LocalAccountRecord>, String> {
    db::list_local_accounts(&app)
}

#[tauri::command]
fn create_local_account(
    app: tauri::AppHandle,
    payload: db::LocalAccountCreateInput,
) -> Result<db::LocalAccountRecord, String> {
    db::create_local_account(&app, payload)
}

#[tauri::command]
fn update_local_account(
    app: tauri::AppHandle,
    payload: db::LocalAccountUpdateInput,
) -> Result<db::LocalAccountRecord, String> {
    db::update_local_account(&app, payload)
}

#[tauri::command]
fn delete_local_account(app: tauri::AppHandle, id: i64) -> Result<bool, String> {
    db::delete_local_account(&app, id)
}

#[tauri::command]
fn set_active_local_account(app: tauri::AppHandle, id: i64) -> Result<bool, String> {
    db::set_active_local_account(&app, id)
}

#[tauri::command]
fn login_local_account(
    app: tauri::AppHandle,
    payload: db::LocalAccountLoginInput,
) -> Result<db::LocalAccountRecord, String> {
    db::login_local_account(&app, payload)
}

#[tauri::command]
fn change_local_account_password(
    app: tauri::AppHandle,
    payload: db::LocalPasswordChangeInput,
) -> Result<bool, String> {
    db::change_local_account_password(&app, payload)
}

#[tauri::command]
fn admin_reset_local_account_password(
    app: tauri::AppHandle,
    payload: db::LocalPasswordResetInput,
) -> Result<bool, String> {
    db::admin_reset_local_account_password(&app, payload)
}

#[tauri::command]
fn forgot_local_account_password(
    app: tauri::AppHandle,
    payload: db::LocalForgotPasswordInput,
) -> Result<bool, String> {
    db::forgot_local_account_password(&app, payload)
}

#[tauri::command]
fn list_employee_columns(
    app: tauri::AppHandle,
) -> Result<Vec<db::EmployeeColumnDefinition>, String> {
    db::list_employee_columns(&app)
}

#[tauri::command]
fn upsert_employee_column(
    app: tauri::AppHandle,
    payload: db::EmployeeColumnUpsertInput,
) -> Result<db::EmployeeColumnDefinition, String> {
    db::upsert_employee_column(&app, payload)
}

#[tauri::command]
fn delete_employee_column(app: tauri::AppHandle, key: String) -> Result<bool, String> {
    db::delete_employee_column(&app, key)
}

#[tauri::command]
fn create_employee(
    app: tauri::AppHandle,
    payload: db::EmployeePayload,
) -> Result<db::EmployeeRecord, String> {
    db::create_employee(&app, payload)
}

#[tauri::command]
fn update_employee(
    app: tauri::AppHandle,
    id: i64,
    payload: db::EmployeePayload,
) -> Result<db::EmployeeRecord, String> {
    db::update_employee(&app, id, payload)
}

#[tauri::command]
fn move_employees_group(
    app: tauri::AppHandle,
    payload: db::MoveEmployeesGroupInput,
) -> Result<i64, String> {
    db::move_employees_group(&app, payload)
}

#[tauri::command]
fn delete_employee(app: tauri::AppHandle, id: i64) -> Result<bool, String> {
    db::delete_employee(&app, id)
}

#[tauri::command]
fn list_teams(app: tauri::AppHandle) -> Result<Vec<db::TeamRecord>, String> {
    db::list_teams(&app)
}

#[tauri::command]
fn upsert_team(
    app: tauri::AppHandle,
    payload: db::TeamUpsertInput,
) -> Result<db::TeamRecord, String> {
    db::upsert_team(&app, payload)
}

#[tauri::command]
fn delete_team(app: tauri::AppHandle, id: i64) -> Result<bool, String> {
    db::delete_team(&app, id)
}

#[tauri::command]
fn reset_all_data(app: tauri::AppHandle) -> Result<bool, String> {
    db::reset_all_data(&app)
}

#[tauri::command]
fn import_excel(
    app: tauri::AppHandle,
    payload: db::ImportExcelInput,
) -> Result<db::ImportReport, String> {
    // Snapshot before import so user can rollback if needed
    let _ = db::create_history_snapshot(&app, "before_import");
    db::import_excel(&app, payload)
}

#[tauri::command]
fn preview_import_excel(
    app: tauri::AppHandle,
    payload: db::ImportExcelInput,
) -> Result<db::ImportPreviewResult, String> {
    db::preview_import_excel(&app, payload)
}

#[tauri::command]
fn inspect_import_columns(
    app: tauri::AppHandle,
    payload: db::ImportExcelInput,
) -> Result<db::ImportColumnsPreview, String> {
    db::inspect_import_columns(&app, payload)
}

#[tauri::command]
fn list_history_snapshots(app: tauri::AppHandle) -> Result<Vec<db::SnapshotInfo>, String> {
    db::list_history_snapshots(&app)
}

#[tauri::command]
fn create_history_snapshot_cmd(
    app: tauri::AppHandle,
    label: String,
) -> Result<db::SnapshotInfo, String> {
    db::create_history_snapshot(&app, &label)
}

#[tauri::command]
fn restore_history_snapshot(
    app: tauri::AppHandle,
    filename: String,
) -> Result<(), String> {
    db::restore_history_snapshot(&app, &filename)
}

#[tauri::command]
fn move_database_to(
    app: tauri::AppHandle,
    target_folder: String,
) -> Result<String, String> {
    db::move_database_to(&app, &target_folder)
}

#[tauri::command]
fn get_db_custom_path(app: tauri::AppHandle) -> Result<Option<String>, String> {
    db::get_db_custom_path(&app)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            ping,
            init_database,
            get_database_status,
            get_backup_settings,
            update_backup_settings,
            backup_database_now,
            run_auto_backup_if_due,
            list_history_snapshots,
            create_history_snapshot_cmd,
            restore_history_snapshot,
            move_database_to,
            get_db_custom_path,
            list_employees,
            search_employees,
            list_employee_group_counts,
            list_local_accounts,
            create_local_account,
            update_local_account,
            delete_local_account,
            set_active_local_account,
            login_local_account,
            change_local_account_password,
            admin_reset_local_account_password,
            forgot_local_account_password,
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
            preview_import_excel
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let app = window.app_handle().clone();
                // Best-effort snapshot on close; do not block close if it fails
                let _ = db::create_history_snapshot(&app, "app_close");
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
