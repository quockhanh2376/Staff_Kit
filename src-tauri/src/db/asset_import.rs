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

const ASSET_CODE_ALIASES: &[&str] = &[
    "assetcode",
    "code",
    "assetid",
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
    "assetcategory",
];
const DISPLAY_NAME_ALIASES: &[&str] = &[
    "displayname",
    "name",
    "assetname",
    "tentaisan",
    "description",
];
const MODEL_ALIASES: &[&str] = &["model", "modelnumber", "modelno"];
const SERIAL_NUMBER_ALIASES: &[&str] = &["serialnumber", "serial", "serialno", "serialnum", "sn"];
const NOTES_ALIASES: &[&str] = &["notes", "note", "remark", "remarks", "ghichu"];

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
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub notes: Option<String>,
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
    pub file_path: String,
    pub sheet_name: Option<String>,
    pub mapping: Option<AssetImportFieldMapping>,
}

#[derive(Debug, Clone)]
pub struct AssetImportBatchSeedInput {
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
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportBatchSummary {
    pub id: i64,
    pub batch_key: String,
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
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub notes: Option<String>,
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

#[derive(Debug, Clone)]
struct AssetImportResolvedMapping {
    mapping: AssetImportFieldMapping,
    asset_code_index: usize,
    asset_type_index: usize,
    display_name_index: usize,
    model_index: Option<usize>,
    serial_number_index: Option<usize>,
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
    asset_code: Option<String>,
    asset_type: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, Clone)]
struct AssetImportBatchRowMeta {
    batch_id: i64,
    status: String,
    edited_fields: Vec<String>,
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
        requires_manual_mapping: mapping_missing_required_fields(&inspection.auto_mapping)
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
        payload.sheet_name.as_deref(),
        payload.mapping,
    )?;

    let mut conn = open_runtime_connection(app)?;
    create_asset_import_batch_seed_conn(
        &mut conn,
        AssetImportBatchSeedInput {
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
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start asset import batch transaction: {err}"))?;

    let batch_key = generate_batch_key_tx(&tx)?;
    tx.execute(
        r#"
        INSERT INTO asset_import_batches(
          batch_key,
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
        VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        "#,
        params![
            batch_key.as_str(),
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
              model,
              serial_number,
              notes,
              validation_errors_json,
              status,
              edited,
              edited_fields_json,
              created_at,
              updated_at
            )
            VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, '[]', ?, 0, '[]', datetime('now'), datetime('now'))
            "#,
            params![
                batch_id,
                row.row_number,
                to_json(&row.raw_values)?,
                normalize_asset_code(row.asset_code),
                normalize_optional_asset_text(row.asset_type),
                normalize_optional_asset_text(row.display_name),
                normalize_optional_asset_text(row.model),
                normalize_optional_asset_text(row.serial_number),
                normalize_optional_asset_text(row.notes),
                ROW_STATUS_VALID,
            ],
        )
        .map_err(humanize_sqlite_error)?;
    }

    revalidate_batch_tx(&tx, batch_id)?;

    let payload_json = json!({
        "batchKey": batch_key,
        "sourceFileName": input.source_file_name,
        "sourceFileType": input.source_file_type,
        "sheetName": input.sheet_name,
        "totalRows": input.headers.len(),
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
                    row.get::<_, String>(15)?,
                    row.get::<_, String>(16)?,
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
        _ => normalize_optional_asset_text(payload.value),
    };

    let column_name = match field_key.as_str() {
        "assetCode" => "asset_code",
        "assetType" => "asset_type",
        "displayName" => "display_name",
        "model" => "model",
        "serialNumber" => "serial_number",
        "notes" => "notes",
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

    revalidate_batch_tx(&tx, batch_id)?;

    let mut stmt = tx
        .prepare(
            r#"
            SELECT
              id,
              asset_code,
              asset_type,
              display_name,
              model,
              serial_number,
              notes
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
            ))
        })
        .map_err(|err| format!("failed to query valid asset import rows: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read valid asset import rows: {err}"))?;

    drop(stmt);

    let mut imported_row_ids = Vec::new();
    let mut imported_asset_codes = Vec::new();

    for (row_id, asset_code, asset_type, display_name, model, serial_number, notes) in rows {
        let record = asset::create_asset_tx(
            &tx,
            &AssetUpsertInput {
                asset_code: asset_code.unwrap_or_default(),
                asset_type: asset_type.unwrap_or_default(),
                display_name: display_name.unwrap_or_default(),
                model,
                serial_number,
                notes,
            },
        )?;

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

    audit::insert_audit_log_tx(
        &tx,
        "asset_import.delete_batch",
        "local_account",
        actor_ref.as_deref(),
        "asset_import_batch",
        batch_id.to_string().as_str(),
        None,
    )?;

    tx.commit()
        .map_err(|err| format!("failed to commit asset import batch delete: {err}"))?;

    Ok(true)
}

fn active_actor_ref(conn: &Connection) -> Result<Option<String>, String> {
    Ok(auth::get_active_local_account_id(conn)?.map(|id| id.to_string()))
}

fn parse_asset_import_source(
    path: &Path,
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
        "csv" => parse_csv_source(path, source_file_name, mapping_override),
        "xlsx" | "xls" | "xlsm" => parse_excel_source(
            path,
            source_file_name,
            source_file_type,
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

    let resolved_mapping = resolve_mapping(&headers, mapping_override.clone())?;
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

    let resolved_mapping = resolve_mapping(&headers, mapping_override.clone())?;
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
    AssetImportRowSeedInput {
        row_number,
        asset_code: mapped_value(&raw_values, mapping.asset_code_index),
        asset_type: mapped_value(&raw_values, mapping.asset_type_index),
        display_name: mapped_value(&raw_values, mapping.display_name_index),
        model: mapping
            .model_index
            .and_then(|index| mapped_value(&raw_values, index)),
        serial_number: mapping
            .serial_number_index
            .and_then(|index| mapped_value(&raw_values, index)),
        notes: mapping
            .notes_index
            .and_then(|index| mapped_value(&raw_values, index)),
        raw_values,
    }
}

fn mapped_value(raw_values: &[AssetImportRawValue], index: usize) -> Option<String> {
    raw_values
        .get(index)
        .map(|item| item.value.clone())
        .and_then(|value| normalize_optional_asset_text(Some(value)))
}

fn resolve_mapping(
    headers: &[String],
    mapping_override: Option<AssetImportFieldMapping>,
) -> Result<AssetImportResolvedMapping, String> {
    let detected = detect_field_mapping(headers);
    let merged = if let Some(override_mapping) = mapping_override {
        merge_field_mapping(detected, override_mapping)
    } else {
        detected
    };

    let Some(missing_required) = mapping_missing_required_fields(&merged) else {
        let header_indices = headers
            .iter()
            .enumerate()
            .map(|(index, header)| (normalize_header_key(header), index))
            .collect::<HashMap<_, _>>();

        return Ok(AssetImportResolvedMapping {
            asset_code_index: resolve_header_index(&header_indices, merged.asset_code.as_deref())?,
            asset_type_index: resolve_header_index(&header_indices, merged.asset_type.as_deref())?,
            display_name_index: resolve_header_index(
                &header_indices,
                merged.display_name.as_deref(),
            )?,
            model_index: resolve_optional_header_index(&header_indices, merged.model.as_deref()),
            serial_number_index: resolve_optional_header_index(
                &header_indices,
                merged.serial_number.as_deref(),
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
        if mapping.display_name.is_none() && DISPLAY_NAME_ALIASES.contains(&normalized.as_str()) {
            mapping.display_name = Some(header.clone());
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
        if mapping.notes.is_none() && NOTES_ALIASES.contains(&normalized.as_str()) {
            mapping.notes = Some(header.clone());
        }
    }

    mapping
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
        model: normalized_mapping_choice(override_mapping.model).or(detected.model),
        serial_number: normalized_mapping_choice(override_mapping.serial_number)
            .or(detected.serial_number),
        notes: normalized_mapping_choice(override_mapping.notes).or(detected.notes),
    }
}

fn normalized_mapping_choice(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn mapping_missing_required_fields(mapping: &AssetImportFieldMapping) -> Option<Vec<&'static str>> {
    let mut missing = Vec::new();
    if mapping.asset_code.is_none() {
        missing.push("assetCode");
    }
    if mapping.asset_type.is_none() {
        missing.push("assetType");
    }
    if mapping.display_name.is_none() {
        missing.push("displayName");
    }
    if missing.is_empty() {
        None
    } else {
        Some(missing)
    }
}

fn mapping_score(mapping: &AssetImportFieldMapping) -> i64 {
    let required_matches = [
        mapping.asset_code.as_ref(),
        mapping.asset_type.as_ref(),
        mapping.display_name.as_ref(),
    ]
    .into_iter()
    .flatten()
    .count() as i64;
    let optional_matches = [
        mapping.model.as_ref(),
        mapping.serial_number.as_ref(),
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

fn normalize_optional_asset_text(value: Option<String>) -> Option<String> {
    normalize_optional_text(value)
}

fn normalize_import_field_key(value: &str) -> Result<String, String> {
    match value.trim() {
        "assetCode" | "assetType" | "displayName" | "model" | "serialNumber" | "notes" => {
            Ok(value.trim().to_string())
        }
        _ => Err(format!("unsupported asset import field '{value}'")),
    }
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

fn revalidate_batch_tx(tx: &Transaction<'_>, batch_id: i64) -> Result<(), String> {
    let rows = load_batch_row_states_tx(tx, batch_id)?;
    let existing_asset_codes = load_existing_asset_codes_tx(tx)?;

    let mut duplicate_counts: HashMap<String, usize> = HashMap::new();
    for row in rows
        .iter()
        .filter(|item| item.status != ROW_STATUS_IMPORTED && item.status != ROW_STATUS_SKIPPED)
    {
        if let Some(asset_code) = row.asset_code.as_deref() {
            *duplicate_counts.entry(asset_code.to_string()).or_default() += 1;
        }
    }

    let duplicate_asset_codes = duplicate_counts
        .into_iter()
        .filter_map(|(asset_code, count)| if count > 1 { Some(asset_code) } else { None })
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

        let validation_errors =
            validate_staged_row(row, &duplicate_asset_codes, &existing_asset_codes);
        let next_status = if row.status == ROW_STATUS_SKIPPED {
            ROW_STATUS_SKIPPED
        } else if validation_errors.is_empty() {
            ROW_STATUS_VALID
        } else {
            ROW_STATUS_ERROR
        };

        tx.execute(
            r#"
            UPDATE asset_import_rows
            SET
              validation_errors_json = ?,
              status = ?,
              updated_at = datetime('now')
            WHERE id = ?
            "#,
            params![to_json(&validation_errors)?, next_status, row.id],
        )
        .map_err(humanize_sqlite_error)?;
    }

    refresh_batch_summary_tx(tx, batch_id)
}

fn validate_staged_row(
    row: &AssetImportRowState,
    duplicate_asset_codes: &HashSet<String>,
    existing_asset_codes: &HashSet<String>,
) -> Vec<String> {
    let mut errors = Vec::new();
    if row.asset_code.is_none() {
        errors.push("assetCode is required".to_string());
    }
    if row.asset_type.is_none() {
        errors.push("assetType is required".to_string());
    }
    if row.display_name.is_none() {
        errors.push("displayName is required".to_string());
    }
    if let Some(asset_code) = row.asset_code.as_deref() {
        if duplicate_asset_codes.contains(asset_code) {
            errors.push("assetCode is duplicated in this batch".to_string());
        }
        if existing_asset_codes.contains(asset_code) {
            errors.push("assetCode already exists in assets".to_string());
        }
    }
    errors
}

fn load_existing_asset_codes_tx(tx: &Transaction<'_>) -> Result<HashSet<String>, String> {
    let mut stmt = tx
        .prepare("SELECT asset_code FROM assets")
        .map_err(|err| format!("failed to prepare asset code lookup query: {err}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|err| format!("failed to query existing asset codes: {err}"))?;

    let mut items = HashSet::new();
    for row in rows {
        items.insert(row.map_err(|err| format!("failed to read existing asset code row: {err}"))?);
    }
    Ok(items)
}

fn load_batch_row_states_tx(
    tx: &Transaction<'_>,
    batch_id: i64,
) -> Result<Vec<AssetImportRowState>, String> {
    let mut stmt = tx
        .prepare(
            r#"
            SELECT id, status, asset_code, asset_type, display_name
            FROM asset_import_rows
            WHERE batch_id = ?
            ORDER BY row_number ASC, id ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare staged asset row query: {err}"))?;
    let rows = stmt
        .query_map(params![batch_id], |row| {
            Ok(AssetImportRowState {
                id: row.get(0)?,
                status: row.get(1)?,
                asset_code: row.get(2)?,
                asset_type: row.get(3)?,
                display_name: row.get(4)?,
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
        source_file_name: row.get(2)?,
        source_file_path: row.get(3)?,
        source_file_type: row.get(4)?,
        sheet_name: row.get(5)?,
        header_row: row.get(6)?,
        status: row.get(7)?,
        total_rows: row.get(8)?,
        valid_rows: row.get(9)?,
        error_rows: row.get(10)?,
        imported_rows: row.get(11)?,
        skipped_rows: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
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
              id, batch_id, row_number, raw_row_json, asset_code, asset_type, display_name, model,
              serial_number, notes, validation_errors_json, status, edited, edited_fields_json, imported_asset_id
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
          id, batch_id, row_number, raw_row_json, asset_code, asset_type, display_name, model,
          serial_number, notes, validation_errors_json, status, edited, edited_fields_json, imported_asset_id
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
        model: row.get(7)?,
        serial_number: row.get(8)?,
        notes: row.get(9)?,
        validation_errors: from_json_row(Some(row.get(10)?))?,
        status: row.get(11)?,
        is_edited: row.get::<_, i64>(12)? > 0,
        edited_fields: from_json_row(Some(row.get(13)?))?,
        imported_asset_id: row.get(14)?,
    })
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
        load_asset_import_batch_detail_conn, AssetImportBatchSeedInput, AssetImportFieldMapping,
        AssetImportRawValue, AssetImportRowSeedInput,
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

    fn seed_asset(conn: &Connection, asset_code: &str) {
        conn.execute(
            r#"
            INSERT INTO assets(asset_code, asset_type, display_name, status, created_at, updated_at)
            VALUES(?, 'Laptop', ?, 'in_stock', datetime('now'), datetime('now'))
            "#,
            params![asset_code, asset_code],
        )
        .expect("insert asset");
    }

    fn sample_mapping() -> AssetImportFieldMapping {
        AssetImportFieldMapping {
            asset_code: Some("Asset Code".to_string()),
            asset_type: Some("Asset Type".to_string()),
            display_name: Some("Display Name".to_string()),
            model: Some("Model".to_string()),
            serial_number: Some("Serial Number".to_string()),
            notes: Some("Notes".to_string()),
        }
    }

    fn sample_headers() -> Vec<String> {
        vec![
            "Asset Code".to_string(),
            "Asset Type".to_string(),
            "Display Name".to_string(),
            "Model".to_string(),
            "Serial Number".to_string(),
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
                    header: "Model".to_string(),
                    value: "7440".to_string(),
                },
                AssetImportRawValue {
                    header: "Serial Number".to_string(),
                    value: format!("SN-{row_number:03}"),
                },
                AssetImportRawValue {
                    header: "Notes".to_string(),
                    value: "Initial import".to_string(),
                },
            ],
            asset_code: Some(asset_code.to_string()),
            asset_type: Some(asset_type.to_string()),
            display_name: Some(display_name.to_string()),
            model: Some("7440".to_string()),
            serial_number: Some(format!("SN-{row_number:03}")),
            notes: Some("Initial import".to_string()),
        }
    }

    fn sample_batch(rows: Vec<AssetImportRowSeedInput>) -> AssetImportBatchSeedInput {
        AssetImportBatchSeedInput {
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

    #[test]
    fn create_batch_marks_duplicate_asset_codes_inside_same_batch_as_errors() {
        let mut conn = open_test_connection();

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(vec![
                row(2, "asset-001", "Laptop", "Dell Latitude 7440"),
                row(3, "ASSET-001", "Laptop", "Dell Latitude 7450"),
            ]),
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
    fn create_batch_marks_duplicate_asset_codes_against_main_assets_table_as_errors() {
        let mut conn = open_test_connection();
        seed_asset(&conn, "ASSET-EXISTING");

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(vec![row(
                2,
                "asset-existing",
                "Laptop",
                "Dell Latitude 7440",
            )]),
        )
        .expect("create asset import batch");

        assert_eq!(batch.summary.total_rows, 1);
        assert_eq!(batch.summary.valid_rows, 0);
        assert_eq!(batch.summary.error_rows, 1);
        assert_eq!(batch.rows[0].status, "error");
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
                sample_batch(vec![row(2, "ASSET-001", "Laptop", "Dell Latitude 7440")]),
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
        assert_eq!(reloaded.summary.total_rows, 1);
        assert_eq!(reloaded.rows.len(), 1);
        assert_eq!(reloaded.rows[0].asset_code.as_deref(), Some("ASSET-001"));

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn import_valid_rows_only_creates_assets_for_rows_still_marked_valid() {
        let mut conn = open_test_connection();
        seed_asset(&conn, "ASSET-EXISTING");

        let batch = create_asset_import_batch_seed_conn(
            &mut conn,
            sample_batch(vec![
                row(2, "ASSET-001", "Laptop", "Dell Latitude 7440"),
                row(3, "ASSET-EXISTING", "Laptop", "Dell Latitude Existing"),
            ]),
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
        assert_eq!(batch_after.summary.error_rows, 1);
        assert_eq!(
            batch_after
                .rows
                .iter()
                .filter(|item| item.status == "imported")
                .count(),
            1
        );
    }
}
