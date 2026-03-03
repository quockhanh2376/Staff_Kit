mod db;

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
    db::import_excel(&app, payload)
}

#[tauri::command]
fn inspect_import_columns(
    app: tauri::AppHandle,
    payload: db::ImportExcelInput,
) -> Result<db::ImportColumnsPreview, String> {
    db::inspect_import_columns(&app, payload)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            ping,
            init_database,
            get_database_status,
            list_employees,
            search_employees,
            list_employee_group_counts,
            list_local_accounts,
            create_local_account,
            update_local_account,
            delete_local_account,
            set_active_local_account,
            list_employee_columns,
            upsert_employee_column,
            delete_employee_column,
            create_employee,
            update_employee,
            delete_employee,
            list_teams,
            upsert_team,
            delete_team,
            reset_all_data,
            inspect_import_columns,
            import_excel
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
