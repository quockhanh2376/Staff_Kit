use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;

use super::{
    asset, audit, auth,
    employee::{self, EmployeeQuery, EmployeeRecord},
    normalize_optional_text, open_runtime_connection,
};

const DEFAULT_STAFF_GROUP: &str = "employee_list";
const ROW_STATUS_VALID: &str = "valid";
const ROW_STATUS_ERROR: &str = "error";
const SEED_SOURCE_LABEL: &str = "Stored Employee Computer Name";
const EMPLOYEE_ASSET_SEED_ENTITY_TYPE: &str = "employee_asset_seed";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeAssetSeedInput {
    pub snapshot_id: Option<i64>,
    pub query: Option<String>,
    pub team_name: Option<String>,
    pub staff_group: Option<String>,
    pub start_date_from: Option<String>,
    pub start_date_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeAssetSeedPreviewRow {
    pub employee_id: String,
    pub full_name: String,
    pub source_computer_name: String,
    pub asset_code: Option<String>,
    pub computer_name: Option<String>,
    pub category_code: Option<String>,
    pub category_name: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeAssetSeedErrorItem {
    pub row_number: i64,
    pub entity_key: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeAssetSeedPreview {
    pub snapshot_id: i64,
    pub source_label: String,
    pub matched_employee_count: i64,
    pub excluded_rows: i64,
    pub total_rows: i64,
    pub valid_rows: i64,
    pub error_rows: i64,
    pub rows: Vec<EmployeeAssetSeedPreviewRow>,
    pub errors: Vec<EmployeeAssetSeedErrorItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeAssetSeedReport {
    pub snapshot_id: i64,
    pub source_label: String,
    pub matched_employee_count: i64,
    pub excluded_rows: i64,
    pub total_rows: i64,
    pub imported: i64,
    pub skipped: i64,
    pub failed: i64,
    pub imported_asset_codes: Vec<String>,
    pub errors: Vec<EmployeeAssetSeedErrorItem>,
}

#[derive(Debug, Clone)]
struct CandidateRow {
    row_number: i64,
    employee_id: String,
    full_name: String,
    source_computer_name: String,
    asset_code: Option<String>,
    computer_name: Option<String>,
    category_code: Option<String>,
    category_name: Option<String>,
    status: String,
    reason: Option<String>,
}

#[derive(Debug, Clone)]
struct BuiltPreview {
    matched_employee_count: i64,
    rows: Vec<CandidateRow>,
    excluded_errors: Vec<EmployeeAssetSeedErrorItem>,
}

#[derive(Debug, Clone)]
struct LoadedPreviewSnapshot {
    snapshot_id: i64,
    actor_account_id: Option<i64>,
    filters: EmployeeAssetSeedInput,
    source_label: String,
    matched_employee_count: i64,
    excluded_rows: i64,
    total_rows: i64,
    error_rows: i64,
    errors: Vec<EmployeeAssetSeedErrorItem>,
    rows: Vec<CandidateRow>,
}

impl BuiltPreview {
    fn source_label(&self) -> String {
        SEED_SOURCE_LABEL.to_string()
    }

    fn excluded_rows(&self) -> i64 {
        self.excluded_errors.len() as i64
    }

    fn total_rows(&self) -> i64 {
        self.rows.len() as i64
    }

    fn valid_rows(&self) -> i64 {
        self.rows.iter().filter(|row| row.status == ROW_STATUS_VALID).count() as i64
    }

    fn error_rows(&self) -> i64 {
        self.rows.iter().filter(|row| row.status == ROW_STATUS_ERROR).count() as i64
    }

    fn errors(&self) -> Vec<EmployeeAssetSeedErrorItem> {
        let mut errors = self
            .rows
            .iter()
            .filter_map(|row| {
                row.reason.as_ref().map(|reason| EmployeeAssetSeedErrorItem {
                    row_number: row.row_number,
                    entity_key: Some(row.employee_id.clone()),
                    reason: reason.clone(),
                })
            })
            .collect::<Vec<_>>();
        errors.extend(self.excluded_errors.clone());
        errors.sort_by(|left, right| left.row_number.cmp(&right.row_number));
        errors
    }
}

pub fn preview_employee_asset_seed(
    app: &AppHandle,
    payload: EmployeeAssetSeedInput,
) -> Result<EmployeeAssetSeedPreview, String> {
    let mut conn = open_runtime_connection(app)?;
    preview_employee_asset_seed_conn(&mut conn, &payload)
}

pub fn import_employee_asset_seed(
    app: &AppHandle,
    payload: EmployeeAssetSeedInput,
) -> Result<EmployeeAssetSeedReport, String> {
    let mut conn = open_runtime_connection(app)?;
    import_employee_asset_seed_conn(&mut conn, &payload)
}

pub(crate) fn preview_employee_asset_seed_conn(
    conn: &mut Connection,
    payload: &EmployeeAssetSeedInput,
) -> Result<EmployeeAssetSeedPreview, String> {
    let actor_id = require_import_actor_id(conn)?;
    let built_preview = build_preview(conn, payload)?;
    let snapshot_id = persist_preview_snapshot(conn, payload, actor_id, &built_preview)?;
    Ok(build_preview_response(snapshot_id, &built_preview))
}

pub(crate) fn import_employee_asset_seed_conn(
    conn: &mut Connection,
    payload: &EmployeeAssetSeedInput,
) -> Result<EmployeeAssetSeedReport, String> {
    let actor_id = require_import_actor_id(conn)?;
    let snapshot_id = payload
        .snapshot_id
        .ok_or_else(|| "employee asset seed snapshotId is required".to_string())?;

    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start employee asset seed transaction: {err}"))?;
    let snapshot = load_preview_snapshot_tx(&tx, snapshot_id)?;

    if snapshot.actor_account_id != Some(actor_id) {
        return Err("employee asset seed snapshot belongs to a different active account".to_string());
    }

    let mut imported = 0_i64;
    let mut skipped = snapshot.error_rows + snapshot.excluded_rows;
    let mut failed = 0_i64;
    let mut imported_asset_codes = Vec::new();
    let mut errors = snapshot.errors.clone();

    for row in &snapshot.rows {
        if row.status != ROW_STATUS_VALID {
            continue;
        }

        let Some(asset_code) = row.asset_code.as_deref() else {
            skipped += 1;
            errors.push(EmployeeAssetSeedErrorItem {
                row_number: row.row_number,
                entity_key: Some(row.employee_id.clone()),
                reason: "asset code could not be derived".to_string(),
            });
            continue;
        };

        if row.computer_name.as_deref().is_none() {
            skipped += 1;
            errors.push(EmployeeAssetSeedErrorItem {
                row_number: row.row_number,
                entity_key: Some(row.employee_id.clone()),
                reason: "computer name could not be derived".to_string(),
            });
            continue;
        }

        let Some(category_id) = load_category_id_by_code_tx(&tx, row.category_code.as_deref())? else {
            skipped += 1;
            errors.push(EmployeeAssetSeedErrorItem {
                row_number: row.row_number,
                entity_key: Some(asset_code.to_string()),
                reason: "asset category could not be resolved".to_string(),
            });
            continue;
        };

        if asset_exists_for_seed_tx(&tx, asset_code)? {
            skipped += 1;
            errors.push(EmployeeAssetSeedErrorItem {
                row_number: row.row_number,
                entity_key: Some(asset_code.to_string()),
                reason: "asset already exists".to_string(),
            });
            continue;
        }

        let asset_type = row
            .category_name
            .clone()
            .unwrap_or_else(|| "Laptop".to_string());

        match asset::create_asset_tx(
            &tx,
            &asset::AssetUpsertInput {
                asset_code: asset_code.to_string(),
                category_id: Some(category_id),
                asset_type,
                display_name: strip_vn_prefix(asset_code),
                display_name_short: None,
                brand: None,
                model: None,
                serial_number: None,
                usage_location: None,
                adapter_number: None,
                warehouse: None,
                notes: Some(format!("Created from EE list employee {}", row.employee_id)),
            },
        ) {
            Ok(record) => {
                imported += 1;
                imported_asset_codes.push(record.asset_code);
            }
            Err(err) => {
                failed += 1;
                errors.push(EmployeeAssetSeedErrorItem {
                    row_number: row.row_number,
                    entity_key: Some(asset_code.to_string()),
                    reason: err,
                });
            }
        }
    }

    let payload_json = json!({
        "snapshotId": snapshot.snapshot_id,
        "filters": build_filters_json(&snapshot.filters),
        "sourceLabel": snapshot.source_label.as_str(),
        "matchedEmployeeCount": snapshot.matched_employee_count,
        "excludedRows": snapshot.excluded_rows,
        "totalRows": snapshot.total_rows,
        "imported": imported,
        "skipped": skipped,
        "failed": failed,
        "importedAssetCodes": imported_asset_codes.clone(),
    })
    .to_string();

    let actor_ref = actor_id.to_string();
    let entity_id = snapshot.snapshot_id.to_string();
    audit::insert_audit_log_tx(
        &tx,
        "employee_asset_seed.import",
        "local_account",
        Some(actor_ref.as_str()),
        EMPLOYEE_ASSET_SEED_ENTITY_TYPE,
        entity_id.as_str(),
        Some(payload_json.as_str()),
    )?;

    tx.execute(
        "UPDATE employee_asset_seed_snapshots SET approved_at = datetime('now') WHERE id = ?",
        params![snapshot.snapshot_id],
    )
    .map_err(|err| format!("failed to mark employee asset seed snapshot as approved: {err}"))?;

    tx.commit()
        .map_err(|err| format!("failed to commit employee asset seed transaction: {err}"))?;

    Ok(EmployeeAssetSeedReport {
        snapshot_id: snapshot.snapshot_id,
        source_label: snapshot.source_label,
        matched_employee_count: snapshot.matched_employee_count,
        excluded_rows: snapshot.excluded_rows,
        total_rows: snapshot.total_rows,
        imported,
        skipped,
        failed,
        imported_asset_codes,
        errors,
    })
}

fn build_preview(
    conn: &Connection,
    payload: &EmployeeAssetSeedInput,
) -> Result<BuiltPreview, String> {
    let query = EmployeeQuery {
        query: payload.query.clone(),
        team_name: payload.team_name.clone(),
        staff_group: Some(
            payload
                .staff_group
                .clone()
                .unwrap_or_else(|| DEFAULT_STAFF_GROUP.to_string()),
        ),
        sort_key: None,
        sort_direction: None,
        start_date_from: payload.start_date_from.clone(),
        start_date_to: payload.start_date_to.clone(),
        limit: None,
        offset: None,
    };

    let employees = employee::query_all_employees_for_filters(conn, query)?;
    let mut built = build_candidate_rows(conn, employees)?;
    built.rows.sort_by(|left, right| left.row_number.cmp(&right.row_number));
    Ok(built)
}

fn build_preview_response(snapshot_id: i64, preview: &BuiltPreview) -> EmployeeAssetSeedPreview {
    EmployeeAssetSeedPreview {
        snapshot_id,
        source_label: preview.source_label(),
        matched_employee_count: preview.matched_employee_count,
        excluded_rows: preview.excluded_rows(),
        total_rows: preview.total_rows(),
        valid_rows: preview.valid_rows(),
        error_rows: preview.error_rows(),
        rows: preview
            .rows
            .iter()
            .map(|row| EmployeeAssetSeedPreviewRow {
                employee_id: row.employee_id.clone(),
                full_name: row.full_name.clone(),
                source_computer_name: row.source_computer_name.clone(),
                asset_code: row.asset_code.clone(),
                computer_name: row.computer_name.clone(),
                category_code: row.category_code.clone(),
                category_name: row.category_name.clone(),
                status: row.status.clone(),
            })
            .collect(),
        errors: preview.errors(),
    }
}

fn build_candidate_rows(
    conn: &Connection,
    employees: Vec<EmployeeRecord>,
) -> Result<BuiltPreview, String> {
    let matched_employee_count = employees.len() as i64;

    let mut duplicate_counts = HashMap::<String, usize>::new();
    for employee in &employees {
        let Some(source) = normalize_seed_source(employee.stored_computer_name.as_deref()) else {
            continue;
        };

        if let Ok((asset_code, _)) = derive_asset_identity(source.as_str()) {
            *duplicate_counts.entry(asset_code).or_default() += 1;
        }
    }

    let mut rows = Vec::new();
    let mut excluded_errors = Vec::new();

    for (index, employee) in employees.into_iter().enumerate() {
        let row_number = i64::try_from(index + 1).unwrap_or(i64::MAX);
        let Some(source) = normalize_seed_source(employee.stored_computer_name.as_deref()) else {
            excluded_errors.push(EmployeeAssetSeedErrorItem {
                row_number,
                entity_key: Some(employee.employee_id.clone()),
                reason: "stored computer name is blank or invalid".to_string(),
            });
            continue;
        };

        let (asset_code, computer_name) = match derive_asset_identity(source.as_str()) {
            Ok(identity) => identity,
            Err(reason) => {
                excluded_errors.push(EmployeeAssetSeedErrorItem {
                    row_number,
                    entity_key: Some(employee.employee_id.clone()),
                    reason,
                });
                continue;
            }
        };

        if duplicate_counts
            .get(asset_code.as_str())
            .copied()
            .unwrap_or_default()
            > 1
        {
            rows.push(CandidateRow {
                row_number,
                employee_id: employee.employee_id,
                full_name: employee.full_name,
                source_computer_name: source,
                asset_code: Some(asset_code),
                computer_name: Some(computer_name),
                category_code: None,
                category_name: None,
                status: ROW_STATUS_ERROR.to_string(),
                reason: Some("duplicate computer name maps to the same asset across employees".to_string()),
            });
            continue;
        }

        let category = asset::load_asset_category_by_prefix_conn(conn, asset_code.as_str())?;
        let Some(category) = category else {
            rows.push(CandidateRow {
                row_number,
                employee_id: employee.employee_id,
                full_name: employee.full_name,
                source_computer_name: source,
                asset_code: Some(asset_code),
                computer_name: Some(computer_name),
                category_code: None,
                category_name: None,
                status: ROW_STATUS_ERROR.to_string(),
                reason: Some("asset category could not be resolved from prefix".to_string()),
            });
            continue;
        };

        if category.tracking_mode != "serialized" {
            rows.push(CandidateRow {
                row_number,
                employee_id: employee.employee_id,
                full_name: employee.full_name,
                source_computer_name: source,
                asset_code: Some(asset_code),
                computer_name: Some(computer_name),
                category_code: None,
                category_name: None,
                status: ROW_STATUS_ERROR.to_string(),
                reason: Some("resolved asset category is not serialized".to_string()),
            });
            continue;
        }

        let (category_code, category_name) = load_category_labels_by_id_conn(conn, category.id)?;
        if asset_exists_for_seed_conn(conn, asset_code.as_str())? {
            rows.push(CandidateRow {
                row_number,
                employee_id: employee.employee_id,
                full_name: employee.full_name,
                source_computer_name: source,
                asset_code: Some(asset_code),
                computer_name: Some(computer_name),
                category_code: Some(category_code),
                category_name: Some(category_name),
                status: ROW_STATUS_ERROR.to_string(),
                reason: Some("asset already exists".to_string()),
            });
            continue;
        }

        rows.push(CandidateRow {
            row_number,
            employee_id: employee.employee_id,
            full_name: employee.full_name,
            source_computer_name: source,
            asset_code: Some(asset_code),
            computer_name: Some(computer_name),
            category_code: Some(category_code),
            category_name: Some(category_name),
            status: ROW_STATUS_VALID.to_string(),
            reason: None,
        });
    }

    Ok(BuiltPreview {
        matched_employee_count,
        rows,
        excluded_errors,
    })
}

fn persist_preview_snapshot(
    conn: &mut Connection,
    payload: &EmployeeAssetSeedInput,
    actor_id: i64,
    preview: &BuiltPreview,
) -> Result<i64, String> {
    let filters_json = build_filters_json(payload).to_string();
    let errors_json = serde_json::to_string(&preview.errors())
        .map_err(|err| format!("failed to serialize employee asset seed preview errors: {err}"))?;
    let source_label = preview.source_label();

    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start employee asset seed preview transaction: {err}"))?;

    tx.execute(
        r#"
        INSERT INTO employee_asset_seed_snapshots(
          actor_account_id,
          filters_json,
          source_label,
          matched_employee_count,
          excluded_rows,
          total_rows,
          valid_rows,
          error_rows,
          errors_json,
          created_at
        )
        VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
        "#,
        params![
            actor_id,
            filters_json.as_str(),
            source_label.as_str(),
            preview.matched_employee_count,
            preview.excluded_rows(),
            preview.total_rows(),
            preview.valid_rows(),
            preview.error_rows(),
            errors_json.as_str(),
        ],
    )
    .map_err(|err| format!("failed to save employee asset seed snapshot: {err}"))?;

    let snapshot_id = tx.last_insert_rowid();
    for row in &preview.rows {
        tx.execute(
            r#"
            INSERT INTO employee_asset_seed_snapshot_rows(
              snapshot_id,
              row_number,
              employee_id,
              full_name,
              source_computer_name,
              asset_code,
              computer_name,
              category_code,
              category_name,
              status,
              created_at
            )
            VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
            params![
                snapshot_id,
                row.row_number,
                row.employee_id.as_str(),
                row.full_name.as_str(),
                row.source_computer_name.as_str(),
                row.asset_code.as_deref(),
                row.computer_name.as_deref(),
                row.category_code.as_deref(),
                row.category_name.as_deref(),
                row.status.as_str(),
            ],
        )
        .map_err(|err| format!("failed to save employee asset seed snapshot row: {err}"))?;
    }

    tx.commit()
        .map_err(|err| format!("failed to commit employee asset seed preview transaction: {err}"))?;

    Ok(snapshot_id)
}

fn load_preview_snapshot_tx(
    tx: &Transaction<'_>,
    snapshot_id: i64,
) -> Result<LoadedPreviewSnapshot, String> {
    let (
        loaded_snapshot_id,
        actor_account_id,
        filters_json,
        source_label,
        matched_employee_count,
        excluded_rows,
        total_rows,
        error_rows,
        errors_json,
    ) = tx
        .query_row(
            r#"
            SELECT
              id,
              actor_account_id,
              filters_json,
              source_label,
              matched_employee_count,
              excluded_rows,
              total_rows,
              error_rows,
              errors_json
            FROM employee_asset_seed_snapshots
            WHERE id = ?
            LIMIT 1
            "#,
            params![snapshot_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .optional()
        .map_err(|err| format!("failed to load employee asset seed snapshot {snapshot_id}: {err}"))?
        .ok_or_else(|| format!("employee asset seed snapshot {snapshot_id} was not found"))?;

    let filters = parse_filters_json(filters_json.as_deref())?;
    let errors = serde_json::from_str::<Vec<EmployeeAssetSeedErrorItem>>(errors_json.as_str())
        .map_err(|err| format!("failed to parse employee asset seed snapshot errors: {err}"))?;

    let mut stmt = tx
        .prepare(
            r#"
            SELECT
              row_number,
              employee_id,
              full_name,
              source_computer_name,
              asset_code,
              computer_name,
              category_code,
              category_name,
              status
            FROM employee_asset_seed_snapshot_rows
            WHERE snapshot_id = ?
            ORDER BY row_number ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare employee asset seed snapshot rows query: {err}"))?;

    let rows = stmt
        .query_map(params![snapshot_id], |row| {
            Ok(CandidateRow {
                row_number: row.get(0)?,
                employee_id: row.get(1)?,
                full_name: row.get(2)?,
                source_computer_name: row.get(3)?,
                asset_code: row.get(4)?,
                computer_name: row.get(5)?,
                category_code: row.get(6)?,
                category_name: row.get(7)?,
                status: row.get(8)?,
                reason: None,
            })
        })
        .map_err(|err| format!("failed to query employee asset seed snapshot rows: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read employee asset seed snapshot rows: {err}"))?;

    Ok(LoadedPreviewSnapshot {
        snapshot_id: loaded_snapshot_id,
        actor_account_id,
        filters,
        source_label,
        matched_employee_count,
        excluded_rows,
        total_rows,
        error_rows,
        errors,
        rows,
    })
}

fn build_filters_json(payload: &EmployeeAssetSeedInput) -> serde_json::Value {
    json!({
        "query": payload.query,
        "teamName": payload.team_name,
        "staffGroup": payload.staff_group,
        "startDateFrom": payload.start_date_from,
        "startDateTo": payload.start_date_to,
    })
}

fn parse_filters_json(raw: Option<&str>) -> Result<EmployeeAssetSeedInput, String> {
    let Some(raw) = raw else {
        return Ok(EmployeeAssetSeedInput::default());
    };

    serde_json::from_str::<EmployeeAssetSeedInput>(raw)
        .map_err(|err| format!("failed to parse employee asset seed filters: {err}"))
}

fn normalize_seed_source(value: Option<&str>) -> Option<String> {
    let normalized = normalize_optional_text(value.map(|item| item.to_string()))?;
    let compact = normalized
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    if compact.is_empty() {
        None
    } else {
        Some(compact)
    }
}

fn derive_asset_identity(source: &str) -> Result<(String, String), String> {
    let normalized = source.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        return Err("stored computer name cannot be blank".to_string());
    }

    if let Some(asset_code) = normalized.strip_prefix("ASW") {
        if asset_code.is_empty() {
            return Err("stored computer name could not derive asset code".to_string());
        }
        return Ok((asset_code.to_string(), normalized));
    }

    Ok((normalized.clone(), format!("ASW{normalized}")))
}

fn strip_vn_prefix(asset_code: &str) -> String {
    let normalized = asset_code.trim();
    if normalized.len() >= 2 && normalized[..2].eq_ignore_ascii_case("VN") {
        normalized[2..].to_string()
    } else {
        normalized.to_string()
    }
}

fn asset_exists_for_seed_conn(
    conn: &Connection,
    asset_code: &str,
) -> Result<bool, String> {
    conn.query_row(
        r#"
        SELECT 1
        FROM assets
        WHERE asset_code = ? COLLATE NOCASE
        LIMIT 1
        "#,
        params![asset_code],
        |_| Ok(true),
    )
    .optional()
    .map(|value| value.unwrap_or(false))
    .map_err(|err| format!("failed to inspect existing seeded assets: {err}"))
}

fn asset_exists_for_seed_tx(
    tx: &Transaction<'_>,
    asset_code: &str,
) -> Result<bool, String> {
    tx.query_row(
        r#"
        SELECT 1
        FROM assets
        WHERE asset_code = ? COLLATE NOCASE
        LIMIT 1
        "#,
        params![asset_code],
        |_| Ok(true),
    )
    .optional()
    .map(|value| value.unwrap_or(false))
    .map_err(|err| format!("failed to inspect existing seeded assets: {err}"))
}

fn load_category_labels_by_id_conn(
    conn: &Connection,
    category_id: i64,
) -> Result<(String, String), String> {
    conn.query_row(
        "SELECT category_code, category_name FROM asset_categories WHERE id = ? LIMIT 1",
        params![category_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(|err| format!("failed to load asset category labels for id {category_id}: {err}"))
}

fn load_category_id_by_code_tx(
    tx: &Transaction<'_>,
    category_code: Option<&str>,
) -> Result<Option<i64>, String> {
    let Some(category_code) = category_code else {
        return Ok(None);
    };

    tx.query_row(
        "SELECT id FROM asset_categories WHERE category_code = ? COLLATE NOCASE LIMIT 1",
        params![category_code],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| format!("failed to load asset category id for '{category_code}': {err}"))
}

fn require_import_actor_id(conn: &Connection) -> Result<i64, String> {
    let Some(actor_id) = auth::get_active_local_account_id(conn)? else {
        return Err("Only admin accounts can create assets from Employee List.".to_string());
    };

    let role = conn
        .query_row(
            "SELECT role FROM app_local_accounts WHERE id = ? LIMIT 1",
            params![actor_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("failed to resolve active local account role: {err}"))?
        .ok_or_else(|| "Only admin accounts can create assets from Employee List.".to_string())?;

    if role.eq_ignore_ascii_case("admin") || role.eq_ignore_ascii_case("super_admin") {
        Ok(actor_id)
    } else {
        Err("Only admin accounts can create assets from Employee List.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use crate::db::{apply_migrations, auth, configure_connection};

    use super::{
        import_employee_asset_seed_conn, preview_employee_asset_seed_conn, EmployeeAssetSeedInput,
        SEED_SOURCE_LABEL,
    };

    fn open_test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        configure_connection(&conn).expect("configure sqlite pragmas");
        apply_migrations(&conn).expect("apply migrations");
        conn
    }

    fn seed_employee(
        conn: &Connection,
        employee_id: &str,
        full_name: &str,
        stored_computer_name: Option<&str>,
        staff_group: &str,
    ) -> i64 {
        conn.execute(
            r#"
            INSERT INTO employees(
              employee_id,
              full_name,
              computername,
              staff_group,
              updated_at
            )
            VALUES(?, ?, ?, ?, datetime('now'))
            "#,
            params![employee_id, full_name, stored_computer_name, staff_group],
        )
        .expect("insert employee");

        conn.last_insert_rowid()
    }

    fn seed_laptop_asset(conn: &Connection, asset_code: &str) {
        conn.execute(
            r#"
            INSERT INTO assets(
              asset_code,
              asset_type,
              display_name,
              status,
              created_at,
              updated_at
            )
            VALUES(?, 'Laptop', ?, 'in_stock', datetime('now'), datetime('now'))
            "#,
            params![asset_code, asset_code],
        )
        .expect("insert asset");
    }

    #[test]
    fn preview_employee_asset_seed_derives_asset_code_computer_name_and_category_from_stored_value() {
        let mut conn = open_test_connection();
        seed_employee(
            &conn,
            "ASWVN1302",
            "Nguyen Van A",
            Some("ASWVNLAP293"),
            "employee_list",
        );

        let preview = preview_employee_asset_seed_conn(
            &mut conn,
            &EmployeeAssetSeedInput {
                staff_group: Some("employee_list".to_string()),
                ..EmployeeAssetSeedInput::default()
            },
        )
        .expect("preview employee asset seed");

        assert!(preview.snapshot_id > 0);
        assert_eq!(preview.source_label, SEED_SOURCE_LABEL);
        assert_eq!(preview.matched_employee_count, 1);
        assert_eq!(preview.excluded_rows, 0);
        assert_eq!(preview.total_rows, 1);
        assert_eq!(preview.valid_rows, 1);
        assert_eq!(preview.error_rows, 0);
        assert_eq!(preview.rows[0].employee_id, "ASWVN1302");
        assert_eq!(preview.rows[0].asset_code.as_deref(), Some("VNLAP293"));
        assert_eq!(preview.rows[0].computer_name.as_deref(), Some("ASWVNLAP293"));
        assert_eq!(preview.rows[0].category_code.as_deref(), Some("laptop"));
    }

    #[test]
    fn preview_employee_asset_seed_flags_duplicate_normalized_computer_names() {
        let mut conn = open_test_connection();
        seed_employee(
            &conn,
            "ASWVN1302",
            "Nguyen Van A",
            Some("ASWVNLAP293"),
            "employee_list",
        );
        seed_employee(
            &conn,
            "ASWVN1303",
            "Nguyen Van B",
            Some("VNLAP293"),
            "employee_list",
        );

        let preview = preview_employee_asset_seed_conn(
            &mut conn,
            &EmployeeAssetSeedInput {
                staff_group: Some("employee_list".to_string()),
                ..EmployeeAssetSeedInput::default()
            },
        )
        .expect("preview employee asset seed with duplicates");

        assert_eq!(preview.total_rows, 2);
        assert_eq!(preview.valid_rows, 0);
        assert_eq!(preview.error_rows, 2);
        assert_eq!(preview.errors.len(), 2);
        assert!(preview.errors[0].reason.to_lowercase().contains("duplicate"));
    }

    #[test]
    fn preview_employee_asset_seed_reports_excluded_rows_for_blank_sources() {
        let mut conn = open_test_connection();
        seed_employee(
            &conn,
            "ASWVN1302",
            "Nguyen Van A",
            Some("ASWVNLAP293"),
            "employee_list",
        );
        seed_employee(
            &conn,
            "ASWVN1303",
            "Nguyen Van B",
            None,
            "employee_list",
        );

        let preview = preview_employee_asset_seed_conn(
            &mut conn,
            &EmployeeAssetSeedInput {
                staff_group: Some("employee_list".to_string()),
                ..EmployeeAssetSeedInput::default()
            },
        )
        .expect("preview employee asset seed with excluded row");

        assert_eq!(preview.matched_employee_count, 2);
        assert_eq!(preview.total_rows, 1);
        assert_eq!(preview.valid_rows, 1);
        assert_eq!(preview.error_rows, 0);
        assert_eq!(preview.excluded_rows, 1);
        assert!(preview.errors.iter().any(|item| item.reason.contains("blank or invalid")));
    }

    #[test]
    fn import_employee_asset_seed_imports_only_reviewed_snapshot_rows() {
        let mut conn = open_test_connection();
        seed_employee(
            &conn,
            "ASWVN1302",
            "Nguyen Van A",
            Some("ASWVNLAP293"),
            "employee_list",
        );
        seed_employee(
            &conn,
            "ASWVN1303",
            "Nguyen Van B",
            Some("VNLAP294"),
            "employee_list",
        );

        let preview = preview_employee_asset_seed_conn(
            &mut conn,
            &EmployeeAssetSeedInput {
                staff_group: Some("employee_list".to_string()),
                ..EmployeeAssetSeedInput::default()
            },
        )
        .expect("preview employee asset seed before data drift");

        conn.execute(
            "UPDATE employees SET computername = 'VNLAP999' WHERE employee_id = 'ASWVN1303'",
            [],
        )
        .expect("update employee stored computer name after preview");

        let report = import_employee_asset_seed_conn(
            &mut conn,
            &EmployeeAssetSeedInput {
                snapshot_id: Some(preview.snapshot_id),
                ..EmployeeAssetSeedInput::default()
            },
        )
        .expect("import employee asset seed from reviewed snapshot");

        assert_eq!(report.snapshot_id, preview.snapshot_id);
        assert_eq!(report.total_rows, 2);
        assert_eq!(report.imported, 2);
        assert_eq!(report.skipped, 0);
        assert_eq!(report.failed, 0);
        assert_eq!(
            report.imported_asset_codes,
            vec!["VNLAP293".to_string(), "VNLAP294".to_string()]
        );

        let imported_codes = conn
            .prepare("SELECT asset_code FROM assets ORDER BY asset_code ASC")
            .expect("prepare imported asset query")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query imported asset codes")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect imported asset codes");
        assert_eq!(imported_codes, vec!["VNLAP293".to_string(), "VNLAP294".to_string()]);

        let imported_display_names = conn
            .prepare("SELECT asset_code, display_name FROM assets ORDER BY asset_code ASC")
            .expect("prepare imported asset display-name query")
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .expect("query imported asset display names")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect imported asset display names");
        assert_eq!(
            imported_display_names,
            vec![
                ("VNLAP293".to_string(), "LAP293".to_string()),
                ("VNLAP294".to_string(), "LAP294".to_string()),
            ]
        );
    }

    #[test]
    fn import_employee_asset_seed_imports_only_new_valid_assets() {
        let mut conn = open_test_connection();
        seed_employee(
            &conn,
            "ASWVN1302",
            "Nguyen Van A",
            Some("ASWVNLAP293"),
            "employee_list",
        );
        seed_employee(
            &conn,
            "ASWVN1303",
            "Nguyen Van B",
            Some("VNLAP294"),
            "employee_list",
        );
        seed_laptop_asset(&conn, "VNLAP293");

        let preview = preview_employee_asset_seed_conn(
            &mut conn,
            &EmployeeAssetSeedInput {
                staff_group: Some("employee_list".to_string()),
                ..EmployeeAssetSeedInput::default()
            },
        )
        .expect("preview employee asset seed before import");

        let report = import_employee_asset_seed_conn(
            &mut conn,
            &EmployeeAssetSeedInput {
                snapshot_id: Some(preview.snapshot_id),
                ..EmployeeAssetSeedInput::default()
            },
        )
        .expect("import employee asset seed");

        assert_eq!(report.total_rows, 2);
        assert_eq!(report.imported, 1);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.failed, 0);
        assert_eq!(report.imported_asset_codes, vec!["VNLAP294".to_string()]);

        let asset_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM assets WHERE asset_code LIKE 'VNLAP29%'", [], |row| {
                row.get(0)
            })
            .expect("count seeded laptop assets");
        assert_eq!(asset_count, 2);
    }

    #[test]
    fn preview_employee_asset_seed_requires_admin_permission() {
        let mut conn = open_test_connection();
        let active_id = auth::get_active_local_account_id(&conn)
            .expect("get active local account id")
            .expect("active account id should exist");

        conn.execute(
            "UPDATE app_local_accounts SET role = 'user' WHERE id = ?",
            params![active_id],
        )
        .expect("downgrade active account to user");

        let error = preview_employee_asset_seed_conn(
            &mut conn,
            &EmployeeAssetSeedInput {
                staff_group: Some("employee_list".to_string()),
                ..EmployeeAssetSeedInput::default()
            },
        )
        .expect_err("preview should require admin permission");

        assert!(error.contains("Only admin accounts"));
    }

    #[test]
    fn import_employee_asset_seed_strips_vn_prefix_for_macpro_display_name() {
        let mut conn = open_test_connection();
        seed_employee(
            &conn,
            "ASWVN1302",
            "Nguyen Van A",
            Some("ASWVNMACPRO010"),
            "employee_list",
        );

        let preview = preview_employee_asset_seed_conn(
            &mut conn,
            &EmployeeAssetSeedInput {
                staff_group: Some("employee_list".to_string()),
                ..EmployeeAssetSeedInput::default()
            },
        )
        .expect("preview employee asset seed");

        let report = import_employee_asset_seed_conn(
            &mut conn,
            &EmployeeAssetSeedInput {
                snapshot_id: Some(preview.snapshot_id),
                ..EmployeeAssetSeedInput::default()
            },
        )
        .expect("import employee asset seed");

        assert_eq!(report.imported_asset_codes, vec!["VNMACPRO010".to_string()]);

        let display_name = conn
            .query_row(
                "SELECT display_name FROM assets WHERE asset_code = 'VNMACPRO010'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("load imported macpro display name");
        assert_eq!(display_name, "MACPRO010");
    }
}