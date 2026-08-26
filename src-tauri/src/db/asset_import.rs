use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use calamine::{open_workbook_auto, Data, DataType, Reader};
use csv::{ReaderBuilder, StringRecord};
use rand::{distributions::Alphanumeric, Rng};
use rusqlite::{params, types::Type, Connection, OptionalExtension, Transaction};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;

use super::asset::{self, AssetRecord, AssetUpsertInput};
use super::{audit, auth, humanize_sqlite_error, normalize_optional_text, open_runtime_connection};

const BATCH_STATUS_PENDING_REVIEW: &str = "pending_review";
const BATCH_STATUS_COMPLETED: &str = "completed";
const ROW_STATUS_VALID: &str = "valid";
const ROW_STATUS_ERROR: &str = "error";
const ROW_STATUS_IMPORTED: &str = "imported";
const ROW_STATUS_SKIPPED: &str = "skipped";
const OWNER_MATCH_NOT_APPLICABLE: &str = "not_applicable";
const OWNER_MATCH_MATCHED: &str = "matched";
const OWNER_MATCH_WARNING: &str = "warning";
const OWNER_MATCH_UNRESOLVED: &str = "unresolved";
const IMPORT_BORROW_REQUEST_STATUS_APPROVED: &str = "approved";
const IMPORT_BORROW_REQUEST_TYPE_BORROW: &str = "borrow";
const IMPORT_ASSET_STATUS_ASSIGNED: &str = "assigned";

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetImportMode {
    #[default]
    Serialized,
    Quantity,
}

impl AssetImportMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Serialized => "serialized",
            Self::Quantity => "quantity",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "serialized" => Ok(Self::Serialized),
            "quantity" => Ok(Self::Quantity),
            other => Err(format!("unsupported asset import mode '{other}'")),
        }
    }
}

const ASSET_CODE_ALIASES: &[&str] = &[
    "assetcode",
    "code",
    "assettag",
    "assetnumber",
    "mataisan",
    "mats",
];
const ASSET_TYPE_ALIASES: &[&str] = &[
    "assettype",
    "type",
    "loaitaisan",
    "category",
    "cateagory",
    "assetcategory",
];
const DISPLAY_NAME_PRIMARY_ALIASES: &[&str] =
    &["displayname", "assetname", "tentaisan", "description"];
const DISPLAY_NAME_FALLBACK_ALIASES: &[&str] = &["name", "itemname"];
const COMPUTER_NAME_ALIASES: &[&str] = &["computername", "computer", "pcname", "hostname"];
const MODEL_ALIASES: &[&str] = &["model", "modelnumber", "modelno"];
const SERIAL_NUMBER_ALIASES: &[&str] = &[
    "serialnumber",
    "serrialnumber",
    "serial",
    "serialno",
    "serialnum",
    "sn",
];
const ADAPTER_NUMBER_ALIASES: &[&str] = &["adapternumber", "adapter", "adapterno", "adapterserial"];
const BRAND_ALIASES: &[&str] = &["brand", "maker", "vendor", "nhanhieu", "nhanhieu"];
const QUANTITY_ALIASES: &[&str] = &["quantity", "qty", "soluong", "solg"];
const WAREHOUSE_ALIASES: &[&str] = &["warehouse", "location", "stocklocation", "kho"];
const USAGE_LOCATION_ALIASES: &[&str] = &[
    "usagelocation",
    "usuagelocation",
    "location",
    "usageplace",
    "workinglocation",
];
const NOTES_ALIASES: &[&str] = &["notes", "note", "remark", "remarks", "ghichu"];
const OWNER_STAFF_ID_ALIASES: &[&str] =
    &["staffid", "eeid", "employeeid", "mãnhânviên", "manhanvien"];
const OWNER_FULL_NAME_ALIASES: &[&str] = &[
    "tênnhânviên",
    "tennhanvien",
    "vietnamesename",
    "fullname",
    "name",
    "hoten",
    "họtên",
];
const OWNER_TEAM_ALIASES: &[&str] = &["team", "client", "clientpmd", "client(pmd)"];
const OWNER_PHONE_NUMBER_ALIASES: &[&str] = &["phonenumber", "phone", "cellphone", "mobilenumber"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportRawValue {
    pub header: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportFieldMapping {
    pub asset_code: Option<String>,
    pub asset_type: Option<String>,
    pub display_name: Option<String>,
    pub computer_name: Option<String>,
    pub usage_location: Option<String>,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub adapter_number: Option<String>,
    pub quantity: Option<String>,
    pub warehouse: Option<String>,
    pub notes: Option<String>,
}

pub(crate) struct AssetHeaderEvidence {
    pub header_row: usize,
    pub mapping: AssetImportFieldMapping,
    pub explicit_display_name: bool,
    pub legacy_asset_id_header: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportInspectInput {
    pub file_path: String,
    pub sheet_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportFileInspection {
    pub file_name: String,
    pub file_type: String,
    pub selected_sheet_name: Option<String>,
    pub available_sheets: Vec<String>,
    pub header_row: i64,
    pub headers: Vec<String>,
    pub mapping: AssetImportFieldMapping,
    pub requires_manual_mapping: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportBatchCreateInput {
    #[serde(default)]
    pub import_type: AssetImportMode,
    pub file_path: String,
    pub sheet_name: Option<String>,
    pub mapping: Option<AssetImportFieldMapping>,
}

#[derive(Debug, Clone)]
pub struct AssetImportBatchSeedInput {
    pub import_type: AssetImportMode,
    pub source_file_name: String,
    pub source_file_path: String,
    pub source_file_type: String,
    pub sheet_name: Option<String>,
    pub header_row: i64,
    pub headers: Vec<String>,
    pub mapping: AssetImportFieldMapping,
    pub rows: Vec<AssetImportRowSeedInput>,
}

#[derive(Debug, Clone)]
pub struct AssetImportRowSeedInput {
    pub row_number: i64,
    pub raw_values: Vec<AssetImportRawValue>,
    pub asset_code: Option<String>,
    pub asset_type: Option<String>,
    pub display_name: Option<String>,
    pub display_name_short: Option<String>,
    pub computer_name: Option<String>,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub adapter_number: Option<String>,
    pub quantity: Option<String>,
    pub warehouse: Option<String>,
    pub usage_location: Option<String>,
    pub notes: Option<String>,
    pub submitted_staff_id: Option<String>,
    pub submitted_full_name: Option<String>,
    pub submitted_team: Option<String>,
    pub submitted_phone_number: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportBatchSummary {
    pub id: i64,
    pub batch_key: String,
    pub import_type: AssetImportMode,
    pub source_file_name: String,
    pub source_file_path: String,
    pub source_file_type: String,
    pub sheet_name: Option<String>,
    pub header_row: i64,
    pub status: String,
    pub total_rows: i64,
    pub valid_rows: i64,
    pub error_rows: i64,
    pub imported_rows: i64,
    pub skipped_rows: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportRowRecord {
    pub id: i64,
    pub batch_id: i64,
    pub row_number: i64,
    pub raw_values: Vec<AssetImportRawValue>,
    pub asset_code: Option<String>,
    pub asset_type: Option<String>,
    pub display_name: Option<String>,
    pub display_name_short: Option<String>,
    pub computer_name: Option<String>,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub adapter_number: Option<String>,
    pub quantity: Option<String>,
    pub warehouse: Option<String>,
    pub usage_location: Option<String>,
    pub notes: Option<String>,
    pub submitted_staff_id: Option<String>,
    pub submitted_full_name: Option<String>,
    pub submitted_team: Option<String>,
    pub submitted_phone_number: Option<String>,
    pub resolved_employee_id: Option<String>,
    pub resolved_employee_row_id: Option<i64>,
    pub resolved_full_name: Option<String>,
    pub resolved_team_name: Option<String>,
    pub owner_match_status: String,
    pub owner_warnings: Vec<String>,
    pub validation_errors: Vec<String>,
    pub status: String,
    pub is_edited: bool,
    pub edited_fields: Vec<String>,
    pub imported_asset_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportBatchDetail {
    #[serde(flatten)]
    pub summary: AssetImportBatchSummary,
    pub headers: Vec<String>,
    pub mapping: AssetImportFieldMapping,
    pub rows: Vec<AssetImportRowRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportRowUpdateInput {
    pub row_id: i64,
    pub field_key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportRowSkipInput {
    pub row_id: i64,
    pub skipped: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportCommitResult {
    pub batch_id: i64,
    pub imported_row_ids: Vec<i64>,
    pub imported_asset_codes: Vec<String>,
    pub imported_count: i64,
    pub remaining_error_rows: i64,
    pub batch_status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDirectImportInput {
    #[serde(default)]
    pub import_type: AssetImportMode,
    pub file_path: String,
    pub sheet_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDirectImportErrorItem {
    pub row_number: i64,
    pub entity_key: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDirectImportPreviewRow {
    pub row_number: i64,
    pub asset_code: Option<String>,
    pub asset_type: Option<String>,
    pub computer_name: Option<String>,
    pub display_name: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub adapter_number: Option<String>,
    pub quantity: Option<String>,
    pub usage_location: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub holder_label: Option<String>,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDirectImportPreview {
    pub file_name: String,
    pub sheet_name: Option<String>,
    pub import_type: AssetImportMode,
    pub total_rows: i64,
    pub valid_rows: i64,
    pub error_rows: i64,
    pub skipped_rows: i64,
    pub rows: Vec<AssetDirectImportPreviewRow>,
    pub errors: Vec<AssetDirectImportErrorItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDirectImportReport {
    pub file_name: String,
    pub sheet_name: Option<String>,
    pub import_type: AssetImportMode,
    pub total_rows: i64,
    pub imported: i64,
    pub skipped: i64,
    pub failed: i64,
    pub imported_asset_codes: Vec<String>,
    pub errors: Vec<AssetDirectImportErrorItem>,
}

#[derive(Debug, Clone)]
struct AssetImportResolvedMapping {
    mapping: AssetImportFieldMapping,
    asset_code_index: Option<usize>,
    asset_type_index: usize,
    display_name_index: usize,
    computer_name_index: Option<usize>,
    usage_location_index: Option<usize>,
    brand_index: Option<usize>,
    model_index: Option<usize>,
    serial_number_index: Option<usize>,
    adapter_number_index: Option<usize>,
    quantity_index: Option<usize>,
    warehouse_index: Option<usize>,
    notes_index: Option<usize>,
}

#[derive(Debug, Clone)]
struct ParsedAssetImportSource {
    source_file_name: String,
    source_file_path: String,
    source_file_type: String,
    sheet_name: Option<String>,
    header_row: i64,
    available_sheets: Vec<String>,
    headers: Vec<String>,
    auto_mapping: AssetImportFieldMapping,
    rows: Vec<AssetImportRowSeedInput>,
}

#[derive(Debug, Clone)]
struct AssetImportRowState {
    id: i64,
    status: String,
    import_type: AssetImportMode,
    asset_code: Option<String>,
    asset_type: Option<String>,
    display_name: Option<String>,
    quantity: Option<String>,
    serial_number: Option<String>,
    submitted_staff_id: Option<String>,
    submitted_full_name: Option<String>,
    submitted_team: Option<String>,
}

#[derive(Debug, Clone)]
struct ExistingAssetIdentity {
    id: i64,
    serial_number: Option<String>,
}

#[derive(Debug, Default)]
struct ExistingAssetIdentities {
    by_code: HashMap<String, ExistingAssetIdentity>,
    by_serial: HashMap<String, ExistingAssetIdentity>,
}

#[derive(Debug, PartialEq, Eq)]
enum SerializedAssetImportClassification {
    New,
    Existing { asset_id: i64 },
    Conflict(String),
}

#[derive(Debug, Clone)]
struct AssetImportBatchRowMeta {
    batch_id: i64,
    status: String,
    edited_fields: Vec<String>,
}

#[derive(Debug, Clone)]
struct EmployeeOwnerLookup {
    row_id: i64,
    employee_id: String,
    full_name: String,
    team_name: Option<String>,
}

#[derive(Debug, Clone)]
struct OwnerResolutionState {
    resolved_employee_id: Option<String>,
    resolved_employee_row_id: Option<i64>,
    resolved_full_name: Option<String>,
    resolved_team_name: Option<String>,
    owner_match_status: String,
    owner_warnings: Vec<String>,
    blocking_error: Option<String>,
}

pub fn inspect_asset_import_file(
    payload: AssetImportInspectInput,
) -> Result<AssetImportFileInspection, String> {
    let file_path = resolve_import_file_path(payload.file_path)?;
    let inspection =
        inspect_asset_import_source(file_path.as_path(), payload.sheet_name.as_deref())?;

    Ok(AssetImportFileInspection {
        file_name: inspection.source_file_name,
        file_type: inspection.source_file_type,
        selected_sheet_name: inspection.sheet_name,
        available_sheets: inspection.available_sheets,
        header_row: inspection.header_row,
        headers: inspection.headers,
        requires_manual_mapping: mapping_missing_required_fields(
            AssetImportMode::Serialized,
            &inspection.auto_mapping,
        )
        .is_some(),
        mapping: inspection.auto_mapping,
    })
}

pub fn create_asset_import_batch(
    app: &AppHandle,
    payload: AssetImportBatchCreateInput,
) -> Result<AssetImportBatchDetail, String> {
    let file_path = resolve_import_file_path(payload.file_path)?;
    let parsed = parse_asset_import_source(
        file_path.as_path(),
        payload.import_type,
        payload.sheet_name.as_deref(),
        payload.mapping,
    )?;

    let mut conn = open_runtime_connection(app)?;
    create_asset_import_batch_seed_conn(
        &mut conn,
        AssetImportBatchSeedInput {
            import_type: payload.import_type,
            source_file_name: parsed.source_file_name,
            source_file_path: parsed.source_file_path,
            source_file_type: parsed.source_file_type,
            sheet_name: parsed.sheet_name,
            header_row: parsed.header_row,
            headers: parsed.headers,
            mapping: parsed.auto_mapping,
            rows: parsed.rows,
        },
    )
}

pub fn preview_asset_import_file(
    app: &AppHandle,
    payload: AssetDirectImportInput,
) -> Result<AssetDirectImportPreview, String> {
    let file_path = resolve_import_file_path(payload.file_path)?;
    let parsed = parse_asset_import_source(
        file_path.as_path(),
        payload.import_type,
        payload.sheet_name.as_deref(),
        None,
    )?;

    let mut conn = open_runtime_connection(app)?;
    preview_asset_import_seed_conn(
        &mut conn,
        asset_import_seed_from_parsed(parsed, payload.import_type),
    )
}

pub fn import_asset_import_file(
    app: &AppHandle,
    payload: AssetDirectImportInput,
) -> Result<AssetDirectImportReport, String> {
    let file_path = resolve_import_file_path(payload.file_path)?;
    let parsed = parse_asset_import_source(
        file_path.as_path(),
        payload.import_type,
        payload.sheet_name.as_deref(),
        None,
    )?;

    let mut conn = open_runtime_connection(app)?;
    import_asset_import_seed_conn(
        &mut conn,
        asset_import_seed_from_parsed(parsed, payload.import_type),
    )
}

pub fn list_asset_import_batches(app: &AppHandle) -> Result<Vec<AssetImportBatchSummary>, String> {
    let conn = open_runtime_connection(app)?;
    list_asset_import_batches_conn(&conn)
}

pub fn get_asset_import_batch_detail(
    app: &AppHandle,
    batch_id: i64,
) -> Result<AssetImportBatchDetail, String> {
    let conn = open_runtime_connection(app)?;
    load_asset_import_batch_detail_conn(&conn, batch_id)
}

pub fn update_asset_import_row(
    app: &AppHandle,
    payload: AssetImportRowUpdateInput,
) -> Result<AssetImportRowRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    update_asset_import_row_conn(&mut conn, payload)
}

pub fn set_asset_import_row_skipped(
    app: &AppHandle,
    payload: AssetImportRowSkipInput,
) -> Result<AssetImportRowRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    set_asset_import_row_skipped_conn(&mut conn, payload)
}

pub fn import_asset_import_batch_valid_rows(
    app: &AppHandle,
    batch_id: i64,
) -> Result<AssetImportCommitResult, String> {
    let mut conn = open_runtime_connection(app)?;
    import_asset_import_batch_valid_rows_conn(&mut conn, batch_id)
}

pub fn delete_asset_import_batch(app: &AppHandle, batch_id: i64) -> Result<bool, String> {
    let mut conn = open_runtime_connection(app)?;
    delete_asset_import_batch_conn(&mut conn, batch_id)
}

pub fn create_asset_manually(
    app: &AppHandle,
    payload: AssetUpsertInput,
) -> Result<AssetRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    let actor_ref = active_actor_ref(&conn)?;
    let record = asset::create_asset_conn(&mut conn, payload)?;

    let payload_json = json!({
        "assetCode": record.asset_code,
        "assetType": record.asset_type,
        "displayName": record.display_name,
        "model": record.model,
        "serialNumber": record.serial_number,
        "notes": record.notes,
    })
    .to_string();

    audit::insert_audit_log_conn(
        &conn,
        "asset.create_manual",
        "local_account",
        actor_ref.as_deref(),
        "asset",
        record.id.to_string().as_str(),
        Some(payload_json.as_str()),
    )?;

    Ok(record)
}

pub(crate) fn create_asset_import_batch_seed_conn(
    conn: &mut Connection,
    input: AssetImportBatchSeedInput,
) -> Result<AssetImportBatchDetail, String> {
    if input.rows.is_empty() {
        return Err("no asset rows were found in the selected file".to_string());
    }

    let actor_ref = active_actor_ref(conn)?;
    let total_rows = input.rows.len() as i64;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start asset import batch transaction: {err}"))?;

    let batch_key = generate_batch_key_tx(&tx)?;
    tx.execute(
        r#"
        INSERT INTO asset_import_batches(
          batch_key,
          import_type,
          source_file_name,
          source_file_path,
          source_file_type,
          sheet_name,
          header_row,
          headers_json,
          mapping_json,
          status,
          created_at,
          updated_at
        )
        VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        "#,
        params![
            batch_key.as_str(),
            input.import_type.as_str(),
            input.source_file_name.as_str(),
            input.source_file_path.as_str(),
            input.source_file_type.as_str(),
            input.sheet_name.as_deref(),
            input.header_row,
            to_json(&input.headers)?,
            to_json(&input.mapping)?,
            BATCH_STATUS_PENDING_REVIEW,
        ],
    )
    .map_err(humanize_sqlite_error)?;

    let batch_id = tx.last_insert_rowid();

    for row in input.rows {
        tx.execute(
            r#"
            INSERT INTO asset_import_rows(
              batch_id,
              row_number,
              raw_row_json,
              asset_code,
              asset_type,
              display_name,
              display_name_short,
              computer_name,
              brand,
              model,
              serial_number,
              adapter_number,
              quantity,
              warehouse,
              usage_location,
              notes,
              submitted_staff_id,
              submitted_full_name,
              submitted_team,
              submitted_phone_number,
              resolved_employee_id,
              resolved_employee_row_id,
              resolved_full_name,
              resolved_team_name,
              owner_match_status,
              owner_warnings_json,
              validation_errors_json,
              status,
              edited,
              edited_fields_json,
              created_at,
              updated_at
            )
            VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, NULL, ?, '[]', '[]', ?, 0, '[]', datetime('now'), datetime('now'))
            "#,
            params![
                batch_id,
                row.row_number,
                to_json(&row.raw_values)?,
                normalize_asset_code(row.asset_code),
                normalize_optional_asset_text(row.asset_type),
                normalize_optional_asset_text(row.display_name),
                normalize_optional_asset_text(row.display_name_short),
                normalize_optional_asset_text(row.computer_name),
                normalize_optional_asset_text(row.brand),
                normalize_optional_asset_text(row.model),
                normalize_optional_asset_text(row.serial_number),
                normalize_optional_asset_text(row.adapter_number),
                normalize_optional_quantity_text(row.quantity),
                normalize_optional_asset_text(row.warehouse),
                normalize_usage_location(row.usage_location),
                normalize_optional_asset_text(row.notes),
                normalize_optional_asset_text(row.submitted_staff_id),
                normalize_optional_asset_text(row.submitted_full_name),
                normalize_optional_asset_text(row.submitted_team),
                normalize_optional_asset_text(row.submitted_phone_number),
                OWNER_MATCH_NOT_APPLICABLE,
                ROW_STATUS_VALID,
            ],
        )
        .map_err(humanize_sqlite_error)?;
    }

    revalidate_batch_tx(&tx, batch_id)?;

    let payload_json = json!({
        "batchKey": batch_key,
        "importType": input.import_type.as_str(),
        "sourceFileName": input.source_file_name,
        "sourceFileType": input.source_file_type,
        "sheetName": input.sheet_name,
        "totalRows": total_rows,
    })
    .to_string();

    audit::insert_audit_log_tx(
        &tx,
        "asset_import.create_batch",
        "local_account",
        actor_ref.as_deref(),
        "asset_import_batch",
        batch_id.to_string().as_str(),
        Some(payload_json.as_str()),
    )?;

    tx.commit()
        .map_err(|err| format!("failed to commit asset import batch transaction: {err}"))?;

    load_asset_import_batch_detail_conn(conn, batch_id)
}

pub(crate) fn list_asset_import_batches_conn(
    conn: &Connection,
) -> Result<Vec<AssetImportBatchSummary>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              id,
              batch_key,
              import_type,
              source_file_name,
              source_file_path,
              source_file_type,
              sheet_name,
              header_row,
              status,
              total_rows,
              valid_rows,
              error_rows,
              imported_rows,
              skipped_rows,
              created_at,
              updated_at
            FROM asset_import_batches
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .map_err(|err| format!("failed to prepare asset import batch list query: {err}"))?;

    let rows = stmt
        .query_map([], |row| map_batch_summary(row))
        .map_err(|err| format!("failed to query asset import batch list: {err}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|err| format!("failed to read asset import batch row: {err}"))?);
    }

    Ok(items)
}

pub(crate) fn load_asset_import_batch_detail_conn(
    conn: &Connection,
    batch_id: i64,
) -> Result<AssetImportBatchDetail, String> {
    let summary = conn
        .query_row(
            r#"
            SELECT
              id,
              batch_key,
              import_type,
              source_file_name,
              source_file_path,
              source_file_type,
              sheet_name,
              header_row,
              status,
              total_rows,
              valid_rows,
              error_rows,
              imported_rows,
              skipped_rows,
              created_at,
              updated_at,
              headers_json,
              mapping_json
            FROM asset_import_batches
            WHERE id = ?
            "#,
            params![batch_id],
            |row| {
                Ok((
                    map_batch_summary(row)?,
                    row.get::<_, String>(16)?,
                    row.get::<_, String>(17)?,
                ))
            },
        )
        .optional()
        .map_err(|err| format!("failed to load asset import batch detail: {err}"))?
        .ok_or_else(|| format!("asset import batch with id {batch_id} was not found"))?;

    let (summary, headers_json, mapping_json) = summary;
    let headers = from_json::<Vec<String>>(Some(headers_json))?;
    let mapping = from_json::<AssetImportFieldMapping>(Some(mapping_json))?;
    let rows = load_asset_import_rows_for_batch(conn, batch_id)?;

    Ok(AssetImportBatchDetail {
        summary,
        headers,
        mapping,
        rows,
    })
}

pub(crate) fn update_asset_import_row_conn(
    conn: &mut Connection,
    payload: AssetImportRowUpdateInput,
) -> Result<AssetImportRowRecord, String> {
    let field_key = normalize_import_field_key(payload.field_key.as_str())?;
    let actor_ref = active_actor_ref(conn)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start asset import row update transaction: {err}"))?;

    let row_meta = load_asset_import_row_meta_tx(&tx, payload.row_id)?;
    if row_meta.status == ROW_STATUS_IMPORTED {
        return Err("imported rows are read-only".to_string());
    }

    let mut edited_fields = row_meta.edited_fields;
    if !edited_fields.iter().any(|item| item == field_key.as_str()) {
        edited_fields.push(field_key.clone());
    }

    let normalized_value = match field_key.as_str() {
        "assetCode" => normalize_asset_code(payload.value),
        "submittedStaffId" => normalize_submitted_staff_id(payload.value),
        "usageLocation" => normalize_usage_location(payload.value),
        "quantity" => normalize_optional_quantity_text(payload.value),
        _ => normalize_optional_asset_text(payload.value),
    };

    let column_name = match field_key.as_str() {
        "assetCode" => "asset_code",
        "assetType" => "asset_type",
        "displayName" => "display_name",
        "displayNameShort" => "display_name_short",
        "computerName" => "computer_name",
        "brand" => "brand",
        "model" => "model",
        "serialNumber" => "serial_number",
        "adapterNumber" => "adapter_number",
        "quantity" => "quantity",
        "warehouse" => "warehouse",
        "usageLocation" => "usage_location",
        "notes" => "notes",
        "submittedStaffId" => "submitted_staff_id",
        "submittedFullName" => "submitted_full_name",
        "submittedTeam" => "submitted_team",
        "submittedPhoneNumber" => "submitted_phone_number",
        _ => return Err("unsupported asset import field".to_string()),
    };

    tx.execute(
        &format!(
            "UPDATE asset_import_rows SET {column_name} = ?, edited = 1, edited_fields_json = ?, updated_at = datetime('now') WHERE id = ?"
        ),
        params![normalized_value, to_json(&edited_fields)?, payload.row_id],
    )
    .map_err(humanize_sqlite_error)?;

    revalidate_batch_tx(&tx, row_meta.batch_id)?;

    let payload_json = json!({
        "rowId": payload.row_id,
        "fieldKey": field_key,
    })
    .to_string();

    audit::insert_audit_log_tx(
        &tx,
        "asset_import.update_row",
        "local_account",
        actor_ref.as_deref(),
        "asset_import_batch",
        row_meta.batch_id.to_string().as_str(),
        Some(payload_json.as_str()),
    )?;

    tx.commit()
        .map_err(|err| format!("failed to commit asset import row update: {err}"))?;

    load_asset_import_row_record_conn(conn, payload.row_id)
}

pub(crate) fn set_asset_import_row_skipped_conn(
    conn: &mut Connection,
    payload: AssetImportRowSkipInput,
) -> Result<AssetImportRowRecord, String> {
    let actor_ref = active_actor_ref(conn)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start asset import row skip transaction: {err}"))?;

    let row_meta = load_asset_import_row_meta_tx(&tx, payload.row_id)?;
    if row_meta.status == ROW_STATUS_IMPORTED {
        return Err("imported rows are read-only".to_string());
    }

    let next_status = if payload.skipped {
        ROW_STATUS_SKIPPED
    } else {
        ROW_STATUS_ERROR
    };

    tx.execute(
        "UPDATE asset_import_rows SET status = ?, updated_at = datetime('now') WHERE id = ?",
        params![next_status, payload.row_id],
    )
    .map_err(humanize_sqlite_error)?;

    revalidate_batch_tx(&tx, row_meta.batch_id)?;

    let payload_json = json!({
        "rowId": payload.row_id,
        "skipped": payload.skipped,
    })
    .to_string();

    audit::insert_audit_log_tx(
        &tx,
        "asset_import.set_row_skipped",
        "local_account",
        actor_ref.as_deref(),
        "asset_import_batch",
        row_meta.batch_id.to_string().as_str(),
        Some(payload_json.as_str()),
    )?;

    tx.commit()
        .map_err(|err| format!("failed to commit asset import row skip transaction: {err}"))?;

    load_asset_import_row_record_conn(conn, payload.row_id)
}

pub(crate) fn import_asset_import_batch_valid_rows_conn(
    conn: &mut Connection,
    batch_id: i64,
) -> Result<AssetImportCommitResult, String> {
    let actor_ref = active_actor_ref(conn)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start asset import commit transaction: {err}"))?;
    let reviewer_account_id = auth::get_active_local_account_id(&tx)?;

    revalidate_batch_tx(&tx, batch_id)?;
    let summary = load_batch_summary_tx(&tx, batch_id)?;

    let mut imported_row_ids = Vec::new();
    let mut imported_asset_codes = Vec::new();

    if summary.import_type == AssetImportMode::Quantity {
        let mut stmt = tx
            .prepare(
                r#"
                SELECT
                  id,
                  asset_type,
                  display_name,
                  brand,
                  model,
                  quantity,
                  warehouse,
                  notes
                FROM asset_import_rows
                WHERE batch_id = ? AND status = ?
                ORDER BY row_number ASC, id ASC
                "#,
            )
            .map_err(|err| format!("failed to prepare valid quantity row query: {err}"))?;

        let rows = stmt
            .query_map(params![batch_id, ROW_STATUS_VALID], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|err| format!("failed to query valid quantity rows: {err}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("failed to read valid quantity rows: {err}"))?;

        drop(stmt);

        let mut imported_stock_item_ids = Vec::new();

        for (row_id, asset_type, display_name, brand, model, quantity, warehouse, notes) in rows {
            let category_value = asset_type
                .clone()
                .ok_or_else(|| format!("row {row_id} is missing assetType"))?;
            let category =
                asset::load_asset_category_by_code_or_name_tx(&tx, category_value.as_str())?
                    .ok_or_else(|| format!("asset category '{category_value}' was not found"))?;

            if category.tracking_mode != AssetImportMode::Quantity.as_str() {
                return Err(format!(
                    "asset category '{}' is not configured for quantity stock",
                    category_value
                ));
            }

            let item_name = display_name
                .clone()
                .ok_or_else(|| format!("row {row_id} is missing displayName"))?;
            let quantity_on_hand = quantity
                .as_deref()
                .ok_or_else(|| format!("row {row_id} is missing quantity"))?
                .parse::<i64>()
                .map_err(|_| format!("row {row_id} has invalid quantity"))?;

            let stock_item_id = asset::create_stock_item_tx(
                &tx,
                &asset::StockItemCreateInput {
                    category_id: category.id,
                    item_name,
                    brand,
                    model,
                    warehouse,
                    quantity_on_hand,
                    note: notes,
                },
            )?;

            tx.execute(
                r#"
                UPDATE asset_import_rows
                SET
                  status = ?,
                  imported_asset_id = NULL,
                  validation_errors_json = '[]',
                  updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![ROW_STATUS_IMPORTED, row_id],
            )
            .map_err(humanize_sqlite_error)?;

            imported_row_ids.push(row_id);
            imported_stock_item_ids.push(stock_item_id);
        }

        revalidate_batch_tx(&tx, batch_id)?;
        let summary = load_batch_summary_tx(&tx, batch_id)?;

        let payload_json = json!({
            "importType": "quantity",
            "importedRowIds": imported_row_ids,
            "importedStockItemIds": imported_stock_item_ids,
        })
        .to_string();

        audit::insert_audit_log_tx(
            &tx,
            "asset_import.import_valid_rows",
            "local_account",
            actor_ref.as_deref(),
            "asset_import_batch",
            batch_id.to_string().as_str(),
            Some(payload_json.as_str()),
        )?;

        tx.commit()
            .map_err(|err| format!("failed to commit asset import transaction: {err}"))?;

        return Ok(AssetImportCommitResult {
            batch_id,
            imported_count: imported_row_ids.len() as i64,
            imported_row_ids,
            imported_asset_codes,
            remaining_error_rows: summary.error_rows,
            batch_status: summary.status,
        });
    }

    let mut stmt = tx
        .prepare(
            r#"
            SELECT
              id,
              asset_code,
              asset_type,
              display_name,
              display_name_short,
              computer_name,
              brand,
              model,
              serial_number,
              adapter_number,
              warehouse,
              usage_location,
              notes,
              submitted_staff_id,
              submitted_full_name,
              submitted_team,
              submitted_phone_number,
              resolved_employee_id,
              resolved_employee_row_id,
              resolved_full_name
            FROM asset_import_rows
            WHERE batch_id = ? AND status = ?
            ORDER BY row_number ASC, id ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare valid asset import row query: {err}"))?;

    let rows = stmt
        .query_map(params![batch_id, ROW_STATUS_VALID], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, Option<String>>(15)?,
                row.get::<_, Option<String>>(16)?,
                row.get::<_, Option<String>>(17)?,
                row.get::<_, Option<i64>>(18)?,
                row.get::<_, Option<String>>(19)?,
            ))
        })
        .map_err(|err| format!("failed to query valid asset import rows: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read valid asset import rows: {err}"))?;

    drop(stmt);

    if rows.iter().any(
        |(row_id, asset_code, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _)| {
            let _ = row_id;
            asset_code.as_deref().is_none()
        },
    ) {
        return Err("serialized asset rows require assetCode before import".to_string());
    }

    for (
        row_id,
        asset_code,
        asset_type,
        display_name,
        display_name_short,
        _,
        brand,
        model,
        serial_number,
        adapter_number,
        warehouse,
        usage_location,
        notes,
        submitted_staff_id,
        submitted_full_name,
        _submitted_team,
        _submitted_phone_number,
        resolved_employee_id,
        resolved_employee_row_id,
        resolved_full_name,
    ) in rows
    {
        let category_value = asset_type
            .clone()
            .ok_or_else(|| format!("row {row_id} is missing assetType"))?;
        let category = asset::load_asset_category_by_code_or_name_tx(&tx, category_value.as_str())?
            .ok_or_else(|| format!("asset category '{category_value}' was not found"))?;

        if category.tracking_mode != AssetImportMode::Serialized.as_str() {
            return Err(format!(
                "asset category '{}' is not configured for serialized assets",
                category_value
            ));
        }

        let record = asset::create_asset_tx(
            &tx,
            &AssetUpsertInput {
                asset_code: asset_code.unwrap_or_default(),
                asset_type: asset_type.unwrap_or_default(),
                display_name: display_name.unwrap_or_default(),
                display_name_short,
                brand,
                model,
                serial_number,
                category_id: Some(category.id),
                usage_location,
                adapter_number,
                warehouse,
                notes,
            },
        )?;

        let requires_owner_resolution = submitted_staff_id.is_some();

        if requires_owner_resolution {
            let employee_row_id = resolved_employee_row_id.ok_or_else(|| {
                format!("row {row_id} cannot import assigned asset without a resolved employee")
            })?;
            let employee_id = resolved_employee_id
                .as_deref()
                .ok_or_else(|| format!("row {row_id} is missing resolved employee id"))?;
            let full_name = resolved_full_name
                .as_deref()
                .or(submitted_full_name.as_deref())
                .ok_or_else(|| format!("row {row_id} is missing resolved employee name"))?;

            create_owner_aware_import_loan_tx(
                &tx,
                record.id,
                record.asset_code.as_str(),
                employee_row_id,
                employee_id,
                full_name,
                reviewer_account_id,
            )?;
        }

        tx.execute(
            r#"
            UPDATE asset_import_rows
            SET
              status = ?,
              imported_asset_id = ?,
              validation_errors_json = '[]',
              updated_at = datetime('now')
            WHERE id = ?
            "#,
            params![ROW_STATUS_IMPORTED, record.id, row_id],
        )
        .map_err(humanize_sqlite_error)?;

        imported_row_ids.push(row_id);
        imported_asset_codes.push(record.asset_code);
    }

    revalidate_batch_tx(&tx, batch_id)?;
    let summary = load_batch_summary_tx(&tx, batch_id)?;

    let payload_json = json!({
        "importedRowIds": imported_row_ids,
        "importedAssetCodes": imported_asset_codes,
    })
    .to_string();

    audit::insert_audit_log_tx(
        &tx,
        "asset_import.import_valid_rows",
        "local_account",
        actor_ref.as_deref(),
        "asset_import_batch",
        batch_id.to_string().as_str(),
        Some(payload_json.as_str()),
    )?;

    tx.commit()
        .map_err(|err| format!("failed to commit asset import batch rows: {err}"))?;

    Ok(AssetImportCommitResult {
        batch_id,
        imported_count: imported_row_ids.len() as i64,
        imported_row_ids,
        imported_asset_codes,
        remaining_error_rows: summary.error_rows,
        batch_status: summary.status,
    })
}

pub(crate) fn delete_asset_import_batch_conn(
    conn: &mut Connection,
    batch_id: i64,
) -> Result<bool, String> {
    delete_asset_import_batch_conn_internal(conn, batch_id, true)
}

pub(crate) fn preview_asset_import_seed_conn(
    conn: &mut Connection,
    input: AssetImportBatchSeedInput,
) -> Result<AssetDirectImportPreview, String> {
    preview_asset_import_seed_conn_with_cleanup(
        conn,
        input,
        delete_asset_import_batch_conn_internal,
    )
}

fn preview_asset_import_seed_conn_with_cleanup<F>(
    conn: &mut Connection,
    input: AssetImportBatchSeedInput,
    cleanup_batch: F,
) -> Result<AssetDirectImportPreview, String>
where
    F: FnOnce(&mut Connection, i64, bool) -> Result<bool, String>,
{
    let batch = create_asset_import_batch_seed_conn(conn, input)?;
    let batch_id = batch.summary.id;
    let preview = asset_direct_preview_from_batch_detail(&batch);

    if let Err(err) = cleanup_batch(conn, batch_id, false) {
        eprintln!(
            "failed to clean up temporary asset import preview batch {}: {}",
            batch_id, err
        );
    }

    Ok(preview)
}

pub(crate) fn import_asset_import_seed_conn(
    conn: &mut Connection,
    input: AssetImportBatchSeedInput,
) -> Result<AssetDirectImportReport, String> {
    import_asset_import_seed_conn_with_cleanup(conn, input, delete_asset_import_batch_conn_internal)
}

fn import_asset_import_seed_conn_with_cleanup<F>(
    conn: &mut Connection,
    input: AssetImportBatchSeedInput,
    cleanup_batch: F,
) -> Result<AssetDirectImportReport, String>
where
    F: FnOnce(&mut Connection, i64, bool) -> Result<bool, String>,
{
    let batch = create_asset_import_batch_seed_conn(conn, input)?;
    let batch_id = batch.summary.id;
    let result = import_asset_import_batch_valid_rows_conn(conn, batch_id)?;
    let batch_after = load_asset_import_batch_detail_conn(conn, batch_id)?;
    let report = asset_direct_report_from_batch_detail(&batch_after, &result);

    if let Err(err) = cleanup_batch(conn, batch_id, false) {
        eprintln!(
            "asset import succeeded but failed to delete temporary import batch {}: {}",
            batch_id, err
        );
    }

    Ok(report)
}

fn delete_asset_import_batch_conn_internal(
    conn: &mut Connection,
    batch_id: i64,
    write_audit_log: bool,
) -> Result<bool, String> {
    let actor_ref = active_actor_ref(conn)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start asset import delete transaction: {err}"))?;

    let existed = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM asset_import_batches WHERE id = ?)",
            params![batch_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|err| format!("failed to check asset import batch existence: {err}"))?
        > 0;

    if !existed {
        return Ok(false);
    }

    tx.execute(
        "DELETE FROM asset_import_batches WHERE id = ?",
        params![batch_id],
    )
    .map_err(humanize_sqlite_error)?;

    if write_audit_log {
        audit::insert_audit_log_tx(
            &tx,
            "asset_import.delete_batch",
            "local_account",
            actor_ref.as_deref(),
            "asset_import_batch",
            batch_id.to_string().as_str(),
            None,
        )?;
    }

    tx.commit()
        .map_err(|err| format!("failed to commit asset import batch delete: {err}"))?;

    Ok(true)
}

fn asset_import_seed_from_parsed(
    parsed: ParsedAssetImportSource,
    import_type: AssetImportMode,
) -> AssetImportBatchSeedInput {
    AssetImportBatchSeedInput {
        import_type,
        source_file_name: parsed.source_file_name,
        source_file_path: parsed.source_file_path,
        source_file_type: parsed.source_file_type,
        sheet_name: parsed.sheet_name,
        header_row: parsed.header_row,
        headers: parsed.headers,
        mapping: parsed.auto_mapping,
        rows: parsed.rows,
    }
}

fn asset_direct_preview_from_batch_detail(
    batch: &AssetImportBatchDetail,
) -> AssetDirectImportPreview {
    AssetDirectImportPreview {
        file_name: batch.summary.source_file_name.clone(),
        sheet_name: batch.summary.sheet_name.clone(),
        import_type: batch.summary.import_type,
        total_rows: batch.summary.total_rows,
        valid_rows: batch.summary.valid_rows,
        error_rows: batch.summary.error_rows,
        skipped_rows: batch.summary.skipped_rows,
        rows: batch
            .rows
            .iter()
            .map(asset_direct_preview_row_from_record)
            .collect(),
        errors: asset_direct_error_items_from_rows(&batch.rows),
    }
}

fn asset_direct_report_from_batch_detail(
    batch: &AssetImportBatchDetail,
    result: &AssetImportCommitResult,
) -> AssetDirectImportReport {
    let legacy_quantity_skipped = if batch.summary.import_type == AssetImportMode::Quantity {
        batch.summary.error_rows
    } else {
        0
    };
    AssetDirectImportReport {
        file_name: batch.summary.source_file_name.clone(),
        sheet_name: batch.summary.sheet_name.clone(),
        import_type: batch.summary.import_type,
        total_rows: batch.summary.total_rows,
        imported: result.imported_count,
        skipped: batch.summary.skipped_rows + legacy_quantity_skipped,
        failed: batch.summary.error_rows - legacy_quantity_skipped,
        imported_asset_codes: result.imported_asset_codes.clone(),
        errors: asset_direct_error_items_from_rows(&batch.rows),
    }
}

fn asset_direct_preview_row_from_record(row: &AssetImportRowRecord) -> AssetDirectImportPreviewRow {
    let holder_label = row
        .resolved_full_name
        .clone()
        .or_else(|| row.submitted_full_name.clone())
        .or_else(|| row.resolved_employee_id.clone())
        .or_else(|| row.submitted_staff_id.clone());

    AssetDirectImportPreviewRow {
        row_number: row.row_number,
        asset_code: row.asset_code.clone(),
        asset_type: row.asset_type.clone(),
        computer_name: row.computer_name.clone(),
        display_name: row.display_name.clone(),
        model: row.model.clone(),
        serial_number: row.serial_number.clone(),
        adapter_number: row.adapter_number.clone(),
        quantity: row.quantity.clone(),
        usage_location: row.usage_location.clone(),
        notes: row.notes.clone(),
        status: row.status.clone(),
        holder_label,
        validation_errors: row.validation_errors.clone(),
    }
}

fn asset_direct_error_items_from_rows(
    rows: &[AssetImportRowRecord],
) -> Vec<AssetDirectImportErrorItem> {
    rows.iter()
        .flat_map(|row| {
            row.validation_errors
                .iter()
                .map(|reason| AssetDirectImportErrorItem {
                    row_number: row.row_number,
                    entity_key: row.asset_code.clone().or_else(|| row.display_name.clone()),
                    reason: reason.clone(),
                })
        })
        .collect()
}

fn active_actor_ref(conn: &Connection) -> Result<Option<String>, String> {
    Ok(auth::get_active_local_account_id(conn)?.map(|id| id.to_string()))
}

fn parse_asset_import_source(
    path: &Path,
    import_type: AssetImportMode,
    requested_sheet_name: Option<&str>,
    mapping_override: Option<AssetImportFieldMapping>,
) -> Result<ParsedAssetImportSource, String> {
    let source_file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("failed to resolve file name for '{}'", path.display()))?
        .to_string();
    let source_file_type = import_file_type(path)?;

    match source_file_type.as_str() {
        "csv" => parse_csv_source(path, source_file_name, import_type, mapping_override),
        "xlsx" | "xls" | "xlsm" => parse_excel_source(
            path,
            source_file_name,
            source_file_type,
            import_type,
            requested_sheet_name,
            mapping_override,
        ),
        other => Err(format!("unsupported asset import file type: {other}")),
    }
}

fn inspect_asset_import_source(
    path: &Path,
    requested_sheet_name: Option<&str>,
) -> Result<ParsedAssetImportSource, String> {
    let source_file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("failed to resolve file name for '{}'", path.display()))?
        .to_string();
    let source_file_type = import_file_type(path)?;

    match source_file_type.as_str() {
        "csv" => {
            let mut reader = ReaderBuilder::new()
                .flexible(true)
                .from_path(path)
                .map_err(|err| format!("failed to open csv file '{}': {err}", path.display()))?;
            let headers = reader
                .headers()
                .map_err(|err| format!("failed to read csv headers '{}': {err}", path.display()))?
                .iter()
                .map(|value| value.trim().to_string())
                .collect::<Vec<_>>();

            if headers.is_empty() {
                return Err("csv file does not contain a header row".to_string());
            }

            Ok(ParsedAssetImportSource {
                source_file_name,
                source_file_path: path.to_string_lossy().to_string(),
                source_file_type: "csv".to_string(),
                sheet_name: None,
                header_row: 1,
                available_sheets: Vec::new(),
                headers: headers.clone(),
                auto_mapping: detect_field_mapping(&headers),
                rows: Vec::new(),
            })
        }
        "xlsx" | "xls" | "xlsm" => {
            let mut workbook = open_workbook_auto(path)
                .map_err(|err| format!("failed to open workbook '{}': {err}", path.display()))?;
            let available_sheets = workbook.sheet_names().to_vec();

            if available_sheets.is_empty() {
                return Err("excel file does not contain any sheets".to_string());
            }

            let (sheet_name, header_row_index, headers) =
                select_excel_header_row(&mut workbook, requested_sheet_name, &available_sheets)?;

            Ok(ParsedAssetImportSource {
                source_file_name,
                source_file_path: path.to_string_lossy().to_string(),
                source_file_type,
                sheet_name: Some(sheet_name),
                header_row: (header_row_index + 1) as i64,
                available_sheets,
                headers: headers.clone(),
                auto_mapping: detect_field_mapping(&headers),
                rows: Vec::new(),
            })
        }
        other => Err(format!("unsupported asset import file type: {other}")),
    }
}

fn parse_csv_source(
    path: &Path,
    source_file_name: String,
    import_type: AssetImportMode,
    mapping_override: Option<AssetImportFieldMapping>,
) -> Result<ParsedAssetImportSource, String> {
    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .map_err(|err| format!("failed to open csv file '{}': {err}", path.display()))?;

    let headers = reader
        .headers()
        .map_err(|err| format!("failed to read csv headers '{}': {err}", path.display()))?
        .iter()
        .map(|value| value.trim().to_string())
        .collect::<Vec<_>>();

    if headers.is_empty() {
        return Err("csv file does not contain a header row".to_string());
    }

    let resolved_mapping = resolve_mapping(&headers, import_type, mapping_override.clone())?;
    let mut rows = Vec::new();

    for (index, record) in reader.records().enumerate() {
        let record = record.map_err(|err| format!("failed to read csv row: {err}"))?;
        if csv_row_is_empty(&record) {
            continue;
        }

        rows.push(build_row_seed_from_csv_record(
            &headers,
            &record,
            (index + 2) as i64,
            &resolved_mapping,
        ));
    }

    Ok(ParsedAssetImportSource {
        source_file_name,
        source_file_path: path.to_string_lossy().to_string(),
        source_file_type: "csv".to_string(),
        sheet_name: None,
        header_row: 1,
        available_sheets: Vec::new(),
        headers,
        auto_mapping: resolved_mapping.mapping.clone(),
        rows,
    })
}

fn parse_excel_source(
    path: &Path,
    source_file_name: String,
    source_file_type: String,
    import_type: AssetImportMode,
    requested_sheet_name: Option<&str>,
    mapping_override: Option<AssetImportFieldMapping>,
) -> Result<ParsedAssetImportSource, String> {
    let mut workbook = open_workbook_auto(path)
        .map_err(|err| format!("failed to open workbook '{}': {err}", path.display()))?;
    let available_sheets = workbook.sheet_names().to_vec();

    if available_sheets.is_empty() {
        return Err("excel file does not contain any sheets".to_string());
    }

    let (sheet_name, header_row_index, headers) =
        select_excel_header_row(&mut workbook, requested_sheet_name, &available_sheets)?;

    let resolved_mapping = resolve_mapping(&headers, import_type, mapping_override.clone())?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|err| format!("failed to load sheet '{sheet_name}': {err}"))?;

    let mut rows = Vec::new();
    for (row_index, row) in range.rows().enumerate().skip(header_row_index + 1) {
        if excel_row_is_empty(row) {
            continue;
        }

        rows.push(build_row_seed_from_excel_row(
            &headers,
            row,
            (row_index + 1) as i64,
            &resolved_mapping,
        ));
    }

    Ok(ParsedAssetImportSource {
        source_file_name,
        source_file_path: path.to_string_lossy().to_string(),
        source_file_type,
        sheet_name: Some(sheet_name),
        header_row: (header_row_index + 1) as i64,
        available_sheets,
        headers,
        auto_mapping: resolved_mapping.mapping.clone(),
        rows,
    })
}

fn select_excel_header_row<RS, R>(
    workbook: &mut R,
    requested_sheet_name: Option<&str>,
    available_sheets: &[String],
) -> Result<(String, usize, Vec<String>), String>
where
    RS: Read + Seek,
    R: Reader<RS>,
{
    if let Some(sheet_name) = requested_sheet_name {
        if !available_sheets.iter().any(|item| item == sheet_name) {
            return Err(format!(
                "sheet '{sheet_name}' was not found in the workbook"
            ));
        }

        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|err| format!("failed to load sheet '{sheet_name}': {err:?}"))?;
        let (header_row_index, headers, _) = detect_excel_header_row(&range)?;
        return Ok((sheet_name.to_string(), header_row_index, headers));
    }

    let mut best_match: Option<(String, usize, Vec<String>, i64)> = None;
    for sheet_name in available_sheets {
        let range = match workbook.worksheet_range(sheet_name) {
            Ok(range) => range,
            Err(_) => continue,
        };
        let Ok((header_row_index, headers, score)) = detect_excel_header_row(&range) else {
            continue;
        };

        let should_replace = best_match
            .as_ref()
            .map(|(_, _, _, current_score)| score > *current_score)
            .unwrap_or(true);
        if should_replace {
            best_match = Some((sheet_name.clone(), header_row_index, headers, score));
        }
    }

    let Some((sheet_name, header_row_index, headers, _)) = best_match else {
        return Err("failed to detect an asset import header row in the workbook".to_string());
    };

    Ok((sheet_name, header_row_index, headers))
}

fn detect_excel_header_row(
    range: &calamine::Range<Data>,
) -> Result<(usize, Vec<String>, i64), String> {
    let mut best_match: Option<(usize, Vec<String>, i64)> = None;

    for (row_index, row) in range.rows().enumerate().take(30) {
        let headers = row.iter().map(cell_to_string).collect::<Vec<_>>();
        if headers.iter().all(|value| value.trim().is_empty()) {
            continue;
        }

        let score = mapping_score(&detect_field_mapping(&headers));
        if score == 0 {
            continue;
        }

        let should_replace = best_match
            .as_ref()
            .map(|(_, _, current_score)| score > *current_score)
            .unwrap_or(true);
        if should_replace {
            best_match = Some((row_index, headers, score));
        }
    }

    best_match.ok_or_else(|| "failed to detect asset import headers".to_string())
}

pub(crate) fn detect_asset_header_evidence(
    range: &calamine::Range<Data>,
) -> Option<AssetHeaderEvidence> {
    let mut best_match: Option<AssetHeaderEvidence> = None;
    let mut best_score = i64::MIN;

    for (row_index, row) in range.rows().enumerate().take(30) {
        let headers = row.iter().map(cell_to_string).collect::<Vec<_>>();
        if headers.iter().all(|value| value.trim().is_empty()) {
            continue;
        }

        let mapping = detect_field_mapping(&headers);
        let score = mapping_score(&mapping);
        if score <= 0 || score <= best_score {
            continue;
        }

        let legacy_asset_id_header = headers
            .iter()
            .find(|header| normalize_header_key(header) == "assetid")
            .cloned();
        best_score = score;
        best_match = Some(AssetHeaderEvidence {
            header_row: row_index,
            mapping,
            explicit_display_name: find_header_by_alias(&headers, DISPLAY_NAME_PRIMARY_ALIASES)
                .or_else(|| find_header_by_alias(&headers, DISPLAY_NAME_FALLBACK_ALIASES))
                .is_some(),
            legacy_asset_id_header,
        });
    }

    best_match
}

fn build_row_seed_from_csv_record(
    headers: &[String],
    record: &StringRecord,
    row_number: i64,
    mapping: &AssetImportResolvedMapping,
) -> AssetImportRowSeedInput {
    let raw_values = headers
        .iter()
        .enumerate()
        .map(|(index, header)| AssetImportRawValue {
            header: header.clone(),
            value: record.get(index).unwrap_or_default().trim().to_string(),
        })
        .collect::<Vec<_>>();

    build_row_seed_from_raw_values(raw_values, row_number, mapping)
}

fn build_row_seed_from_excel_row(
    headers: &[String],
    row: &[Data],
    row_number: i64,
    mapping: &AssetImportResolvedMapping,
) -> AssetImportRowSeedInput {
    let raw_values = headers
        .iter()
        .enumerate()
        .map(|(index, header)| AssetImportRawValue {
            header: header.clone(),
            value: row.get(index).map(cell_to_string).unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    build_row_seed_from_raw_values(raw_values, row_number, mapping)
}

fn build_row_seed_from_raw_values(
    raw_values: Vec<AssetImportRawValue>,
    row_number: i64,
    mapping: &AssetImportResolvedMapping,
) -> AssetImportRowSeedInput {
    let asset_type = mapped_value(&raw_values, mapping.asset_type_index);
    let display_name = mapped_value(&raw_values, mapping.display_name_index);
    let computer_name = mapping
        .computer_name_index
        .and_then(|index| mapped_value(&raw_values, index));
    let has_legacy_asset_id = raw_values.iter().any(|item| {
        normalize_header_key(item.header.as_str()) == "assetid" && !item.value.trim().is_empty()
    });
    let mapped_asset_code = mapping
        .asset_code_index
        .and_then(|index| mapped_value(&raw_values, index))
        .or_else(|| {
            if has_legacy_asset_id {
                derive_asset_code_from_legacy_fields(
                    asset_type.as_deref(),
                    display_name.as_deref(),
                    computer_name.as_deref(),
                )
            } else {
                derive_asset_code_from_display_name(asset_type.as_deref(), display_name.as_deref())
            }
        });
    let asset_code = mapped_asset_code.and_then(|value| {
        let source_header = mapping
            .asset_code_index
            .and_then(|index| raw_values.get(index))
            .map(|item| normalize_header_key(item.header.as_str()));
        if source_header.as_deref() == Some("assetid")
            && value.chars().all(|ch| ch.is_ascii_digit())
        {
            derive_asset_code_from_legacy_fields(
                asset_type.as_deref(),
                display_name.as_deref(),
                mapping
                    .computer_name_index
                    .and_then(|index| mapped_value(&raw_values, index))
                    .as_deref(),
            )
        } else {
            Some(value)
        }
    });
    let asset_code = asset_code.or_else(|| {
        if has_legacy_asset_id {
            None
        } else {
            derive_asset_code_from_display_name(asset_type.as_deref(), display_name.as_deref())
        }
    });
    let display_name_short = derive_display_name_short(
        asset_type.as_deref(),
        display_name.as_deref(),
        asset_code.as_deref(),
    );
    let computer_name = computer_name
        .or_else(|| {
            asset_code
                .as_ref()
                .map(|value| format!("ASW{}", value.trim().to_ascii_uppercase()))
        })
        .and_then(|value| normalize_optional_asset_text(Some(value)));

    AssetImportRowSeedInput {
        row_number,
        asset_code,
        asset_type,
        display_name,
        display_name_short,
        computer_name,
        brand: mapping
            .brand_index
            .and_then(|index| mapped_value(&raw_values, index)),
        model: mapping
            .model_index
            .and_then(|index| mapped_value(&raw_values, index)),
        serial_number: mapping
            .serial_number_index
            .and_then(|index| mapped_value(&raw_values, index)),
        adapter_number: mapping
            .adapter_number_index
            .and_then(|index| mapped_value(&raw_values, index)),
        quantity: mapping
            .quantity_index
            .and_then(|index| mapped_quantity_value(&raw_values, index)),
        warehouse: mapping
            .warehouse_index
            .and_then(|index| mapped_value(&raw_values, index)),
        usage_location: mapping
            .usage_location_index
            .and_then(|index| mapped_usage_location_value(&raw_values, index)),
        notes: mapping
            .notes_index
            .and_then(|index| mapped_value(&raw_values, index)),
        submitted_staff_id: find_value_by_header_alias(&raw_values, OWNER_STAFF_ID_ALIASES)
            .and_then(|value| normalize_submitted_staff_id(Some(value))),
        submitted_full_name: find_value_by_header_alias(&raw_values, OWNER_FULL_NAME_ALIASES)
            .and_then(|value| normalize_optional_asset_text(Some(value))),
        submitted_team: find_value_by_header_alias(&raw_values, OWNER_TEAM_ALIASES)
            .and_then(|value| normalize_optional_asset_text(Some(value))),
        submitted_phone_number: find_value_by_header_alias(&raw_values, OWNER_PHONE_NUMBER_ALIASES)
            .and_then(|value| normalize_optional_asset_text(Some(value))),
        raw_values,
    }
}

fn find_value_by_header_alias(
    raw_values: &[AssetImportRawValue],
    aliases: &[&str],
) -> Option<String> {
    raw_values
        .iter()
        .find(|item| {
            let normalized = normalize_header_key(item.header.as_str());
            aliases
                .iter()
                .any(|alias| normalized == normalize_header_key(alias))
        })
        .map(|item| item.value.clone())
}

fn mapped_value(raw_values: &[AssetImportRawValue], index: usize) -> Option<String> {
    raw_values
        .get(index)
        .map(|item| item.value.clone())
        .and_then(|value| normalize_optional_asset_text(Some(value)))
}

fn mapped_quantity_value(raw_values: &[AssetImportRawValue], index: usize) -> Option<String> {
    raw_values
        .get(index)
        .map(|item| item.value.clone())
        .and_then(|value| normalize_optional_quantity_text(Some(value)))
}

fn mapped_usage_location_value(raw_values: &[AssetImportRawValue], index: usize) -> Option<String> {
    raw_values
        .get(index)
        .map(|item| item.value.clone())
        .and_then(|value| normalize_usage_location(Some(value)))
}

fn resolve_mapping(
    headers: &[String],
    import_type: AssetImportMode,
    mapping_override: Option<AssetImportFieldMapping>,
) -> Result<AssetImportResolvedMapping, String> {
    let detected = detect_field_mapping(headers);
    let merged = if let Some(override_mapping) = mapping_override {
        merge_field_mapping(detected, override_mapping)
    } else {
        detected
    };

    let Some(missing_required) = mapping_missing_required_fields(import_type, &merged) else {
        let header_indices = headers
            .iter()
            .enumerate()
            .map(|(index, header)| (normalize_header_key(header), index))
            .collect::<HashMap<_, _>>();

        return Ok(AssetImportResolvedMapping {
            asset_code_index: resolve_optional_header_index(
                &header_indices,
                merged.asset_code.as_deref(),
            ),
            asset_type_index: resolve_header_index(&header_indices, merged.asset_type.as_deref())?,
            display_name_index: resolve_header_index(
                &header_indices,
                merged.display_name.as_deref(),
            )?,
            computer_name_index: resolve_optional_header_index(
                &header_indices,
                merged.computer_name.as_deref(),
            ),
            usage_location_index: resolve_optional_header_index(
                &header_indices,
                merged.usage_location.as_deref(),
            ),
            brand_index: resolve_optional_header_index(&header_indices, merged.brand.as_deref()),
            model_index: resolve_optional_header_index(&header_indices, merged.model.as_deref()),
            serial_number_index: resolve_optional_header_index(
                &header_indices,
                merged.serial_number.as_deref(),
            ),
            adapter_number_index: resolve_optional_header_index(
                &header_indices,
                merged.adapter_number.as_deref(),
            ),
            quantity_index: resolve_optional_header_index(
                &header_indices,
                merged.quantity.as_deref(),
            ),
            warehouse_index: resolve_optional_header_index(
                &header_indices,
                merged.warehouse.as_deref(),
            ),
            notes_index: resolve_optional_header_index(&header_indices, merged.notes.as_deref()),
            mapping: merged,
        });
    };

    Err(format!(
        "required asset import columns were not mapped: {}",
        missing_required.join(", ")
    ))
}

fn resolve_header_index(
    header_indices: &HashMap<String, usize>,
    header_name: Option<&str>,
) -> Result<usize, String> {
    let header_name =
        header_name.ok_or_else(|| "required asset import column mapping is missing".to_string())?;
    header_indices
        .get(normalize_header_key(header_name).as_str())
        .copied()
        .ok_or_else(|| format!("mapped source column '{header_name}' was not found in the file"))
}

fn resolve_optional_header_index(
    header_indices: &HashMap<String, usize>,
    header_name: Option<&str>,
) -> Option<usize> {
    header_name
        .map(normalize_header_key)
        .and_then(|normalized| header_indices.get(normalized.as_str()).copied())
}

fn detect_field_mapping(headers: &[String]) -> AssetImportFieldMapping {
    let mut mapping = AssetImportFieldMapping::default();

    for header in headers {
        let normalized = normalize_header_key(header);
        if mapping.asset_code.is_none() && ASSET_CODE_ALIASES.contains(&normalized.as_str()) {
            mapping.asset_code = Some(header.clone());
            continue;
        }
        if mapping.asset_type.is_none() && ASSET_TYPE_ALIASES.contains(&normalized.as_str()) {
            mapping.asset_type = Some(header.clone());
            continue;
        }
        if mapping.computer_name.is_none() && COMPUTER_NAME_ALIASES.contains(&normalized.as_str()) {
            mapping.computer_name = Some(header.clone());
            continue;
        }
        if mapping.usage_location.is_none() && USAGE_LOCATION_ALIASES.contains(&normalized.as_str())
        {
            mapping.usage_location = Some(header.clone());
            continue;
        }
        if mapping.brand.is_none() && BRAND_ALIASES.contains(&normalized.as_str()) {
            mapping.brand = Some(header.clone());
            continue;
        }
        if mapping.model.is_none() && MODEL_ALIASES.contains(&normalized.as_str()) {
            mapping.model = Some(header.clone());
            continue;
        }
        if mapping.serial_number.is_none() && SERIAL_NUMBER_ALIASES.contains(&normalized.as_str()) {
            mapping.serial_number = Some(header.clone());
            continue;
        }
        if mapping.adapter_number.is_none() && ADAPTER_NUMBER_ALIASES.contains(&normalized.as_str())
        {
            mapping.adapter_number = Some(header.clone());
            continue;
        }
        if mapping.quantity.is_none() && QUANTITY_ALIASES.contains(&normalized.as_str()) {
            mapping.quantity = Some(header.clone());
            continue;
        }
        if mapping.warehouse.is_none() && WAREHOUSE_ALIASES.contains(&normalized.as_str()) {
            mapping.warehouse = Some(header.clone());
            continue;
        }
        if mapping.notes.is_none() && NOTES_ALIASES.contains(&normalized.as_str()) {
            mapping.notes = Some(header.clone());
        }
    }

    if mapping.display_name.is_none() {
        mapping.display_name = find_header_by_alias(headers, DISPLAY_NAME_PRIMARY_ALIASES)
            .or_else(|| find_header_by_alias(headers, DISPLAY_NAME_FALLBACK_ALIASES))
            .or_else(|| mapping.computer_name.clone());
    }

    mapping
}

fn find_header_by_alias(headers: &[String], aliases: &[&str]) -> Option<String> {
    headers
        .iter()
        .find(|header| {
            let normalized = normalize_header_key(header);
            aliases
                .iter()
                .any(|alias| normalized == normalize_header_key(alias))
        })
        .cloned()
}

fn merge_field_mapping(
    detected: AssetImportFieldMapping,
    override_mapping: AssetImportFieldMapping,
) -> AssetImportFieldMapping {
    AssetImportFieldMapping {
        asset_code: normalized_mapping_choice(override_mapping.asset_code).or(detected.asset_code),
        asset_type: normalized_mapping_choice(override_mapping.asset_type).or(detected.asset_type),
        display_name: normalized_mapping_choice(override_mapping.display_name)
            .or(detected.display_name),
        computer_name: normalized_mapping_choice(override_mapping.computer_name)
            .or(detected.computer_name),
        usage_location: normalized_mapping_choice(override_mapping.usage_location)
            .or(detected.usage_location),
        brand: normalized_mapping_choice(override_mapping.brand).or(detected.brand),
        model: normalized_mapping_choice(override_mapping.model).or(detected.model),
        serial_number: normalized_mapping_choice(override_mapping.serial_number)
            .or(detected.serial_number),
        adapter_number: normalized_mapping_choice(override_mapping.adapter_number)
            .or(detected.adapter_number),
        quantity: normalized_mapping_choice(override_mapping.quantity).or(detected.quantity),
        warehouse: normalized_mapping_choice(override_mapping.warehouse).or(detected.warehouse),
        notes: normalized_mapping_choice(override_mapping.notes).or(detected.notes),
    }
}

fn normalized_mapping_choice(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn mapping_missing_required_fields(
    import_type: AssetImportMode,
    mapping: &AssetImportFieldMapping,
) -> Option<Vec<&'static str>> {
    let mut missing = Vec::new();
    if mapping.asset_type.is_none() {
        missing.push("assetType");
    }
    if mapping.display_name.is_none() {
        missing.push("displayName");
    }
    if matches!(import_type, AssetImportMode::Quantity) && mapping.quantity.is_none() {
        missing.push("quantity");
    }
    if missing.is_empty() {
        None
    } else {
        Some(missing)
    }
}

fn mapping_score(mapping: &AssetImportFieldMapping) -> i64 {
    let required_matches = [mapping.asset_type.as_ref(), mapping.display_name.as_ref()]
        .into_iter()
        .flatten()
        .count() as i64;
    let optional_matches = [
        mapping.asset_code.as_ref(),
        mapping.computer_name.as_ref(),
        mapping.usage_location.as_ref(),
        mapping.brand.as_ref(),
        mapping.model.as_ref(),
        mapping.serial_number.as_ref(),
        mapping.adapter_number.as_ref(),
        mapping.quantity.as_ref(),
        mapping.warehouse.as_ref(),
        mapping.notes.as_ref(),
    ]
    .into_iter()
    .flatten()
    .count() as i64;
    required_matches * 10 + optional_matches
}

fn resolve_import_file_path(file_path: String) -> Result<PathBuf, String> {
    let normalized = normalize_optional_text(Some(file_path))
        .ok_or_else(|| "asset import file path is required".to_string())?;
    let path = PathBuf::from(&normalized);
    if !path.exists() {
        return Err(format!(
            "asset import file does not exist: {}",
            path.display()
        ));
    }
    Ok(path)
}

fn import_file_type(path: &Path) -> Result<String, String> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| format!("failed to detect file type for '{}'", path.display()))?;

    match ext.as_str() {
        "csv" | "xlsx" | "xls" | "xlsm" => Ok(ext),
        other => Err(format!("unsupported asset import file type: {other}")),
    }
}

fn normalize_asset_code(value: Option<String>) -> Option<String> {
    normalize_optional_text(value).map(|item| item.to_uppercase())
}

fn normalize_identity_key(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

fn normalize_submitted_staff_id(value: Option<String>) -> Option<String> {
    normalize_optional_text(value).map(|item| item.to_uppercase())
}

fn normalize_optional_asset_text(value: Option<String>) -> Option<String> {
    normalize_optional_text(value)
}

fn normalize_optional_quantity_text(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_import_field_key(value: &str) -> Result<String, String> {
    match value.trim() {
        "assetCode"
        | "assetType"
        | "displayName"
        | "displayNameShort"
        | "computerName"
        | "brand"
        | "model"
        | "serialNumber"
        | "adapterNumber"
        | "quantity"
        | "warehouse"
        | "usageLocation"
        | "notes"
        | "submittedStaffId"
        | "submittedFullName"
        | "submittedTeam"
        | "submittedPhoneNumber" => Ok(value.trim().to_string()),
        _ => Err(format!("unsupported asset import field '{value}'")),
    }
}

fn extract_numeric_suffix(value: &str) -> Option<String> {
    let digits = value
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

fn is_legacy_staff_id_alias(value: &str) -> bool {
    let normalized = value.trim().to_ascii_uppercase();
    let digits = normalized
        .strip_prefix("ASWVN")
        .or_else(|| normalized.strip_prefix("ASW"))
        .unwrap_or(normalized.as_str());

    !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit())
}

fn derive_asset_code_from_display_name(
    asset_type: Option<&str>,
    display_name: Option<&str>,
) -> Option<String> {
    let normalized_type = asset_type.map(normalize_compare_text).unwrap_or_default();
    let display_name = normalize_optional_asset_text(display_name.map(str::to_string))?;
    let compact = display_name
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();

    if normalized_type == "laptop" {
        if let Some(suffix) = compact.strip_prefix("VNLAP") {
            if !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()) {
                return Some(format!("VNLAP{suffix}"));
            }
        }
    }

    if normalized_type == "monitor" {
        if let Some(suffix) = compact.strip_prefix("VNMON") {
            if !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()) {
                return Some(format!("VNMON{suffix}"));
            }
        }

        if let Some(suffix) = compact.strip_prefix("MON") {
            if !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()) {
                return Some(format!("VNMON{suffix}"));
            }
        }
    }

    None
}

fn derive_asset_code_from_legacy_fields(
    asset_type: Option<&str>,
    display_name: Option<&str>,
    computer_name: Option<&str>,
) -> Option<String> {
    let display_name = normalize_optional_asset_text(display_name.map(str::to_string))?;
    let candidate = display_name.trim().to_ascii_uppercase();
    let computer_name = computer_name
        .map(str::trim)
        .map(|value| value.to_ascii_uppercase());
    let derived_from_computer_name = computer_name
        .as_deref()
        .and_then(|value| value.strip_prefix("ASW"))
        .filter(|value| *value == candidate);

    let normalized_type = asset_type.map(normalize_compare_text).unwrap_or_default();
    let recognized_prefix = match normalized_type.as_str() {
        "laptop" => candidate.starts_with("VNLAP"),
        "monitor" => candidate.starts_with("VNMON"),
        value if value.contains("mac") => {
            candidate.starts_with("VNM") || candidate.starts_with("VNMAC")
        }
        _ => candidate.starts_with("VN"),
    };

    if recognized_prefix && derived_from_computer_name.is_some() {
        Some(candidate)
    } else {
        None
    }
}

fn normalize_compare_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect::<String>()
}

fn asset_import_request_key_exists_tx(
    tx: &Transaction<'_>,
    request_key: &str,
) -> Result<bool, String> {
    tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM borrow_requests WHERE request_key = ?)",
        params![request_key],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value > 0)
    .map_err(|err| format!("failed to check owner-aware import borrow request key: {err}"))
}

fn generate_asset_import_request_key_tx(tx: &Transaction<'_>) -> Result<String, String> {
    for _ in 0..16 {
        let suffix = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect::<String>()
            .to_uppercase();
        let request_key = format!("AIBR-{suffix}");
        if !asset_import_request_key_exists_tx(tx, request_key.as_str())? {
            return Ok(request_key);
        }
    }

    Err("failed to generate a unique asset-import borrow request key".to_string())
}

fn create_owner_aware_import_loan_tx(
    tx: &Transaction<'_>,
    asset_id: i64,
    asset_code: &str,
    employee_row_id: i64,
    employee_id: &str,
    full_name: &str,
    reviewer_account_id: Option<i64>,
) -> Result<(), String> {
    let request_key = generate_asset_import_request_key_tx(tx)?;
    tx.execute(
        r#"
        INSERT INTO borrow_requests(
          request_key,
          employee_id_fk,
          submitted_employee_id,
          submitted_full_name,
          status,
          request_type,
          submit_source_ip,
          decided_by_account_id,
          submitted_at,
          decided_at
        )
        VALUES(?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        "#,
        params![
            request_key,
            employee_row_id,
            employee_id,
            full_name,
            IMPORT_BORROW_REQUEST_STATUS_APPROVED,
            IMPORT_BORROW_REQUEST_TYPE_BORROW,
            Some("asset_import_wizard"),
            reviewer_account_id,
        ],
    )
    .map_err(humanize_sqlite_error)?;

    let request_id = tx.last_insert_rowid();

    tx.execute(
        r#"
        INSERT INTO borrow_request_items(
          borrow_request_id,
          asset_id,
          asset_code_snapshot
        )
        VALUES(?, ?, ?)
        "#,
        params![request_id, asset_id, asset_code],
    )
    .map_err(humanize_sqlite_error)?;

    asset::set_asset_status_tx(tx, asset_id, IMPORT_ASSET_STATUS_ASSIGNED)?;

    tx.execute(
        r#"
        INSERT INTO asset_loans(
          asset_id,
          employee_id_fk,
          borrow_request_id,
          approved_by_account_id,
          borrowed_at
        )
        VALUES(?, ?, ?, ?, datetime('now'))
        "#,
        params![asset_id, employee_row_id, request_id, reviewer_account_id],
    )
    .map_err(humanize_sqlite_error)?;

    Ok(())
}

fn normalize_header_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect::<String>()
}

fn csv_row_is_empty(record: &StringRecord) -> bool {
    record.iter().all(|value| value.trim().is_empty())
}

fn excel_row_is_empty(row: &[Data]) -> bool {
    row.iter().all(|cell| cell_to_string(cell).is_empty())
}

fn cell_to_string(cell: &Data) -> String {
    if let Some(value) = cell.get_string() {
        return value.trim().to_string();
    }
    if let Some(value) = cell.get_float() {
        let rounded = value.round();
        if (value - rounded).abs() < f64::EPSILON {
            return format!("{rounded:.0}");
        }
        return value.to_string();
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

fn generate_batch_key_tx(tx: &Transaction<'_>) -> Result<String, String> {
    for _ in 0..16 {
        let suffix = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect::<String>()
            .to_uppercase();
        let batch_key = format!("AIB-{suffix}");
        let exists: i64 = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM asset_import_batches WHERE batch_key = ?)",
                params![batch_key.as_str()],
                |row| row.get(0),
            )
            .map_err(|err| format!("failed to check asset import batch key: {err}"))?;
        if exists == 0 {
            return Ok(batch_key);
        }
    }

    Err("failed to generate a unique asset import batch key".to_string())
}

fn load_employee_owner_lookup_tx(
    tx: &Transaction<'_>,
) -> Result<HashMap<String, Vec<EmployeeOwnerLookup>>, String> {
    let mut stmt = tx
        .prepare(
            r#"
            SELECT
              e.id,
              e.employee_id,
              e.full_name,
              t.name
            FROM employees e
            LEFT JOIN teams t ON t.id = e.team_id
            ORDER BY e.id ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare employee owner lookup query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(EmployeeOwnerLookup {
                row_id: row.get(0)?,
                employee_id: row.get(1)?,
                full_name: row.get(2)?,
                team_name: row.get(3)?,
            })
        })
        .map_err(|err| format!("failed to query employee owner lookup rows: {err}"))?;

    let mut items: HashMap<String, Vec<EmployeeOwnerLookup>> = HashMap::new();
    for row in rows {
        let item = row.map_err(|err| format!("failed to read employee owner lookup row: {err}"))?;
        let suffix = extract_numeric_suffix(item.employee_id.as_str());
        items
            .entry(item.employee_id.to_ascii_uppercase())
            .or_default()
            .push(item.clone());
        if let Some(suffix) = suffix {
            items.entry(suffix).or_default().push(item);
        }
    }
    Ok(items)
}

fn row_requires_owner_resolution(row: &AssetImportRowState) -> bool {
    row.submitted_staff_id.is_some()
}

fn resolve_owner_state(
    row: &AssetImportRowState,
    employee_lookup: &HashMap<String, Vec<EmployeeOwnerLookup>>,
) -> OwnerResolutionState {
    if !row_requires_owner_resolution(row) {
        return OwnerResolutionState {
            resolved_employee_id: None,
            resolved_employee_row_id: None,
            resolved_full_name: None,
            resolved_team_name: None,
            owner_match_status: OWNER_MATCH_NOT_APPLICABLE.to_string(),
            owner_warnings: Vec::new(),
            blocking_error: None,
        };
    }

    let Some(submitted_staff_id) = row.submitted_staff_id.as_deref() else {
        return OwnerResolutionState {
            resolved_employee_id: None,
            resolved_employee_row_id: None,
            resolved_full_name: None,
            resolved_team_name: None,
            owner_match_status: OWNER_MATCH_UNRESOLVED.to_string(),
            owner_warnings: Vec::new(),
            blocking_error: Some(
                "employee owner could not be resolved because StaffID is missing".to_string(),
            ),
        };
    };

    let normalized_staff_id = submitted_staff_id.to_ascii_uppercase();
    let candidates = employee_lookup
        .get(normalized_staff_id.as_str())
        .or_else(|| {
            if is_legacy_staff_id_alias(normalized_staff_id.as_str()) {
                extract_numeric_suffix(normalized_staff_id.as_str())
                    .and_then(|suffix| employee_lookup.get(suffix.as_str()))
            } else {
                None
            }
        });

    let Some(candidates) = candidates else {
        return OwnerResolutionState {
            resolved_employee_id: None,
            resolved_employee_row_id: None,
            resolved_full_name: None,
            resolved_team_name: None,
            owner_match_status: OWNER_MATCH_UNRESOLVED.to_string(),
            owner_warnings: Vec::new(),
            blocking_error: Some(format!(
                "employee owner '{}' was not found in employee master",
                submitted_staff_id
            )),
        };
    };

    if candidates.len() != 1 {
        return OwnerResolutionState {
            resolved_employee_id: None,
            resolved_employee_row_id: None,
            resolved_full_name: None,
            resolved_team_name: None,
            owner_match_status: OWNER_MATCH_UNRESOLVED.to_string(),
            owner_warnings: Vec::new(),
            blocking_error: Some(format!(
                "employee owner '{}' matched multiple employees in employee master",
                submitted_staff_id
            )),
        };
    }

    let candidate = &candidates[0];
    let mut owner_warnings = Vec::new();
    if let Some(submitted_name) = row.submitted_full_name.as_deref() {
        if normalize_compare_text(submitted_name)
            != normalize_compare_text(candidate.full_name.as_str())
        {
            owner_warnings
                .push("submitted employee name does not match employee master".to_string());
        }
    }
    if let Some(submitted_team) = row.submitted_team.as_deref() {
        let candidate_team = candidate.team_name.as_deref().unwrap_or_default();
        if normalize_compare_text(submitted_team) != normalize_compare_text(candidate_team) {
            owner_warnings.push("submitted team does not match employee master".to_string());
        }
    }

    OwnerResolutionState {
        resolved_employee_id: Some(candidate.employee_id.clone()),
        resolved_employee_row_id: Some(candidate.row_id),
        resolved_full_name: Some(candidate.full_name.clone()),
        resolved_team_name: candidate.team_name.clone(),
        owner_match_status: if owner_warnings.is_empty() {
            OWNER_MATCH_MATCHED.to_string()
        } else {
            OWNER_MATCH_WARNING.to_string()
        },
        owner_warnings,
        blocking_error: None,
    }
}

fn revalidate_batch_tx(tx: &Transaction<'_>, batch_id: i64) -> Result<(), String> {
    let rows = load_batch_row_states_tx(tx, batch_id)?;
    let existing_assets = load_existing_asset_identities_tx(tx)?;
    let employee_lookup = load_employee_owner_lookup_tx(tx)?;

    let mut duplicate_counts: HashMap<String, usize> = HashMap::new();
    let mut duplicate_serial_counts: HashMap<String, usize> = HashMap::new();
    for row in rows
        .iter()
        .filter(|item| item.status != ROW_STATUS_IMPORTED && item.status != ROW_STATUS_SKIPPED)
    {
        if let Some(asset_code) = row.asset_code.as_deref() {
            *duplicate_counts.entry(asset_code.to_string()).or_default() += 1;
        }
        if let Some(serial_number) = row.serial_number.as_deref() {
            *duplicate_serial_counts
                .entry(serial_number.to_ascii_uppercase())
                .or_default() += 1;
        }
    }

    let duplicate_asset_codes = duplicate_counts
        .into_iter()
        .filter_map(|(asset_code, count)| if count > 1 { Some(asset_code) } else { None })
        .collect::<HashSet<_>>();
    let duplicate_serial_numbers = duplicate_serial_counts
        .into_iter()
        .filter_map(|(serial_number, count)| if count > 1 { Some(serial_number) } else { None })
        .collect::<HashSet<_>>();

    for row in &rows {
        if row.status == ROW_STATUS_IMPORTED {
            tx.execute(
                "UPDATE asset_import_rows SET validation_errors_json = '[]' WHERE id = ?",
                params![row.id],
            )
            .map_err(humanize_sqlite_error)?;
            continue;
        }

        let owner_state = resolve_owner_state(row, &employee_lookup);
        let (validation_errors, next_status) = if row.import_type == AssetImportMode::Serialized {
            match classify_serialized_asset_row(
                row,
                &existing_assets,
                &duplicate_asset_codes,
                &duplicate_serial_numbers,
            ) {
                SerializedAssetImportClassification::Existing { .. } => {
                    (Vec::new(), ROW_STATUS_SKIPPED)
                }
                SerializedAssetImportClassification::Conflict(conflict) => {
                    let mut errors = vec![conflict];
                    if let Some(blocking_error) = owner_state.blocking_error.as_ref() {
                        errors.push(blocking_error.clone());
                    }
                    (
                        errors,
                        if row.status == ROW_STATUS_SKIPPED {
                            ROW_STATUS_SKIPPED
                        } else {
                            ROW_STATUS_ERROR
                        },
                    )
                }
                SerializedAssetImportClassification::New => {
                    let mut errors = validate_staged_row(row);
                    if let Some(blocking_error) = owner_state.blocking_error.as_ref() {
                        errors.push(blocking_error.clone());
                    }
                    let next_status = if row.status == ROW_STATUS_SKIPPED {
                        ROW_STATUS_SKIPPED
                    } else if errors.is_empty() {
                        ROW_STATUS_VALID
                    } else {
                        ROW_STATUS_ERROR
                    };
                    (errors, next_status)
                }
            }
        } else {
            let mut errors = validate_staged_row(row);
            if let Some(blocking_error) = owner_state.blocking_error.as_ref() {
                errors.push(blocking_error.clone());
            }
            let next_status = if row.status == ROW_STATUS_SKIPPED {
                ROW_STATUS_SKIPPED
            } else if errors.is_empty() {
                ROW_STATUS_VALID
            } else {
                ROW_STATUS_ERROR
            };
            (errors, next_status)
        };

        tx.execute(
            r#"
            UPDATE asset_import_rows
            SET
              resolved_employee_id = ?,
              resolved_employee_row_id = ?,
              resolved_full_name = ?,
              resolved_team_name = ?,
              owner_match_status = ?,
              owner_warnings_json = ?,
              validation_errors_json = ?,
              status = ?,
              updated_at = datetime('now')
            WHERE id = ?
            "#,
            params![
                owner_state.resolved_employee_id,
                owner_state.resolved_employee_row_id,
                owner_state.resolved_full_name,
                owner_state.resolved_team_name,
                owner_state.owner_match_status,
                to_json(&owner_state.owner_warnings)?,
                to_json(&validation_errors)?,
                next_status,
                row.id
            ],
        )
        .map_err(humanize_sqlite_error)?;
    }

    refresh_batch_summary_tx(tx, batch_id)
}

fn validate_staged_row(row: &AssetImportRowState) -> Vec<String> {
    let mut errors = Vec::new();
    if row.import_type == AssetImportMode::Serialized && row.asset_code.is_none() {
        errors.push("assetCode is required for serialized assets".to_string());
    }
    if row.asset_type.is_none() {
        errors.push("assetType is required".to_string());
    }
    if row.display_name.is_none() {
        errors.push("displayName is required".to_string());
    }
    if row.import_type == AssetImportMode::Quantity {
        match row.quantity.as_deref() {
            None => errors.push("quantity is required".to_string()),
            Some(value) => match value.parse::<i64>() {
                Ok(parsed) if parsed > 0 => {}
                _ => errors.push("quantity must be a positive integer".to_string()),
            },
        }
    }
    errors
}

fn load_existing_asset_identities_tx(
    tx: &Transaction<'_>,
) -> Result<ExistingAssetIdentities, String> {
    let mut stmt = tx
        .prepare("SELECT id, asset_code, serial_number FROM assets")
        .map_err(|err| format!("failed to prepare asset identity lookup query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|err| format!("failed to query existing asset identities: {err}"))?;

    let mut items = ExistingAssetIdentities::default();
    for row in rows {
        let (id, asset_code, serial_number) =
            row.map_err(|err| format!("failed to read existing asset identity row: {err}"))?;
        let identity = ExistingAssetIdentity {
            id,
            serial_number: serial_number.and_then(|value| normalize_optional_text(Some(value))),
        };
        items.by_code.insert(
            normalize_identity_key(asset_code.as_str()),
            identity.clone(),
        );
        if let Some(serial_number) = identity.serial_number.as_deref() {
            items
                .by_serial
                .insert(normalize_identity_key(serial_number), identity);
        }
    }
    Ok(items)
}

fn classify_serialized_asset_row(
    row: &AssetImportRowState,
    existing_assets: &ExistingAssetIdentities,
    duplicate_asset_codes: &HashSet<String>,
    duplicate_serial_numbers: &HashSet<String>,
) -> SerializedAssetImportClassification {
    let Some(asset_code) = row.asset_code.as_deref() else {
        return SerializedAssetImportClassification::New;
    };

    let code_key = normalize_identity_key(asset_code);
    let serial_key = row.serial_number.as_deref().map(normalize_identity_key);
    let code_asset = existing_assets.by_code.get(&code_key);
    let serial_asset = serial_key
        .as_deref()
        .and_then(|key| existing_assets.by_serial.get(key));

    match (code_asset, serial_asset) {
        (Some(code_asset), Some(serial_asset)) if code_asset.id != serial_asset.id => {
            SerializedAssetImportClassification::Conflict(format!(
                "Conflict: assetCode '{}' and serialNumber '{}' identify different existing assets",
                asset_code,
                row.serial_number.as_deref().unwrap_or_default()
            ))
        }
        (Some(code_asset), Some(_)) => {
            if serial_matches_existing(code_asset, serial_key.as_deref()) {
                SerializedAssetImportClassification::Existing {
                    asset_id: code_asset.id,
                }
            } else {
                SerializedAssetImportClassification::Conflict(format!(
                    "Conflict: serialNumber '{}' does not match existing assetCode '{}'",
                    row.serial_number.as_deref().unwrap_or_default(),
                    asset_code
                ))
            }
        }
        (Some(code_asset), None) => {
            if serial_key.is_none() || code_asset.serial_number.is_none() {
                SerializedAssetImportClassification::Existing {
                    asset_id: code_asset.id,
                }
            } else {
                SerializedAssetImportClassification::Conflict(format!(
                    "Conflict: serialNumber '{}' does not match existing assetCode '{}'",
                    row.serial_number.as_deref().unwrap_or_default(),
                    asset_code
                ))
            }
        }
        (None, Some(serial_asset)) => SerializedAssetImportClassification::Conflict(format!(
            "Conflict: serialNumber '{}' belongs to existing asset '{}' but assetCode '{}' does not match",
            row.serial_number.as_deref().unwrap_or_default(),
            serial_asset.id,
            asset_code
        )),
        (None, None) => {
            if duplicate_asset_codes.contains(&code_key) {
                SerializedAssetImportClassification::Conflict(format!(
                    "Conflict: assetCode '{}' is duplicated in this batch",
                    asset_code
                ))
            } else if serial_key
                .as_deref()
                .is_some_and(|key| duplicate_serial_numbers.contains(key))
            {
                SerializedAssetImportClassification::Conflict(format!(
                    "Conflict: serialNumber '{}' is duplicated in this batch",
                    row.serial_number.as_deref().unwrap_or_default()
                ))
            } else {
                SerializedAssetImportClassification::New
            }
        }
    }
}

fn serial_matches_existing(
    existing_asset: &ExistingAssetIdentity,
    incoming_serial_key: Option<&str>,
) -> bool {
    match (existing_asset.serial_number.as_deref(), incoming_serial_key) {
        (None, _) | (_, None) => true,
        (Some(existing), Some(incoming)) => normalize_identity_key(existing) == incoming,
    }
}

fn load_batch_row_states_tx(
    tx: &Transaction<'_>,
    batch_id: i64,
) -> Result<Vec<AssetImportRowState>, String> {
    let mut stmt = tx
        .prepare(
            r#"
            SELECT
              r.id,
              r.status,
              b.import_type,
              r.asset_code,
              r.asset_type,
              r.display_name,
              r.quantity,
              r.serial_number,
              r.submitted_staff_id,
              r.submitted_full_name,
              r.submitted_team
            FROM asset_import_rows r
            INNER JOIN asset_import_batches b ON b.id = r.batch_id
            WHERE r.batch_id = ?
            ORDER BY r.row_number ASC, r.id ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare staged asset row query: {err}"))?;
    let rows = stmt
        .query_map(params![batch_id], |row| {
            Ok(AssetImportRowState {
                id: row.get(0)?,
                status: row.get(1)?,
                import_type: parse_asset_import_mode_sql(row.get(2)?)?,
                asset_code: row.get(3)?,
                asset_type: row.get(4)?,
                display_name: row.get(5)?,
                quantity: row.get(6)?,
                serial_number: row.get(7)?,
                submitted_staff_id: row.get(8)?,
                submitted_full_name: row.get(9)?,
                submitted_team: row.get(10)?,
            })
        })
        .map_err(|err| format!("failed to query staged asset rows: {err}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|err| format!("failed to read staged asset row: {err}"))?);
    }
    Ok(items)
}

fn refresh_batch_summary_tx(tx: &Transaction<'_>, batch_id: i64) -> Result<(), String> {
    let mut total_rows = 0_i64;
    let mut valid_rows = 0_i64;
    let mut error_rows = 0_i64;
    let mut imported_rows = 0_i64;
    let mut skipped_rows = 0_i64;

    let mut stmt = tx
        .prepare(
            "SELECT status, COUNT(*) FROM asset_import_rows WHERE batch_id = ? GROUP BY status",
        )
        .map_err(|err| format!("failed to prepare asset import batch counter query: {err}"))?;
    let rows = stmt
        .query_map(params![batch_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|err| format!("failed to query asset import batch counters: {err}"))?;

    for row in rows {
        let (status, count) =
            row.map_err(|err| format!("failed to read asset import batch counter row: {err}"))?;
        total_rows += count;
        match status.as_str() {
            ROW_STATUS_VALID => valid_rows = count,
            ROW_STATUS_ERROR => error_rows = count,
            ROW_STATUS_IMPORTED => imported_rows = count,
            ROW_STATUS_SKIPPED => skipped_rows = count,
            _ => {}
        }
    }

    let next_status = if valid_rows == 0 && error_rows == 0 {
        BATCH_STATUS_COMPLETED
    } else {
        BATCH_STATUS_PENDING_REVIEW
    };

    tx.execute(
        r#"
        UPDATE asset_import_batches
        SET
          status = ?,
          total_rows = ?,
          valid_rows = ?,
          error_rows = ?,
          imported_rows = ?,
          skipped_rows = ?,
          updated_at = datetime('now')
        WHERE id = ?
        "#,
        params![
            next_status,
            total_rows,
            valid_rows,
            error_rows,
            imported_rows,
            skipped_rows,
            batch_id
        ],
    )
    .map_err(humanize_sqlite_error)?;

    Ok(())
}

fn load_batch_summary_tx(
    tx: &Transaction<'_>,
    batch_id: i64,
) -> Result<AssetImportBatchSummary, String> {
    tx.query_row(
        r#"
        SELECT
          id,
          batch_key,
          import_type,
          source_file_name,
          source_file_path,
          source_file_type,
          sheet_name,
          header_row,
          status,
          total_rows,
          valid_rows,
          error_rows,
          imported_rows,
          skipped_rows,
          created_at,
          updated_at
        FROM asset_import_batches
        WHERE id = ?
        "#,
        params![batch_id],
        map_batch_summary,
    )
    .optional()
    .map_err(|err| format!("failed to load asset import batch summary: {err}"))?
    .ok_or_else(|| format!("asset import batch with id {batch_id} was not found"))
}

fn map_batch_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetImportBatchSummary> {
    Ok(AssetImportBatchSummary {
        id: row.get(0)?,
        batch_key: row.get(1)?,
        import_type: parse_asset_import_mode_sql(row.get(2)?)?,
        source_file_name: row.get(3)?,
        source_file_path: row.get(4)?,
        source_file_type: row.get(5)?,
        sheet_name: row.get(6)?,
        header_row: row.get(7)?,
        status: row.get(8)?,
        total_rows: row.get(9)?,
        valid_rows: row.get(10)?,
        error_rows: row.get(11)?,
        imported_rows: row.get(12)?,
        skipped_rows: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn parse_asset_import_mode_sql(value: String) -> rusqlite::Result<AssetImportMode> {
    AssetImportMode::parse(value.as_str()).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
        )
    })
}

fn load_asset_import_rows_for_batch(
    conn: &Connection,
    batch_id: i64,
) -> Result<Vec<AssetImportRowRecord>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              id, batch_id, row_number, raw_row_json, asset_code, asset_type, display_name, display_name_short,
              computer_name, brand, model, serial_number, adapter_number, quantity, warehouse, usage_location, notes,
              submitted_staff_id, submitted_full_name, submitted_team, submitted_phone_number,
              resolved_employee_id, resolved_employee_row_id, resolved_full_name, resolved_team_name,
              owner_match_status, owner_warnings_json,
              validation_errors_json, status, edited, edited_fields_json, imported_asset_id
            FROM asset_import_rows
            WHERE batch_id = ?
            ORDER BY row_number ASC, id ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare asset import row detail query: {err}"))?;
    let rows = stmt
        .query_map(params![batch_id], |row| map_asset_import_row(row))
        .map_err(|err| format!("failed to query asset import rows: {err}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|err| format!("failed to read asset import row detail: {err}"))?);
    }
    Ok(items)
}

fn load_asset_import_row_record_conn(
    conn: &Connection,
    row_id: i64,
) -> Result<AssetImportRowRecord, String> {
    conn.query_row(
        r#"
        SELECT
          id, batch_id, row_number, raw_row_json, asset_code, asset_type, display_name, display_name_short,
          computer_name, brand, model, serial_number, adapter_number, quantity, warehouse, usage_location, notes,
          submitted_staff_id, submitted_full_name, submitted_team, submitted_phone_number,
          resolved_employee_id, resolved_employee_row_id, resolved_full_name, resolved_team_name,
          owner_match_status, owner_warnings_json,
          validation_errors_json, status, edited, edited_fields_json, imported_asset_id
        FROM asset_import_rows
        WHERE id = ?
        "#,
        params![row_id],
        |row| map_asset_import_row(row),
    )
    .optional()
    .map_err(|err| format!("failed to load asset import row detail: {err}"))?
    .ok_or_else(|| format!("asset import row with id {row_id} was not found"))
}

fn map_asset_import_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetImportRowRecord> {
    Ok(AssetImportRowRecord {
        id: row.get(0)?,
        batch_id: row.get(1)?,
        row_number: row.get(2)?,
        raw_values: from_json_row(Some(row.get(3)?))?,
        asset_code: row.get(4)?,
        asset_type: row.get(5)?,
        display_name: row.get(6)?,
        display_name_short: row.get(7)?,
        computer_name: row.get(8)?,
        brand: row.get(9)?,
        model: row.get(10)?,
        serial_number: row.get(11)?,
        adapter_number: row.get(12)?,
        quantity: row.get(13)?,
        warehouse: row.get(14)?,
        usage_location: row.get(15)?,
        notes: row.get(16)?,
        submitted_staff_id: row.get(17)?,
        submitted_full_name: row.get(18)?,
        submitted_team: row.get(19)?,
        submitted_phone_number: row.get(20)?,
        resolved_employee_id: row.get(21)?,
        resolved_employee_row_id: row.get(22)?,
        resolved_full_name: row.get(23)?,
        resolved_team_name: row.get(24)?,
        owner_match_status: row.get(25)?,
        owner_warnings: from_json_row(Some(row.get(26)?))?,
        validation_errors: from_json_row(Some(row.get(27)?))?,
        status: row.get(28)?,
        is_edited: row.get::<_, i64>(29)? > 0,
        edited_fields: from_json_row(Some(row.get(30)?))?,
        imported_asset_id: row.get(31)?,
    })
}

fn normalize_usage_location(value: Option<String>) -> Option<String> {
    let normalized = normalize_optional_text(value)?;
    let uppercase = normalized.to_uppercase();
    if uppercase.contains("TẠI CTY")
        || uppercase.contains("TAI CTY")
        || uppercase.contains("CÔNG TY")
        || uppercase.contains("CONG TY")
        || uppercase.contains("OFFICE")
    {
        return Some("office".to_string());
    }
    if uppercase.contains("TẠI NHÀ")
        || uppercase.contains("TAI NHA")
        || uppercase.contains(" NHÀ")
        || uppercase.contains("HOME")
    {
        return Some("home".to_string());
    }
    None
}

fn derive_display_name_short(
    asset_type: Option<&str>,
    display_name: Option<&str>,
    asset_code: Option<&str>,
) -> Option<String> {
    let normalized_type = asset_type.map(normalize_compare_text).unwrap_or_default();
    if normalized_type != "monitor" {
        return None;
    }

    if let Some(display_name) = normalize_optional_asset_text(display_name.map(str::to_string)) {
        return Some(display_name);
    }

    let asset_code = normalize_asset_code(asset_code.map(str::to_string))?;
    let suffix = asset_code.trim_start_matches("VNMON");
    if suffix.is_empty() {
        extract_numeric_suffix(asset_code.as_str()).map(|digits| format!("Mon{digits}"))
    } else {
        Some(format!("Mon{suffix}"))
    }
}

fn load_asset_import_row_meta_tx(
    tx: &Transaction<'_>,
    row_id: i64,
) -> Result<AssetImportBatchRowMeta, String> {
    tx.query_row(
        "SELECT batch_id, status, edited_fields_json FROM asset_import_rows WHERE id = ?",
        params![row_id],
        |row| {
            Ok(AssetImportBatchRowMeta {
                batch_id: row.get(0)?,
                status: row.get(1)?,
                edited_fields: from_json_row(Some(row.get(2)?))?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to load asset import row metadata: {err}"))?
    .ok_or_else(|| format!("asset import row with id {row_id} was not found"))
}

fn to_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|err| format!("failed to encode asset import json: {err}"))
}

fn from_json<T>(value: Option<String>) -> Result<T, String>
where
    T: DeserializeOwned + Default,
{
    match value {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw.as_str())
            .map_err(|err| format!("failed to decode asset import json: {err}")),
        _ => Ok(T::default()),
    }
}

fn from_json_row<T>(value: Option<String>) -> rusqlite::Result<T>
where
    T: DeserializeOwned + Default,
{
    from_json(value).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
        )
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use rusqlite::{params, Connection};

    use crate::db::{apply_migrations, configure_connection};

    use super::{
        create_asset_import_batch_seed_conn, import_asset_import_batch_valid_rows_conn,
        import_asset_import_seed_conn, import_asset_import_seed_conn_with_cleanup,
        load_asset_import_batch_detail_conn, parse_asset_import_source,
        preview_asset_import_seed_conn, preview_asset_import_seed_conn_with_cleanup,
        update_asset_import_row_conn, AssetImportBatchSeedInput, AssetImportFieldMapping,
        AssetImportMode, AssetImportRawValue, AssetImportRowSeedInput, AssetImportRowUpdateInput,
    };

    fn open_test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        configure_connection(&conn).expect("configure sqlite pragmas");
        apply_migrations(&conn).expect("apply migrations");
        conn
    }

    fn temp_db_path(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("staff-kit-{test_name}-{unique}.sqlite3"))
    }

    fn temp_csv_path(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("staff-kit-{test_name}-{unique}.csv"))
    }

    fn write_temp_csv(test_name: &str, rows: &[&str]) -> PathBuf {
        let path = temp_csv_path(test_name);
        fs::write(&path, rows.join("\n")).expect("write temp csv");
        path
    }

    fn seed_asset(conn: &Connection, asset_code: &str) {
        seed_asset_with_identity(conn, asset_code, None, "in_stock");
    }

    fn seed_asset_with_identity(
        conn: &Connection,
        asset_code: &str,
        serial_number: Option<&str>,
        status: &str,
    ) -> i64 {
        conn.execute(
            r#"
            INSERT INTO assets(asset_code, asset_type, display_name, serial_number, status, created_at, updated_at)
            VALUES(?, 'Laptop', ?, ?, ?, datetime('now'), datetime('now'))
            "#,
            params![asset_code, asset_code, serial_number, status],
        )
        .expect("insert asset");

        conn.last_insert_rowid()
    }

    fn sample_mapping() -> AssetImportFieldMapping {
        AssetImportFieldMapping {
            asset_code: Some("Asset Code".to_string()),
            asset_type: Some("Asset Type".to_string()),
            display_name: Some("Display Name".to_string()),
            computer_name: None,
            usage_location: None,
            brand: Some("Brand".to_string()),
            model: Some("Model".to_string()),
            serial_number: Some("Serial Number".to_string()),
            adapter_number: None,
            quantity: Some("Quantity".to_string()),
            warehouse: Some("Warehouse".to_string()),
            notes: Some("Notes".to_string()),
        }
    }

    fn sample_headers() -> Vec<String> {
        vec![
            "Asset Code".to_string(),
            "Asset Type".to_string(),
            "Display Name".to_string(),
            "Brand".to_string(),
            "Model".to_string(),
            "Serial Number".to_string(),
            "Quantity".to_string(),
            "Warehouse".to_string(),
            "Notes".to_string(),
        ]
    }

    fn row(
        row_number: i64,
        asset_code: &str,
        asset_type: &str,
        display_name: &str,
    ) -> AssetImportRowSeedInput {
        AssetImportRowSeedInput {
            row_number,
            raw_values: vec![
                AssetImportRawValue {
                    header: "Asset Code".to_string(),
                    value: asset_code.to_string(),
                },
                AssetImportRawValue {
                    header: "Asset Type".to_string(),
                    value: asset_type.to_string(),
                },
                AssetImportRawValue {
                    header: "Display Name".to_string(),
                    value: display_name.to_string(),
                },
                AssetImportRawValue {
                    header: "Brand".to_string(),
                    value: "Dell".to_string(),
                },
                AssetImportRawValue {
                    header: "Model".to_string(),
                    value: "7440".to_string(),
                },
                AssetImportRawValue {
                    header: "Serial Number".to_string(),
                    value: format!("SN-{row_number:03}"),
                },
                AssetImportRawValue {
                    header: "Warehouse".to_string(),
                    value: "HCM".to_string(),
                },
                AssetImportRawValue {
                    header: "Notes".to_string(),
                    value: "Initial import".to_string(),
                },
            ],
            asset_code: Some(asset_code.to_string()),
            asset_type: Some(asset_type.to_string()),
            display_name: Some(display_name.to_string()),
            display_name_short: None,
            computer_name: Some(format!("ASW{}", asset_code.to_ascii_uppercase())),
            brand: Some("Dell".to_string()),
            model: Some("7440".to_string()),
            serial_number: Some(format!("SN-{row_number:03}")),
            adapter_number: None,
            quantity: None,
            warehouse: Some("HCM".to_string()),
            usage_location: None,
            notes: Some("Initial import".to_string()),
            submitted_staff_id: None,
            submitted_full_name: None,
            submitted_team: None,
            submitted_phone_number: None,
        }
    }

    fn row_without_asset_code(
        row_number: i64,
        asset_type: &str,
        display_name: &str,
    ) -> AssetImportRowSeedInput {
        AssetImportRowSeedInput {
            row_number,
            raw_values: vec![
                AssetImportRawValue {
                    header: "Asset Type".to_string(),
                    value: asset_type.to_string(),
                },
                AssetImportRawValue {
                    header: "Display Name".to_string(),
                    value: display_name.to_string(),
                },
                AssetImportRawValue {
                    header: "Brand".to_string(),
                    value: "Dell".to_string(),
                },
                AssetImportRawValue {
                    header: "Model".to_string(),
                    value: "7440".to_string(),
                },
                AssetImportRawValue {
                    header: "Serial Number".to_string(),
                    value: format!("SN-{row_number:03}"),
                },
                AssetImportRawValue {
                    header: "Warehouse".to_string(),
                    value: "HCM".to_string(),
                },
                AssetImportRawValue {
                    header: "Notes".to_string(),
                    value: "Initial import".to_string(),
                },
            ],
            asset_code: None,
            asset_type: Some(asset_type.to_string()),
            display_name: Some(display_name.to_string()),
            display_name_short: None,
            computer_name: None,
            brand: Some("Dell".to_string()),
            model: Some("7440".to_string()),
            serial_number: Some(format!("SN-{row_number:03}")),
            adapter_number: None,
            quantity: None,
            warehouse: Some("HCM".to_string()),
            usage_location: None,
            notes: Some("Initial import".to_string()),
            submitted_staff_id: None,
            submitted_full_name: None,
            submitted_team: None,
            submitted_phone_number: None,
        }
    }

    fn quantity_row_without_asset_code(
        row_number: i64,
        asset_type: &str,
        display_name: &str,
        quantity: &str,
    ) -> AssetImportRowSeedInput {
        AssetImportRowSeedInput {
            row_number,
            raw_values: vec![
                AssetImportRawValue {
                    header: "Asset Type".to_string(),
                    value: asset_type.to_string(),
                },
                AssetImportRawValue {
                    header: "Display Name".to_string(),
                    value: display_name.to_string(),
                },
                AssetImportRawValue {
                    header: "Brand".to_string(),
                    value: "Logitech".to_string(),
                },
                AssetImportRawValue {
                    header: "Quantity".to_string(),
                    value: quantity.to_string(),
                },
                AssetImportRawValue {
                    header: "Warehouse".to_string(),
                    value: "HCM".to_string(),
                },
                AssetImportRawValue {
                    header: "Notes".to_string(),
                    value: "Initial import".to_string(),
                },
            ],
            asset_code: None,
            asset_type: Some(asset_type.to_string()),
            display_name: Some(display_name.to_string()),
            display_name_short: None,
            computer_name: None,
            brand: Some("Logitech".to_string()),
            model: None,
            serial_number: None,
            adapter_number: None,
            quantity: Some(quantity.to_string()),
            warehouse: Some("HCM".to_string()),
            usage_location: None,
            notes: Some("Initial import".to_string()),
            submitted_staff_id: None,
            submitted_full_name: None,
            submitted_team: None,
            submitted_phone_number: None,
        }
    }

    fn sample_batch(
        import_type: AssetImportMode,
        rows: Vec<AssetImportRowSeedInput>,
    ) -> AssetImportBatchSeedInput {
        AssetImportBatchSeedInput {
            import_type,
            source_file_name: "assets.csv".to_string(),
            source_file_path: "C:\\temp\\assets.csv".to_string(),
            source_file_type: "csv".to_string(),
            sheet_name: None,
            header_row: 1,
            headers: sample_headers(),
            mapping: sample_mapping(),
            rows,
        }
    }

    fn seed_employee(
        conn: &Connection,
        employee_id: &str,
        full_name: &str,
        team_name: &str,
        staff_group: &str,
    ) -> i64 {
        conn.execute(
            "INSERT OR IGNORE INTO teams(name) VALUES (?)",
            params![team_name],
        )
        .expect("insert team");
        let team_id: i64 = conn
            .query_row(
                "SELECT id FROM teams WHERE name = ?",
                params![team_name],
                |row| row.get(0),
            )
            .expect("load team id");

        conn.execute(
            r#"
            INSERT INTO employees(
              employee_id,
              full_name,
              team_id,
              staff_group,
              updated_at
            )
            VALUES(?, ?, ?, ?, datetime('now'))
            "#,
            params![employee_id, full_name, team_id, staff_group],
        )
        .expect("insert employee");

        conn.last_insert_rowid()
    }

    fn monitor_owner_row(
        row_number: i64,
        submitted_staff_id: &str,
        submitted_full_name: &str,
        asset_code: &str,
        usage_location: Option<&str>,
        display_name: &str,
    ) -> AssetImportRowSeedInput {
        AssetImportRowSeedInput {
            row_number,
            raw_values: vec![
                AssetImportRawValue {
                    header: "StaffID".to_string(),
                    value: submitted_staff_id.to_string(),
                },
                AssetImportRawValue {
                    header: "TÃªn NhÃ¢n ViÃªn".to_string(),
                    value: submitted_full_name.to_string(),
                },
                AssetImportRawValue {
                    header: "Asset code".to_string(),
                    value: asset_code.to_string(),
                },
                AssetImportRawValue {
                    header: "Category".to_string(),
                    value: "Monitor".to_string(),
                },
                AssetImportRawValue {
                    header: "Usuage Location".to_string(),
                    value: usage_location.unwrap_or_default().to_string(),
                },
                AssetImportRawValue {
                    header: "Asset Name".to_string(),
                    value: display_name.to_string(),
                },
                AssetImportRawValue {
                    header: "Model".to_string(),
                    value: "LG 27".to_string(),
                },
            ],
            asset_code: Some(asset_code.to_string()),
            asset_type: Some("Monitor".to_string()),
            display_name: Some(display_name.to_string()),
            display_name_short: Some(display_name.to_string()),
            computer_name: Some(format!("ASW{}", asset_code.to_ascii_uppercase())),
            brand: Some("LG".to_string()),
            model: Some("LG 27".to_string()),
            serial_number: None,
            adapter_number: None,
            quantity: None,
            warehouse: None,
            usage_location: usage_location.map(str::to_string),
            notes: Some("Monitor import".to_string()),
            submitted_staff_id: Some(submitted_staff_id.to_string()),
            submitted_full_name: Some(submitted_full_name.to_string()),
            submitted_team: None,
            submitted_phone_number: None,
        }
    }

    fn owner_row(
        row_number: i64,
        submitted_staff_id: &str,
        submitted_full_name: &str,
        submitted_team: &str,
        asset_code: &str,
    ) -> AssetImportRowSeedInput {
        AssetImportRowSeedInput {
            row_number,
            raw_values: vec![
                AssetImportRawValue {
                    header: "StaffID".to_string(),
                    value: submitted_staff_id.to_string(),
                },
                AssetImportRawValue {
                    header: "Tên Nhân Viên".to_string(),
                    value: submitted_full_name.to_string(),
                },
                AssetImportRawValue {
                    header: "Team".to_string(),
                    value: submitted_team.to_string(),
                },
                AssetImportRawValue {
                    header: "Phone Number".to_string(),
                    value: "0900000000".to_string(),
                },
                AssetImportRawValue {
                    header: "Assetcode".to_string(),
                    value: asset_code.to_string(),
                },
                AssetImportRawValue {
                    header: "Category".to_string(),
                    value: "Laptop".to_string(),
                },
                AssetImportRawValue {
                    header: "Asset Name".to_string(),
                    value: format!("ASW{asset_code}"),
                },
                AssetImportRawValue {
                    header: "Model".to_string(),
                    value: "Dell Latitude 7440".to_string(),
                },
                AssetImportRawValue {
                    header: "Serrial Number".to_string(),
                    value: format!("SN-{row_number:03}"),
                },
            ],
            asset_code: Some(asset_code.to_string()),
            asset_type: Some("Laptop".to_string()),
            display_name: Some(format!("ASW{asset_code}")),
            display_name_short: None,
            computer_name: Some(format!("ASW{asset_code}")),
            brand: Some("Dell".to_string()),
            model: Some("Dell Latitude 7440".to_string()),
            serial_number: Some(format!("SN-{row_number:03}")),
            adapter_number: None,
            quantity: None,
            warehouse: Some("HCM".to_string()),
            usage_location: None,
            notes: Some("Issued laptop".to_string()),
            submitted_staff_id: Some(submitted_staff_id.to_string()),
            submitted_full_name: Some(submitted_full_name.to_string()),
            submitted_team: Some(submitted_team.to_string()),
            submitted_phone_number: Some("0900000000".to_string()),
        }
    }

    #[test]
    fn parse_csv_source_preserves_asset_list_owner_snapshot_and_raw_adapter_column() {
        let csv_path = write_temp_csv(
            "asset-list-serialized",
            &[
                "StaffID,Tên Nhân Viên,Team,Phone Number,Assetcode,Category,Computer Name,Asset Name,Model,Serrial Number,Adapter number,Note",
                "ASW1302,Lư Thế Hùng,ExamWorks,0909154452,VNLAP235,Laptop,ASWVNLAP235,ASWVNLAP235,Dell Latitude 3520,7900LG3,7900LG3,",
                ",,,,VNLAP502,Laptop,ASWVNLAP502,ASWVNLAP502,Lenovo E16,,,",
            ],
        );

        let parsed =
            parse_asset_import_source(csv_path.as_path(), AssetImportMode::Serialized, None, None)
                .expect("parse asset list style csv");

        assert_eq!(parsed.auto_mapping.asset_code.as_deref(), Some("Assetcode"));
        assert_eq!(
            parsed.auto_mapping.serial_number.as_deref(),
            Some("Serrial Number")
        );
        assert_eq!(parsed.rows.len(), 2);
        assert_eq!(
            parsed.rows[0].submitted_staff_id.as_deref(),
            Some("ASW1302")
        );
        assert_eq!(parsed.rows[1].submitted_staff_id, None);
        assert_eq!(parsed.rows[0].asset_code.as_deref(), Some("VNLAP235"));
        assert!(parsed.rows[0]
            .raw_values
            .iter()
            .any(|item| item.header == "Adapter number" && item.value == "7900LG3"));

        let _ = fs::remove_file(csv_path);
    }

    #[test]
    fn parse_csv_source_uses_computer_name_as_display_name_when_asset_name_is_missing() {
        let csv_path = write_temp_csv(
            "asset-list-no-asset-name",
            &[
                "StaffID,Team,Phone Number,Assetcode,Category,Computer Name,Model,Serrial Number",
                "ASWVN1001,Medhealth Team,0908207111,VNLAP518,Laptop,ASWVNLAP518,Lenovo E14,PF-5MBN9A",
            ],
        );

        let parsed =
            parse_asset_import_source(csv_path.as_path(), AssetImportMode::Serialized, None, None)
                .expect("parse asset list without asset name");

        assert_eq!(
            parsed.auto_mapping.display_name.as_deref(),
            Some("Computer Name")
        );
        assert_eq!(
            parsed.auto_mapping.computer_name.as_deref(),
            Some("Computer Name")
        );
        assert_eq!(parsed.rows[0].display_name.as_deref(), Some("ASWVNLAP518"));
        assert_eq!(parsed.rows[0].computer_name.as_deref(), Some("ASWVNLAP518"));

        let _ = fs::remove_file(csv_path);
    }

    #[test]
    fn import_valid_rows_persists_explicit_computer_name_and_adapter_number_from_workbook() {
        let mut conn = open_test_connection();
        let csv_path = write_temp_csv(
            "asset-list-computer-name",
            &[
                "Assetcode,Category,Computer Name,Asset Name,Model,Serrial Number,Adapter number,Note",
                "VNLAP235,Laptop,ASWVNLAP235,ASWVNLAP235,Dell Latitude 3520,7900LG3,7900LG3,",
            ],
        );

        let parsed =
            parse_asset_import_source(csv_path.as_path(), AssetImportMode::Serialized, None, None)
                .expect("parse workbook with explicit computer name");

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            AssetImportBatchSeedInput {
                import_type: AssetImportMode::Serialized,
                source_file_name: parsed.source_file_name,
                source_file_path: parsed.source_file_path,
                source_file_type: parsed.source_file_type,
                sheet_name: parsed.sheet_name,
                header_row: parsed.header_row,
                headers: parsed.headers,
                mapping: parsed.auto_mapping,
                rows: parsed.rows,
            },
        )
        .expect("create parsed serialized batch");

        let result = import_asset_import_batch_valid_rows_conn(&mut conn, batch.summary.id)
            .expect("import parsed serialized batch");

        assert_eq!(result.imported_count, 1);

        let dashboard_row = crate::db::asset::list_asset_dashboard_serialized_conn(&conn)
            .expect("load serialized dashboard rows")
            .into_iter()
            .find(|row| row.asset_code == "VNLAP235")
            .expect("find imported laptop in dashboard rows");

        assert_eq!(dashboard_row.computer_name.as_deref(), Some("ASWVNLAP235"));
        assert_eq!(dashboard_row.adapter_number.as_deref(), Some("7900LG3"));

        let status: String = conn
            .query_row(
                "SELECT status FROM assets WHERE asset_code = ?",
                params!["VNLAP235"],
                |row| row.get(0),
            )
            .expect("load imported asset status");
        assert_eq!(status, "in_stock");

        let _ = fs::remove_file(csv_path);
    }

    #[test]
    fn import_valid_rows_falls_back_to_asw_asset_code_when_workbook_computer_name_is_blank() {
        let mut conn = open_test_connection();
        let csv_path = write_temp_csv(
            "asset-list-computer-name-fallback",
            &[
                "Assetcode,Category,Computer Name,Asset Name,Model,Serrial Number,Adapter number,Note",
                "VNLAP502,Laptop,,ASWVNLAP502,Lenovo E16,PF-5MBN9A,56K33KS,",
            ],
        );

        let parsed =
            parse_asset_import_source(csv_path.as_path(), AssetImportMode::Serialized, None, None)
                .expect("parse workbook with blank computer name");

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            AssetImportBatchSeedInput {
                import_type: AssetImportMode::Serialized,
                source_file_name: parsed.source_file_name,
                source_file_path: parsed.source_file_path,
                source_file_type: parsed.source_file_type,
                sheet_name: parsed.sheet_name,
                header_row: parsed.header_row,
                headers: parsed.headers,
                mapping: parsed.auto_mapping,
                rows: parsed.rows,
            },
        )
        .expect("create parsed serialized batch");

        let result = import_asset_import_batch_valid_rows_conn(&mut conn, batch.summary.id)
            .expect("import parsed serialized batch");

        assert_eq!(result.imported_count, 1);

        let dashboard_row = crate::db::asset::list_asset_dashboard_serialized_conn(&conn)
            .expect("load serialized dashboard rows")
            .into_iter()
            .find(|row| row.asset_code == "VNLAP502")
            .expect("find imported laptop in dashboard rows");

        assert_eq!(dashboard_row.computer_name.as_deref(), Some("ASWVNLAP502"));
        assert_eq!(dashboard_row.adapter_number.as_deref(), Some("56K33KS"));

        let _ = fs::remove_file(csv_path);
    }

    #[test]
    fn parse_csv_source_normalizes_monitor_usage_location_and_short_name() {
        let csv_path = write_temp_csv(
            "monitor-serialized",
            &[
                "StaffID,Tên Nhân Viên,Team,Phone Number,Asset code,Category,Usuage Location,Asset Name,Model,Serrial Number,Adapter number,Note",
                "ASWVN1134,TRẦN GIA THÀNH,,,VNMON709,Monitor,Tại CTY (Vui Lòng Bảo Quản Cẩn Thận), Mon709 ,LG 27,,,",
            ],
        );

        let parsed =
            parse_asset_import_source(csv_path.as_path(), AssetImportMode::Serialized, None, None)
                .expect("parse monitor style csv");

        assert_eq!(
            parsed.auto_mapping.usage_location.as_deref(),
            Some("Usuage Location")
        );
        assert_eq!(parsed.rows[0].usage_location.as_deref(), Some("office"));
        assert_eq!(parsed.rows[0].display_name.as_deref(), Some("Mon709"));
        assert_eq!(parsed.rows[0].display_name_short.as_deref(), Some("Mon709"));

        let _ = fs::remove_file(csv_path);
    }

    #[test]
    fn import_monitor_owner_sheet_with_typo_category_and_generic_name_columns() {
        let mut conn = open_test_connection();
        seed_employee(
            &conn,
            "ASWVN1004",
            "Nguyen Thi Thu Trang",
            "Operations",
            "employee_list",
        );

        let csv_path = write_temp_csv(
            "monitor-owner-mon-workbook",
            &[
                "STAFF ID,CATEAGORY,Name,LOCATION,Asset name,Aset name 2",
                "ASWVN1004,Monitor,Nguyen Thi Thu Trang,Tại NHÀ,MON475,",
            ],
        );

        let parsed =
            parse_asset_import_source(csv_path.as_path(), AssetImportMode::Serialized, None, None)
                .expect("parse monitor owner sheet");

        assert_eq!(parsed.auto_mapping.asset_type.as_deref(), Some("CATEAGORY"));
        assert_eq!(
            parsed.auto_mapping.display_name.as_deref(),
            Some("Asset name")
        );
        assert_eq!(
            parsed.auto_mapping.usage_location.as_deref(),
            Some("LOCATION")
        );
        assert_eq!(parsed.rows[0].asset_code.as_deref(), Some("VNMON475"));
        assert_eq!(parsed.rows[0].display_name.as_deref(), Some("MON475"));
        assert_eq!(parsed.rows[0].usage_location.as_deref(), Some("home"));
        assert_eq!(
            parsed.rows[0].submitted_full_name.as_deref(),
            Some("Nguyen Thi Thu Trang")
        );

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            AssetImportBatchSeedInput {
                import_type: AssetImportMode::Serialized,
                source_file_name: parsed.source_file_name,
                source_file_path: parsed.source_file_path,
                source_file_type: parsed.source_file_type,
                sheet_name: parsed.sheet_name,
                header_row: parsed.header_row,
                headers: parsed.headers,
                mapping: parsed.auto_mapping,
                rows: parsed.rows,
            },
        )
        .expect("create parsed monitor owner batch");

        assert_eq!(batch.summary.valid_rows, 1);
        assert_eq!(batch.summary.error_rows, 0);

        let result = import_asset_import_batch_valid_rows_conn(&mut conn, batch.summary.id)
            .expect("import parsed monitor owner batch");
        assert_eq!(result.imported_count, 1);

        let dashboard_row = crate::db::asset::list_asset_dashboard_serialized_conn(&conn)
            .expect("load serialized dashboard rows")
            .into_iter()
            .find(|row| row.asset_code == "VNMON475")
            .expect("find imported monitor in dashboard rows");

        assert_eq!(dashboard_row.display_name, "MON475");
        assert_eq!(dashboard_row.display_name_short.as_deref(), Some("MON475"));
        assert_eq!(dashboard_row.usage_location.as_deref(), Some("home"));
        assert_eq!(
            dashboard_row.holder_employee_id.as_deref(),
            Some("ASWVN1004")
        );

        let _ = fs::remove_file(csv_path);
    }

    #[test]
    fn parse_legacy_asset_id_uses_high_confidence_asset_name_tag_instead_of_numeric_id() {
        let csv_path = write_temp_csv(
            "legacy-asset-id-tag",
            &[
                "Asset ID,Asset Name,Category,Computer Name,Model",
                "504,VNLAP504,Laptop,ASWVNLAP504,ThinkPad E16",
            ],
        );

        let parsed =
            parse_asset_import_source(csv_path.as_path(), AssetImportMode::Serialized, None, None)
                .expect("parse legacy asset row");

        assert_eq!(parsed.rows[0].asset_code.as_deref(), Some("VNLAP504"));
        assert_ne!(parsed.rows[0].asset_code.as_deref(), Some("504"));

        let _ = fs::remove_file(csv_path);
    }

    #[test]
    fn canonical_asset_tag_header_is_the_serialized_business_key() {
        let csv_path = write_temp_csv(
            "canonical-asset-tag",
            &[
                "Asset Tag,Asset Name,Category,Serial Number",
                "VNLAP504,Lenovo ThinkPad E16,Laptop,",
            ],
        );

        let parsed =
            parse_asset_import_source(csv_path.as_path(), AssetImportMode::Serialized, None, None)
                .expect("parse canonical asset tag row");

        assert_eq!(parsed.auto_mapping.asset_code.as_deref(), Some("Asset Tag"));
        assert_eq!(parsed.rows[0].asset_code.as_deref(), Some("VNLAP504"));
        assert_eq!(parsed.rows[0].serial_number, None);

        let _ = fs::remove_file(csv_path);
    }

    #[test]
    fn asset_id_without_legacy_computer_name_is_rejected_during_staging() {
        let csv_path = write_temp_csv(
            "asset-id-without-legacy-proof",
            &["Asset ID,Asset Name,Category", "504,VNLAP504,Laptop"],
        );

        let parsed =
            parse_asset_import_source(csv_path.as_path(), AssetImportMode::Serialized, None, None)
                .expect("parse ambiguous legacy row");
        let mut conn = open_test_connection();
        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            AssetImportBatchSeedInput {
                import_type: AssetImportMode::Serialized,
                source_file_name: parsed.source_file_name,
                source_file_path: parsed.source_file_path,
                source_file_type: parsed.source_file_type,
                sheet_name: parsed.sheet_name,
                header_row: parsed.header_row,
                headers: parsed.headers,
                mapping: parsed.auto_mapping,
                rows: parsed.rows,
            },
        )
        .expect("stage ambiguous legacy row");

        assert_eq!(batch.rows[0].asset_code, None);
        assert!(batch.rows[0]
            .validation_errors
            .iter()
            .any(|item| item.contains("assetCode")));
        let _ = fs::remove_file(csv_path);
    }

    #[test]
    fn legacy_asset_id_row_derives_tag_only_from_matching_computer_name() {
        let csv_path = write_temp_csv(
            "legacy-asset-id-with-computer-name",
            &[
                "Asset ID,Asset Name,Category,Computer Name",
                "504,VNLAP504,Laptop,ASWVNLAP504",
            ],
        );

        let parsed =
            parse_asset_import_source(csv_path.as_path(), AssetImportMode::Serialized, None, None)
                .expect("parse compatible legacy asset row");

        assert_eq!(parsed.rows[0].asset_code.as_deref(), Some("VNLAP504"));
        assert_eq!(parsed.rows[0].computer_name.as_deref(), Some("ASWVNLAP504"));

        let _ = fs::remove_file(csv_path);
    }

    #[test]
    fn empty_serial_number_is_allowed_for_serialized_assets() {
        let mut conn = open_test_connection();
        let mut asset = row(2, "VNLAP504", "Laptop", "Lenovo ThinkPad E16");
        asset.serial_number = None;
        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(AssetImportMode::Serialized, vec![asset]),
        )
        .expect("create serialized batch without a serial number");

        assert_eq!(batch.rows[0].serial_number, None);
        assert_eq!(batch.rows[0].status, "valid");
    }

    #[test]
    fn duplicate_serial_numbers_are_rejected_in_batch_and_against_assets() {
        let mut conn = open_test_connection();
        conn.execute(
            "INSERT INTO assets(asset_code, asset_type, display_name, serial_number, status) VALUES('VNLAP-EXISTING', 'Laptop', 'Existing', 'SN-EXISTING', 'in_stock')",
            [],
        )
        .expect("seed existing serial number");

        let mut first = row(2, "VNLAP504", "Laptop", "Laptop A");
        first.serial_number = Some("SN-DUP".to_string());
        let mut second = row(3, "VNLAP505", "Laptop", "Laptop B");
        second.serial_number = Some("sn-dup".to_string());
        let mut existing = row(4, "VNLAP506", "Laptop", "Laptop C");
        existing.serial_number = Some("sn-existing".to_string());

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(AssetImportMode::Serialized, vec![first, second, existing]),
        )
        .expect("create serialized batch with duplicate serial numbers");

        assert_eq!(batch.summary.valid_rows, 0);
        assert_eq!(batch.summary.skipped_rows, 0);
        assert_eq!(batch.summary.error_rows, 3);
        assert!(batch.rows[0]
            .validation_errors
            .iter()
            .any(|item| item.contains("duplicated")));
        assert!(batch.rows[1]
            .validation_errors
            .iter()
            .any(|item| item.contains("duplicated")));
        assert!(batch.rows[2]
            .validation_errors
            .iter()
            .any(|item| item.contains("Conflict")));
    }

    #[test]
    fn exact_existing_asset_is_classified_as_skipped_without_validation_error() {
        let mut conn = open_test_connection();
        seed_asset_with_identity(&conn, "VNLAP504", Some("SN-504"), "in_stock");
        let mut existing = row(504, "VNLAP504", "Laptop", "Lenovo ThinkPad E16");
        existing.serial_number = Some("SN-504".to_string());

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(AssetImportMode::Serialized, vec![existing]),
        )
        .expect("create exact existing asset batch");

        assert_eq!(batch.summary.total_rows, 1);
        assert_eq!(batch.summary.valid_rows, 0);
        assert_eq!(batch.summary.skipped_rows, 1);
        assert_eq!(batch.summary.error_rows, 0);
        assert_eq!(batch.rows[0].status, "skipped");
        assert!(batch.rows[0].validation_errors.is_empty());
    }

    #[test]
    fn existing_asset_with_compatible_serial_data_is_skipped() {
        let mut conn = open_test_connection();
        seed_asset_with_identity(&conn, "VNLAP504", Some("SN-504"), "in_stock");
        seed_asset_with_identity(&conn, "VNLAP505", None, "in_stock");

        let mut code_only = row(504, "VNLAP504", "Laptop", "Lenovo ThinkPad E16");
        code_only.serial_number = None;
        let mut incoming_serial = row(505, "VNLAP505", "Laptop", "Lenovo ThinkPad E16");
        incoming_serial.serial_number = Some("SN-505".to_string());

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![code_only, incoming_serial],
            ),
        )
        .expect("create compatible existing asset batch");

        assert_eq!(batch.summary.valid_rows, 0);
        assert_eq!(batch.summary.skipped_rows, 2);
        assert_eq!(batch.summary.error_rows, 0);
        assert!(batch
            .rows
            .iter()
            .all(|item| item.status == "skipped" && item.validation_errors.is_empty()));
    }

    #[test]
    fn asset_code_and_serial_cross_identity_conflict_is_an_error() {
        let mut conn = open_test_connection();
        seed_asset_with_identity(&conn, "ASSET-A", Some("SN-A"), "in_stock");
        seed_asset_with_identity(&conn, "ASSET-B", Some("SN-B"), "in_stock");

        let mut conflict = row(2, "ASSET-A", "Laptop", "Conflicting Laptop");
        conflict.serial_number = Some("SN-B".to_string());

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(AssetImportMode::Serialized, vec![conflict]),
        )
        .expect("create conflicting asset batch");

        assert_eq!(batch.summary.valid_rows, 0);
        assert_eq!(batch.summary.skipped_rows, 0);
        assert_eq!(batch.summary.error_rows, 1);
        assert_eq!(batch.rows[0].status, "error");
        assert!(batch.rows[0]
            .validation_errors
            .iter()
            .any(|item| item.contains("Conflict") || item.contains("conflict")));
    }

    #[test]
    fn mixed_six_existing_and_four_new_assets_has_six_skips_and_four_new_rows() {
        let mut conn = open_test_connection();
        let existing_codes = [
            "VNLAP504", "VNLAP505", "VNLAP506", "VNLAP523", "VNLAP526", "VNLAP536",
        ];
        let new_codes = ["VNHPH001", "VNHPH002", "VNHPH003", "VNHPH004"];
        let mut rows = Vec::new();

        for (index, asset_code) in existing_codes.iter().enumerate() {
            let row_number = (index + 2) as i64;
            let serial_number = format!("SN-{row_number:03}");
            seed_asset_with_identity(&conn, asset_code, Some(serial_number.as_str()), "in_stock");
            rows.push(row(row_number, asset_code, "Laptop", "Lenovo Laptop"));
        }
        for (index, asset_code) in new_codes.iter().enumerate() {
            let row_number = (index + 8) as i64;
            rows.push(row(row_number, asset_code, "Laptop", "Samsung Laptop"));
        }

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(AssetImportMode::Serialized, rows),
        )
        .expect("create mixed asset batch");

        assert_eq!(batch.summary.total_rows, 10);
        assert_eq!(batch.summary.valid_rows, 4);
        assert_eq!(batch.summary.skipped_rows, 6);
        assert_eq!(batch.summary.error_rows, 0);
        assert_eq!(
            batch
                .rows
                .iter()
                .filter(|item| item.status == "skipped")
                .count(),
            6
        );
        assert_eq!(
            batch
                .rows
                .iter()
                .filter(|item| item.status == "valid")
                .count(),
            4
        );
    }

    #[test]
    fn mixed_asset_import_commits_only_new_rows_and_reimport_is_zero_write() {
        let mut conn = open_test_connection();
        let existing_codes = [
            "VNLAP504", "VNLAP505", "VNLAP506", "VNLAP523", "VNLAP526", "VNLAP536",
        ];
        let new_codes = ["VNHPH001", "VNHPH002", "VNHPH003", "VNHPH004"];
        let mut rows = Vec::new();
        let mut existing_asset_id = 0_i64;

        for (index, asset_code) in existing_codes.iter().enumerate() {
            let row_number = (index + 2) as i64;
            let serial_number = format!("SN-{row_number:03}");
            let status = if *asset_code == "VNLAP504" {
                "assigned"
            } else {
                "in_stock"
            };
            let asset_id =
                seed_asset_with_identity(&conn, asset_code, Some(serial_number.as_str()), status);
            if *asset_code == "VNLAP504" {
                existing_asset_id = asset_id;
            }
            rows.push(row(row_number, asset_code, "Laptop", "Lenovo Laptop"));
        }
        for (index, asset_code) in new_codes.iter().enumerate() {
            let row_number = (index + 8) as i64;
            rows.push(row(row_number, asset_code, "Laptop", "Samsung Laptop"));
        }

        let employee_row_id = seed_employee(
            &conn,
            "EE-REIMPORT",
            "Reimport Holder",
            "Reimport Team",
            "employee_list",
        );
        conn.execute(
            r#"
            INSERT INTO borrow_requests(
              request_key, employee_id_fk, submitted_employee_id, submitted_full_name,
              status, request_type
            )
            VALUES('REIMPORT-ACTIVE', ?, 'EE-REIMPORT', 'Reimport Holder', 'approved', 'borrow')
            "#,
            params![employee_row_id],
        )
        .expect("insert active loan request");
        let request_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO borrow_request_items(borrow_request_id, asset_id, asset_code_snapshot) VALUES(?, ?, 'VNLAP504')",
            params![request_id, existing_asset_id],
        )
        .expect("insert active loan request item");
        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id) VALUES(?, ?, ?)",
            params![existing_asset_id, employee_row_id, request_id],
        )
        .expect("insert active loan");

        let input = sample_batch(AssetImportMode::Serialized, rows);
        let preview = preview_asset_import_seed_conn(&mut conn, input.clone())
            .expect("preview mixed reimport batch");
        assert_eq!(preview.total_rows, 10);
        assert_eq!(preview.valid_rows, 4);
        assert_eq!(preview.skipped_rows, 6);
        assert_eq!(preview.error_rows, 0);

        let batch = create_asset_import_batch_seed_conn(&mut conn, input)
            .expect("create mixed reimport batch");
        let result = import_asset_import_batch_valid_rows_conn(&mut conn, batch.summary.id)
            .expect("commit only new reimport rows");
        assert_eq!(result.imported_count, 4);
        assert_eq!(
            result.imported_asset_codes,
            new_codes
                .iter()
                .map(|code| code.to_string())
                .collect::<Vec<_>>()
        );

        let asset_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))
            .expect("count assets after mixed import");
        assert_eq!(asset_count, 10);
        let existing_state: (String, String, String, i64) = conn
            .query_row(
                r#"
                SELECT a.status, a.display_name, a.serial_number,
                       (SELECT COUNT(*) FROM asset_loans l WHERE l.asset_id = a.id AND l.returned_at IS NULL)
                FROM assets a WHERE a.asset_code = 'VNLAP504'
                "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("load existing asset after mixed import");
        assert_eq!(
            existing_state,
            (
                "assigned".to_string(),
                "VNLAP504".to_string(),
                "SN-002".to_string(),
                1
            )
        );

        let second_batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                existing_codes
                    .iter()
                    .enumerate()
                    .map(|(index, code)| row((index + 2) as i64, code, "Laptop", "Lenovo Laptop"))
                    .chain(new_codes.iter().enumerate().map(|(index, code)| {
                        row((index + 8) as i64, code, "Laptop", "Samsung Laptop")
                    }))
                    .collect(),
            ),
        )
        .expect("create second exact reimport batch");
        assert_eq!(second_batch.summary.valid_rows, 0);
        assert_eq!(second_batch.summary.skipped_rows, 10);
        assert_eq!(second_batch.summary.error_rows, 0);

        let second_result =
            import_asset_import_batch_valid_rows_conn(&mut conn, second_batch.summary.id)
                .expect("approve existing-only reimport batch");
        assert_eq!(second_result.imported_count, 0);
        let final_asset_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))
            .expect("count assets after zero-write reimport");
        assert_eq!(final_asset_count, 10);
    }

    #[test]
    fn parse_csv_source_stages_quantity_rows_from_quantity_header_with_trailing_space() {
        let csv_path = write_temp_csv(
            "mouse-key-quantity",
            &[
                "Assetcode,Category,Asset Name,Model,Quantity ,Note",
                "VNMouse,Mouse,Mouse Logi,Logitech G102,50,",
            ],
        );

        let parsed =
            parse_asset_import_source(csv_path.as_path(), AssetImportMode::Quantity, None, None)
                .expect("parse quantity style csv");

        assert_eq!(parsed.auto_mapping.asset_code.as_deref(), Some("Assetcode"));
        assert_eq!(parsed.auto_mapping.quantity.as_deref(), Some("Quantity"));
        assert_eq!(parsed.rows[0].quantity.as_deref(), Some("50"));
        assert!(parsed.rows[0]
            .raw_values
            .iter()
            .any(|item| item.header == "Quantity" && item.value == "50"));

        let _ = fs::remove_file(csv_path);
    }

    #[test]
    fn create_batch_marks_duplicate_asset_codes_inside_same_batch_as_errors() {
        let mut conn = open_test_connection();

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![
                    row(2, "asset-001", "Laptop", "Dell Latitude 7440"),
                    row(3, "ASSET-001", "Laptop", "Dell Latitude 7450"),
                ],
            ),
        )
        .expect("create asset import batch");

        assert_eq!(batch.summary.total_rows, 2);
        assert_eq!(batch.summary.valid_rows, 0);
        assert_eq!(batch.summary.error_rows, 2);
        assert!(
            batch.rows.iter().all(|item| item.status == "error"),
            "all duplicate rows should stay in error state"
        );
    }

    #[test]
    fn create_batch_marks_existing_asset_codes_against_main_assets_table_as_skipped() {
        let mut conn = open_test_connection();
        seed_asset(&conn, "ASSET-EXISTING");

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![row(2, "asset-existing", "Laptop", "Dell Latitude 7440")],
            ),
        )
        .expect("create asset import batch");

        assert_eq!(batch.summary.total_rows, 1);
        assert_eq!(batch.summary.valid_rows, 0);
        assert_eq!(batch.summary.skipped_rows, 1);
        assert_eq!(batch.summary.error_rows, 0);
        assert_eq!(batch.rows[0].status, "skipped");
        assert!(batch.rows[0].validation_errors.is_empty());
    }

    #[test]
    fn create_batch_rejects_serialized_rows_without_asset_code_during_staging() {
        let mut conn = open_test_connection();

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![row_without_asset_code(2, "Laptop", "Dell Latitude 7440")],
            ),
        )
        .expect("create serialized asset import batch without asset code");

        assert_eq!(batch.summary.total_rows, 1);
        assert_eq!(batch.summary.valid_rows, 0);
        assert_eq!(batch.summary.error_rows, 1);
        assert_eq!(batch.rows[0].status, "error");
        assert_eq!(batch.rows[0].asset_code, None);
        assert!(batch.rows[0]
            .validation_errors
            .iter()
            .any(|item| item.contains("assetCode is required")));
    }

    #[test]
    fn create_batch_allows_quantity_rows_without_asset_code_during_staging() {
        let mut conn = open_test_connection();

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Quantity,
                vec![quantity_row_without_asset_code(
                    2,
                    "Mouse",
                    "Logitech M650",
                    "10",
                )],
            ),
        )
        .expect("create quantity asset import batch without asset code");

        assert_eq!(batch.summary.total_rows, 1);
        assert_eq!(batch.summary.valid_rows, 1);
        assert_eq!(batch.summary.error_rows, 0);
        assert_eq!(batch.rows[0].status, "valid");
        assert_eq!(batch.rows[0].asset_code, None);
    }

    #[test]
    fn create_batch_persists_mode_specific_review_fields() {
        let mut conn = open_test_connection();

        let serialized_batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![row_without_asset_code(2, "Laptop", "Dell Latitude 7440")],
            ),
        )
        .expect("create serialized batch for review fields");
        assert_eq!(serialized_batch.rows[0].brand.as_deref(), Some("Dell"));
        assert_eq!(serialized_batch.rows[0].warehouse.as_deref(), Some("HCM"));
        assert_eq!(serialized_batch.rows[0].quantity, None);

        let quantity_batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Quantity,
                vec![quantity_row_without_asset_code(
                    3,
                    "Mouse",
                    "Logitech M650",
                    "10",
                )],
            ),
        )
        .expect("create quantity batch for review fields");
        assert_eq!(quantity_batch.rows[0].brand.as_deref(), Some("Logitech"));
        assert_eq!(quantity_batch.rows[0].warehouse.as_deref(), Some("HCM"));
        assert_eq!(quantity_batch.rows[0].quantity.as_deref(), Some("10"));
    }

    #[test]
    fn quantity_rows_revalidate_after_inline_quantity_fix() {
        let mut conn = open_test_connection();

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Quantity,
                vec![quantity_row_without_asset_code(
                    2,
                    "Mouse",
                    "Logitech M650",
                    "0",
                )],
            ),
        )
        .expect("create quantity batch with invalid quantity");

        assert_eq!(batch.summary.valid_rows, 0);
        assert_eq!(batch.summary.error_rows, 1);
        assert!(batch.rows[0]
            .validation_errors
            .iter()
            .any(|item| item == "quantity must be a positive integer"));

        let updated = update_asset_import_row_conn(
            &mut conn,
            AssetImportRowUpdateInput {
                row_id: batch.rows[0].id,
                field_key: "quantity".to_string(),
                value: Some("5".to_string()),
            },
        )
        .expect("fix quantity inline");

        assert_eq!(updated.quantity.as_deref(), Some("5"));
        assert_eq!(updated.status, "valid");
        assert!(updated.validation_errors.is_empty());
    }

    #[test]
    fn persisted_batch_detail_can_be_reloaded_from_sqlite_after_reopen() {
        let db_path = temp_db_path("asset-import-batch-reload");

        let batch_id = {
            let mut conn = Connection::open(&db_path).expect("open sqlite file");
            configure_connection(&conn).expect("configure sqlite pragmas");
            apply_migrations(&conn).expect("apply migrations");

            let batch = create_asset_import_batch_seed_conn(
                &mut conn,
                sample_batch(
                    AssetImportMode::Quantity,
                    vec![row(2, "ASSET-001", "Laptop", "Dell Latitude 7440")],
                ),
            )
            .expect("create asset import batch");

            batch.summary.id
        };

        let conn = Connection::open(&db_path).expect("reopen sqlite file");
        configure_connection(&conn).expect("configure sqlite pragmas");
        apply_migrations(&conn).expect("apply migrations");
        let reloaded = load_asset_import_batch_detail_conn(&conn, batch_id)
            .expect("reload asset import batch");

        assert_eq!(reloaded.summary.id, batch_id);
        assert_eq!(reloaded.summary.import_type, AssetImportMode::Quantity);
        assert_eq!(reloaded.summary.total_rows, 1);
        assert_eq!(reloaded.rows.len(), 1);
        assert_eq!(reloaded.rows[0].asset_code.as_deref(), Some("ASSET-001"));
        assert_eq!(reloaded.rows[0].brand.as_deref(), Some("Dell"));
        assert_eq!(reloaded.rows[0].warehouse.as_deref(), Some("HCM"));

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn import_valid_rows_commits_quantity_batches_into_stock_items() {
        let mut conn = open_test_connection();

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Quantity,
                vec![quantity_row_without_asset_code(
                    2,
                    "Mouse",
                    "Logitech M650",
                    "10",
                )],
            ),
        )
        .expect("create quantity batch");

        let result = import_asset_import_batch_valid_rows_conn(&mut conn, batch.summary.id)
            .expect("import valid quantity rows into stock_items");

        assert_eq!(result.imported_count, 1);
        assert_eq!(result.imported_row_ids.len(), 1);

        let stock_item = conn
            .query_row(
                r#"
                SELECT si.item_name, si.quantity_on_hand, si.assigned_quantity, si.warehouse, si.note
                FROM stock_items si
                INNER JOIN asset_categories c ON c.id = si.category_id
                WHERE c.category_code = 'mouse'
                "#,
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .expect("load imported stock item");

        assert_eq!(stock_item.0, "Logitech M650");
        assert_eq!(stock_item.1, 10);
        assert_eq!(stock_item.2, 0);
        assert_eq!(stock_item.3.as_deref(), Some("HCM"));
        assert_eq!(stock_item.4.as_deref(), Some("Initial import"));

        let batch_after = load_asset_import_batch_detail_conn(&conn, batch.summary.id)
            .expect("reload quantity batch after import");
        assert_eq!(batch_after.summary.imported_rows, 1);
        assert_eq!(batch_after.summary.error_rows, 0);
        assert_eq!(batch_after.summary.status, "completed");
        assert_eq!(batch_after.rows[0].status, "imported");
    }

    #[test]
    fn preview_asset_import_seed_reports_valid_and_error_rows_without_persisting_batch() {
        let mut conn = open_test_connection();

        let preview = preview_asset_import_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Quantity,
                vec![
                    quantity_row_without_asset_code(2, "Mouse", "Logitech M650", "10"),
                    quantity_row_without_asset_code(3, "Mouse", "Broken Mouse", "0"),
                ],
            ),
        )
        .expect("preview quantity import");

        assert_eq!(preview.total_rows, 2);
        assert_eq!(preview.valid_rows, 1);
        assert_eq!(preview.error_rows, 1);
        assert_eq!(preview.rows.len(), 2);
        assert_eq!(preview.errors.len(), 1);

        let batch_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM asset_import_batches", [], |row| {
                row.get(0)
            })
            .expect("count temporary batches after preview");
        assert_eq!(batch_count, 0);
    }

    #[test]
    fn preview_asset_import_seed_still_returns_preview_when_cleanup_fails() {
        let mut conn = open_test_connection();

        let preview = preview_asset_import_seed_conn_with_cleanup(
            &mut conn,
            sample_batch(
                AssetImportMode::Quantity,
                vec![quantity_row_without_asset_code(
                    2,
                    "Mouse",
                    "Logitech M650",
                    "10",
                )],
            ),
            |_, _, _| Err("cleanup failed".to_string()),
        )
        .expect("preview should succeed even if cleanup fails");

        assert_eq!(preview.total_rows, 1);
        assert_eq!(preview.valid_rows, 1);
        assert_eq!(preview.error_rows, 0);
    }

    #[test]
    fn import_asset_import_seed_imports_only_valid_rows_and_cleans_up_temporary_batch() {
        let mut conn = open_test_connection();

        let report = import_asset_import_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Quantity,
                vec![
                    quantity_row_without_asset_code(2, "Mouse", "Logitech M650", "10"),
                    quantity_row_without_asset_code(3, "Mouse", "Broken Mouse", "0"),
                ],
            ),
        )
        .expect("import quantity rows directly");

        assert_eq!(report.total_rows, 2);
        assert_eq!(report.imported, 1);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.failed, 0);
        assert_eq!(report.errors.len(), 1);

        let stock_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM stock_items", [], |row| row.get(0))
            .expect("count stock rows after direct import");
        assert_eq!(stock_count, 1);

        let batch_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM asset_import_batches", [], |row| {
                row.get(0)
            })
            .expect("count temporary batches after direct import");
        assert_eq!(batch_count, 0);
    }

    #[test]
    fn import_asset_import_seed_still_returns_report_when_cleanup_fails() {
        let mut conn = open_test_connection();

        let report = import_asset_import_seed_conn_with_cleanup(
            &mut conn,
            sample_batch(
                AssetImportMode::Quantity,
                vec![quantity_row_without_asset_code(
                    2,
                    "Mouse",
                    "Logitech M650",
                    "10",
                )],
            ),
            |_, _, _| Err("cleanup failed".to_string()),
        )
        .expect("import should succeed even if cleanup fails");

        assert_eq!(report.total_rows, 1);
        assert_eq!(report.imported, 1);
        assert_eq!(report.skipped, 0);

        let stock_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM stock_items", [], |row| row.get(0))
            .expect("count stock rows after import");
        assert_eq!(stock_count, 1);
    }

    #[test]
    fn import_valid_rows_does_not_import_serialized_batches_without_asset_codes() {
        let mut conn = open_test_connection();

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![row_without_asset_code(2, "Laptop", "Dell Latitude 7440")],
            ),
        )
        .expect("create serialized batch without asset code");

        let result = import_asset_import_batch_valid_rows_conn(&mut conn, batch.summary.id)
            .expect("invalid rows remain pending review without import");

        assert_eq!(result.imported_count, 0);
        assert_eq!(result.remaining_error_rows, 1);
    }

    #[test]
    fn import_valid_rows_only_creates_assets_for_rows_still_marked_valid() {
        let mut conn = open_test_connection();
        seed_asset(&conn, "ASSET-EXISTING");

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![
                    row(2, "ASSET-001", "Laptop", "Dell Latitude 7440"),
                    row(3, "ASSET-EXISTING", "Laptop", "Dell Latitude Existing"),
                ],
            ),
        )
        .expect("create asset import batch");

        let result = import_asset_import_batch_valid_rows_conn(&mut conn, batch.summary.id)
            .expect("import only valid rows");

        assert_eq!(result.imported_row_ids.len(), 1);

        let imported_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM assets WHERE asset_code IN ('ASSET-001', 'ASSET-EXISTING')",
                [],
                |row| row.get(0),
            )
            .expect("count imported assets");
        assert_eq!(
            imported_count, 2,
            "one existing asset plus one newly imported asset should exist"
        );

        let batch_after = load_asset_import_batch_detail_conn(&conn, batch.summary.id)
            .expect("reload asset import batch");
        assert_eq!(batch_after.summary.imported_rows, 1);
        assert_eq!(batch_after.summary.skipped_rows, 1);
        assert_eq!(batch_after.summary.error_rows, 0);
        assert_eq!(
            batch_after
                .rows
                .iter()
                .filter(|item| item.status == "imported")
                .count(),
            1
        );
    }

    #[test]
    fn create_batch_resolves_laptop_owner_by_staff_id_suffix_and_keeps_team_mismatch_as_warning() {
        let mut conn = open_test_connection();
        seed_employee(
            &conn,
            "ASWVN729",
            "Trần Tú Linh",
            "Consolidated Operations Group Limited",
            "internal_movement",
        );

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![owner_row(2, "ASW729", "TRẦN TÚ LINH", "COG", "VNLAP512")],
            ),
        )
        .expect("create owner-aware laptop batch");

        assert_eq!(batch.summary.valid_rows, 1);
        assert_eq!(batch.summary.error_rows, 0);
        assert_eq!(batch.rows[0].status, "valid");
        assert_eq!(
            batch.rows[0].resolved_employee_id.as_deref(),
            Some("ASWVN729")
        );
        assert_eq!(
            batch.rows[0].resolved_full_name.as_deref(),
            Some("Trần Tú Linh")
        );
        assert_eq!(batch.rows[0].owner_match_status.as_str(), "warning");
        assert!(
            !batch.rows[0].owner_warnings.is_empty(),
            "team mismatch should surface as a warning, not a blocking error"
        );
    }

    #[test]
    fn create_batch_marks_laptop_rows_without_resolved_employee_as_errors() {
        let mut conn = open_test_connection();

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![owner_row(
                    2,
                    "ASWVN9999",
                    "Unknown User",
                    "Unknown Team",
                    "VNLAP999",
                )],
            ),
        )
        .expect("create unresolved owner batch");

        assert_eq!(batch.summary.valid_rows, 0);
        assert_eq!(batch.summary.error_rows, 1);
        assert_eq!(batch.rows[0].status, "error");
        assert!(batch.rows[0].resolved_employee_id.is_none());
        assert!(
            batch.rows[0]
                .validation_errors
                .iter()
                .any(|item| item.contains("employee")),
            "unresolved owner rows should be blocked from import"
        );
    }

    #[test]
    fn owner_resolution_uses_full_staff_id_and_rejects_unrelated_prefixes() {
        let mut conn = open_test_connection();
        seed_employee(
            &conn,
            "ASWVN001",
            "Leading Zero Employee",
            "IT",
            "employee_list",
        );

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![owner_row(
                    2,
                    "ASWCVN001",
                    "Leading Zero Employee",
                    "IT",
                    "VNLAP001",
                )],
            ),
        )
        .expect("create unrelated-prefix owner batch");

        assert_eq!(batch.summary.valid_rows, 0);
        assert_eq!(batch.summary.error_rows, 1);
        assert!(batch.rows[0].resolved_employee_id.is_none());
        assert!(batch.rows[0]
            .validation_errors
            .iter()
            .any(|item| item.contains("employee")));

        let exact_batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![owner_row(
                    2,
                    "ASWVN001",
                    "Leading Zero Employee",
                    "IT",
                    "VNLAP002",
                )],
            ),
        )
        .expect("create exact owner batch");

        assert_eq!(exact_batch.summary.valid_rows, 1);
        assert_eq!(
            exact_batch.rows[0].resolved_employee_id.as_deref(),
            Some("ASWVN001")
        );
    }

    #[test]
    fn owner_rows_revalidate_after_inline_staff_id_fix() {
        let mut conn = open_test_connection();
        seed_employee(
            &conn,
            "ASWVN1302",
            "LÆ° Tháº¿ HÃ¹ng",
            "Examworks",
            "employee_list",
        );

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![owner_row(
                    2,
                    "ASWVN9999",
                    "LÆ° Tháº¿ HÃ¹ng",
                    "Examworks",
                    "VNLAP777",
                )],
            ),
        )
        .expect("create unresolved owner batch");

        assert_eq!(batch.rows[0].status, "error");

        let updated = update_asset_import_row_conn(
            &mut conn,
            AssetImportRowUpdateInput {
                row_id: batch.rows[0].id,
                field_key: "submittedStaffId".to_string(),
                value: Some("1302".to_string()),
            },
        )
        .expect("fix submitted staff id inline");

        assert_eq!(updated.status, "valid");
        assert_eq!(updated.resolved_employee_id.as_deref(), Some("ASWVN1302"));
        assert!(updated.validation_errors.is_empty());
    }

    #[test]
    fn import_valid_rows_creates_active_loan_for_resolved_laptop_owner_rows() {
        let mut conn = open_test_connection();
        let employee_row_id =
            seed_employee(&conn, "ASWVN1302", "Lư Thế Hùng", "Examworks", "onboarding");

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![owner_row(
                    2,
                    "ASW1302",
                    "Lư Thế Hùng",
                    "ExamWorks",
                    "VNLAP235",
                )],
            ),
        )
        .expect("create resolved owner batch");

        let result = import_asset_import_batch_valid_rows_conn(&mut conn, batch.summary.id)
            .expect("import resolved laptop owner row");

        assert_eq!(result.imported_count, 1);

        let asset_row = conn
            .query_row(
                "SELECT id, status FROM assets WHERE asset_code = 'VNLAP235'",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("load imported laptop asset");
        assert_eq!(asset_row.1, "assigned");

        let active_loan = conn
            .query_row(
                "SELECT employee_id_fk FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
                params![asset_row.0],
                |row| row.get::<_, i64>(0),
            )
            .expect("load active asset loan");
        assert_eq!(active_loan, employee_row_id);
    }

    #[test]
    fn import_valid_rows_creates_active_loan_for_resolved_monitor_owner_rows_and_persists_dashboard_metadata(
    ) {
        let mut conn = open_test_connection();
        let employee_row_id = seed_employee(
            &conn,
            "ASWVN1134",
            "Tran Gia Thanh",
            "Examworks",
            "internal_movement",
        );

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![monitor_owner_row(
                    2,
                    "ASWVN1134",
                    "Tran Gia Thanh",
                    "VNMON709",
                    Some("office"),
                    "Mon709",
                )],
            ),
        )
        .expect("create resolved monitor owner batch");

        let result = import_asset_import_batch_valid_rows_conn(&mut conn, batch.summary.id)
            .expect("import resolved monitor owner row");

        assert_eq!(result.imported_count, 1);

        let asset_row = conn
            .query_row(
                r#"
                SELECT a.status, a.display_name_short, a.usage_location, c.category_code, a.id
                FROM assets a
                LEFT JOIN asset_categories c ON c.id = a.category_id
                WHERE a.asset_code = 'VNMON709'
                "#,
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )
            .expect("load imported monitor asset");
        assert_eq!(asset_row.0, "assigned");
        assert_eq!(asset_row.1.as_deref(), Some("Mon709"));
        assert_eq!(asset_row.2.as_deref(), Some("office"));
        assert_eq!(asset_row.3.as_deref(), Some("monitor"));

        let active_loan = conn
            .query_row(
                "SELECT employee_id_fk FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
                params![asset_row.4],
                |row| row.get::<_, i64>(0),
            )
            .expect("load active monitor loan");
        assert_eq!(active_loan, employee_row_id);
    }

    #[test]
    fn import_valid_rows_keeps_available_rows_in_stock_when_owner_data_is_absent() {
        let mut conn = open_test_connection();

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(
                AssetImportMode::Serialized,
                vec![AssetImportRowSeedInput {
                    row_number: 2,
                    raw_values: vec![
                        AssetImportRawValue {
                            header: "Assetcode".to_string(),
                            value: "VNLAP600".to_string(),
                        },
                        AssetImportRawValue {
                            header: "Category".to_string(),
                            value: "Laptop".to_string(),
                        },
                        AssetImportRawValue {
                            header: "Asset Name".to_string(),
                            value: "ASWVNLAP600".to_string(),
                        },
                    ],
                    asset_code: Some("VNLAP600".to_string()),
                    asset_type: Some("Laptop".to_string()),
                    display_name: Some("ASWVNLAP600".to_string()),
                    display_name_short: None,
                    computer_name: Some("ASWVNLAP600".to_string()),
                    brand: Some("Dell".to_string()),
                    model: Some("Latitude 5440".to_string()),
                    serial_number: None,
                    adapter_number: None,
                    quantity: None,
                    warehouse: Some("HCM".to_string()),
                    usage_location: None,
                    notes: Some("Warehouse import".to_string()),
                    submitted_staff_id: None,
                    submitted_full_name: None,
                    submitted_team: None,
                    submitted_phone_number: None,
                }],
            ),
        )
        .expect("create available laptop batch");

        let result = import_asset_import_batch_valid_rows_conn(&mut conn, batch.summary.id)
            .expect("import available row");

        assert_eq!(result.imported_count, 1);

        let asset_row = conn
            .query_row(
                "SELECT status, serial_number FROM assets WHERE asset_code = 'VNLAP600'",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .expect("load available imported asset");
        assert_eq!(asset_row.0, "in_stock");
        assert_eq!(asset_row.1, None);

        let active_loan_count = conn
            .query_row(
                "SELECT COUNT(*) FROM asset_loans WHERE asset_id = (SELECT id FROM assets WHERE asset_code = 'VNLAP600') AND returned_at IS NULL",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count active loans");
        assert_eq!(active_loan_count, 0);
    }
}
