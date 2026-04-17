mod db;
mod lan_assets;
mod lan_server;
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
fn preview_employee_asset_seed(
    app: tauri::AppHandle,
    payload: db::EmployeeAssetSeedInput,
) -> Result<db::EmployeeAssetSeedPreview, String> {
    db::preview_employee_asset_seed(&app, payload)
}

#[tauri::command]
fn import_employee_asset_seed(
    app: tauri::AppHandle,
    payload: db::EmployeeAssetSeedInput,
) -> Result<db::EmployeeAssetSeedReport, String> {
    db::import_employee_asset_seed(&app, payload)
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
fn restore_history_snapshot(app: tauri::AppHandle, filename: String) -> Result<(), String> {
    db::restore_history_snapshot(&app, &filename)
}

#[tauri::command]
fn move_database_to(app: tauri::AppHandle, target_folder: String) -> Result<String, String> {
    db::move_database_to(&app, &target_folder)
}

#[tauri::command]
fn restore_database_from_file(app: tauri::AppHandle, source_path: String) -> Result<(), String> {
    db::restore_database_from_file(&app, &source_path)
}

#[tauri::command]
fn get_db_custom_path(app: tauri::AppHandle) -> Result<Option<String>, String> {
    db::get_db_custom_path(&app)
}

#[tauri::command]
fn get_borrow_lan_settings(app: tauri::AppHandle) -> Result<db::BorrowLanSettings, String> {
    db::get_borrow_lan_settings(&app)
}

#[tauri::command]
fn update_borrow_lan_settings(
    app: tauri::AppHandle,
    payload: db::BorrowLanSettingsUpdateInput,
) -> Result<db::BorrowLanSettings, String> {
    db::update_borrow_lan_settings(&app, payload)
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

#[tauri::command]
fn inspect_asset_import_file(
    payload: db::AssetImportInspectInput,
) -> Result<db::AssetImportFileInspection, String> {
    db::inspect_asset_import_file(payload)
}

#[tauri::command]
fn preview_asset_import_file(
    app: tauri::AppHandle,
    payload: db::AssetDirectImportInput,
) -> Result<db::AssetDirectImportPreview, String> {
    db::preview_asset_import_file(&app, payload)
}

#[tauri::command]
fn import_asset_import_file(
    app: tauri::AppHandle,
    payload: db::AssetDirectImportInput,
) -> Result<db::AssetDirectImportReport, String> {
    db::import_asset_import_file(&app, payload)
}

#[tauri::command]
fn create_asset_import_batch(
    app: tauri::AppHandle,
    payload: db::AssetImportBatchCreateInput,
) -> Result<db::AssetImportBatchDetail, String> {
    db::create_asset_import_batch(&app, payload)
}

#[tauri::command]
fn list_asset_import_batches(
    app: tauri::AppHandle,
) -> Result<Vec<db::AssetImportBatchSummary>, String> {
    db::list_asset_import_batches(&app)
}

#[tauri::command]
fn get_asset_import_batch_detail(
    app: tauri::AppHandle,
    batch_id: i64,
) -> Result<db::AssetImportBatchDetail, String> {
    db::get_asset_import_batch_detail(&app, batch_id)
}

#[tauri::command]
fn update_asset_import_row(
    app: tauri::AppHandle,
    payload: db::AssetImportRowUpdateInput,
) -> Result<db::AssetImportRowRecord, String> {
    db::update_asset_import_row(&app, payload)
}

#[tauri::command]
fn set_asset_import_row_skipped(
    app: tauri::AppHandle,
    payload: db::AssetImportRowSkipInput,
) -> Result<db::AssetImportRowRecord, String> {
    db::set_asset_import_row_skipped(&app, payload)
}

#[tauri::command]
fn import_asset_import_batch_valid_rows(
    app: tauri::AppHandle,
    batch_id: i64,
) -> Result<db::AssetImportCommitResult, String> {
    db::import_asset_import_batch_valid_rows(&app, batch_id)
}

#[tauri::command]
fn delete_asset_import_batch(app: tauri::AppHandle, batch_id: i64) -> Result<bool, String> {
    db::delete_asset_import_batch(&app, batch_id)
}

#[tauri::command]
fn create_asset_manually(
    app: tauri::AppHandle,
    payload: db::AssetUpsertInput,
) -> Result<db::AssetRecord, String> {
    db::create_asset_manually(&app, payload)
}

#[tauri::command]
fn upsert_assets(
    app: tauri::AppHandle,
    payload: Vec<db::AssetUpsertInput>,
) -> Result<Vec<db::AssetRecord>, String> {
    db::upsert_assets(&app, payload)
}

#[tauri::command]
fn list_asset_categories(app: tauri::AppHandle) -> Result<Vec<db::AssetCategoryRecord>, String> {
    db::list_asset_categories(&app)
}

#[tauri::command]
fn list_asset_category_details(
    app: tauri::AppHandle,
) -> Result<Vec<db::AssetCategoryDetailRecord>, String> {
    db::list_asset_category_details(&app)
}

#[tauri::command]
fn create_asset_category(
    app: tauri::AppHandle,
    payload: db::AssetCategoryUpsertInput,
) -> Result<db::AssetCategoryDetailRecord, String> {
    db::create_asset_category(&app, payload)
}

#[tauri::command]
fn update_asset_category(
    app: tauri::AppHandle,
    payload: db::AssetCategoryUpsertInput,
) -> Result<db::AssetCategoryDetailRecord, String> {
    db::update_asset_category(&app, payload)
}

#[tauri::command]
fn deactivate_asset_category(
    app: tauri::AppHandle,
    category_id: i64,
) -> Result<db::AssetCategoryDetailRecord, String> {
    db::deactivate_asset_category(&app, category_id)
}

#[tauri::command]
fn get_asset_dashboard_summary(
    app: tauri::AppHandle,
) -> Result<db::AssetDashboardSummary, String> {
    db::get_asset_dashboard_summary(&app)
}

#[tauri::command]
fn list_asset_dashboard_serialized(
    app: tauri::AppHandle,
) -> Result<Vec<db::AssetDashboardSerializedRecord>, String> {
    db::list_asset_dashboard_serialized(&app)
}

#[tauri::command]
fn list_asset_dashboard_quantity(
    app: tauri::AppHandle,
) -> Result<Vec<db::AssetDashboardQuantityRecord>, String> {
    db::list_asset_dashboard_quantity(&app)
}

#[tauri::command]
fn update_stock_item_quantity(
    app: tauri::AppHandle,
    payload: db::StockItemQuantityUpdateInput,
) -> Result<db::AssetDashboardQuantityRecord, String> {
    db::update_stock_item_quantity(&app, payload)
}

#[tauri::command]
fn list_pending_borrow_requests(
    app: tauri::AppHandle,
) -> Result<Vec<db::BorrowRequestRecord>, String> {
    db::list_pending_borrow_requests(&app)
}

#[tauri::command]
fn get_borrow_request_detail(
    app: tauri::AppHandle,
    request_id: i64,
) -> Result<db::BorrowRequestRecord, String> {
    db::get_borrow_request_detail(&app, request_id)
}

#[tauri::command]
fn approve_borrow_request(
    app: tauri::AppHandle,
    request_id: i64,
) -> Result<db::BorrowRequestRecord, String> {
    db::approve_borrow_request(&app, request_id)
}

#[tauri::command]
fn reject_borrow_request(
    app: tauri::AppHandle,
    payload: db::BorrowRequestRejectInput,
) -> Result<db::BorrowRequestRecord, String> {
    db::reject_borrow_request(&app, payload)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Err(error) = lan_server::start(app.handle().clone()) {
                eprintln!("failed to start Staff Kit LAN borrow server: {error}");
            }
            // Show the main window after WebView2 has finished initializing.
            // We set visible:false in tauri.conf.json to avoid the black flash
            // that appears before the first frame is rendered.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }
            Ok(())
        })
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
            restore_database_from_file,
            get_borrow_lan_settings,
            update_borrow_lan_settings,
            detect_borrow_lan_host,
            probe_lan_server,
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
