use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeAssetSeedInput {
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeAssetSeedErrorItem {
    pub row_number: i64,
    pub entity_key: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeAssetSeedPreview {
    pub total_rows: i64,
    pub valid_rows: i64,
    pub error_rows: i64,
    pub rows: Vec<EmployeeAssetSeedPreviewRow>,
    pub errors: Vec<EmployeeAssetSeedErrorItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeAssetSeedReport {
    pub total_rows: i64,
    pub imported: i64,
    pub skipped: i64,
    pub failed: i64,
    pub imported_asset_codes: Vec<String>,
    pub errors: Vec<EmployeeAssetSeedErrorItem>,
}

pub fn preview_employee_asset_seed(
    app: &AppHandle,
    payload: EmployeeAssetSeedInput,
) -> Result<EmployeeAssetSeedPreview, String> {
    let conn = open_runtime_connection(app)?;
    preview_employee_asset_seed_conn(&conn, &payload)
}

pub fn import_employee_asset_seed(
    app: &AppHandle,
    payload: EmployeeAssetSeedInput,
) -> Result<EmployeeAssetSeedReport, String> {
    let mut conn = open_runtime_connection(app)?;
    import_employee_asset_seed_conn(&mut conn, &payload)
}

pub(crate) fn preview_employee_asset_seed_conn(
    conn: &Connection,
    payload: &EmployeeAssetSeedInput,
) -> Result<EmployeeAssetSeedPreview, String> {
    Ok(build_preview(conn, payload)?)
}

pub(crate) fn import_employee_asset_seed_conn(
    conn: &mut Connection,
    payload: &EmployeeAssetSeedInput,
) -> Result<EmployeeAssetSeedReport, String> {
    let preview = build_preview(conn, payload)?;
    let actor_ref = active_actor_ref(conn)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start employee asset seed transaction: {err}"))?;

    let mut imported = 0_i64;
    let mut skipped = preview.error_rows;
    let mut failed = 0_i64;
    let mut imported_asset_codes = Vec::new();
    let mut errors = preview.errors.clone();

    for row in &preview.rows {
        if row.status != ROW_STATUS_VALID {
            continue;
        }

        let Some(asset_code) = row.asset_code.as_deref() else {
            skipped += 1;
            errors.push(EmployeeAssetSeedErrorItem {
                row_number: imported + skipped + failed,
                entity_key: Some(row.employee_id.clone()),
                reason: "asset code could not be derived".to_string(),
            });
            continue;
        };
        let Some(computer_name) = row.computer_name.as_deref() else {
            skipped += 1;
            errors.push(EmployeeAssetSeedErrorItem {
                row_number: imported + skipped + failed,
                entity_key: Some(row.employee_id.clone()),
                reason: "computer name could not be derived".to_string(),
            });
            continue;
        };
        let Some(category_id) = load_category_id_by_code_tx(&tx, row.category_code.as_deref())? else {
            skipped += 1;
            errors.push(EmployeeAssetSeedErrorItem {
                row_number: imported + skipped + failed,
                entity_key: Some(asset_code.to_string()),
                reason: "asset category could not be resolved".to_string(),
            });
            continue;
        };

        if asset_exists_for_seed_tx(&tx, asset_code, computer_name)? {
            skipped += 1;
            errors.push(EmployeeAssetSeedErrorItem {
                row_number: imported + skipped + failed,
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
                display_name: computer_name.to_string(),
                display_name_short: None,
                computer_name: Some(computer_name.to_string()),
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
                    row_number: imported + skipped + failed,
                    entity_key: Some(asset_code.to_string()),
                    reason: err,
                });
            }
        }
    }

    let payload_json = json!({
        "filters": {
            "query": payload.query,
            "teamName": payload.team_name,
            "staffGroup": payload.staff_group,
            "startDateFrom": payload.start_date_from,
            "startDateTo": payload.start_date_to,
        },
        "totalRows": preview.total_rows,
        "imported": imported,
        "skipped": skipped,
        "failed": failed,
        "importedAssetCodes": imported_asset_codes,
    })
    .to_string();

    audit::insert_audit_log_tx(
        &tx,
        "employee_asset_seed.import",
        "local_account",
        actor_ref.as_deref(),
        "employee_asset_seed",
        DEFAULT_STAFF_GROUP,
        Some(payload_json.as_str()),
    )?;

    tx.commit()
        .map_err(|err| format!("failed to commit employee asset seed transaction: {err}"))?;

    Ok(EmployeeAssetSeedReport {
        total_rows: preview.total_rows,
        imported,
        skipped,
        failed,
        imported_asset_codes,
        errors,
    })
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

fn build_preview(
    conn: &Connection,
    payload: &EmployeeAssetSeedInput,
) -> Result<EmployeeAssetSeedPreview, String> {
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
    let mut rows = build_candidate_rows(conn, employees)?;
    rows.sort_by(|left, right| left.row_number.cmp(&right.row_number));

    let valid_rows = rows.iter().filter(|row| row.status == ROW_STATUS_VALID).count() as i64;
    let error_rows = rows.iter().filter(|row| row.status == ROW_STATUS_ERROR).count() as i64;
    let errors = rows
        .iter()
        .filter_map(|row| {
            row.reason.as_ref().map(|reason| EmployeeAssetSeedErrorItem {
                row_number: row.row_number,
                entity_key: Some(row.employee_id.clone()),
                reason: reason.clone(),
            })
        })
        .collect::<Vec<_>>();

    Ok(EmployeeAssetSeedPreview {
        total_rows: rows.len() as i64,
        valid_rows,
        error_rows,
        rows: rows
            .into_iter()
            .map(|row| EmployeeAssetSeedPreviewRow {
                employee_id: row.employee_id,
                full_name: row.full_name,
                source_computer_name: row.source_computer_name,
                asset_code: row.asset_code,
                computer_name: row.computer_name,
                category_code: row.category_code,
                category_name: row.category_name,
                status: row.status,
            })
            .collect(),
        errors,
    })
}

fn build_candidate_rows(
    conn: &Connection,
    employees: Vec<EmployeeRecord>,
) -> Result<Vec<CandidateRow>, String> {
    let source_rows = employees
        .into_iter()
        .filter_map(|employee| {
            normalize_seed_source(employee.stored_computer_name.as_deref()).map(|source| (employee, source))
        })
        .collect::<Vec<_>>();

    let mut duplicate_counts = HashMap::<String, usize>::new();
    for (_, source) in &source_rows {
        if let Ok((asset_code, _)) = derive_asset_identity(source.as_str()) {
            *duplicate_counts.entry(asset_code).or_default() += 1;
        }
    }

    let mut rows = Vec::with_capacity(source_rows.len());
    for (index, (employee, source)) in source_rows.into_iter().enumerate() {
        let row_number = i64::try_from(index + 1).unwrap_or(i64::MAX);

        let (asset_code, computer_name) = derive_asset_identity(source.as_str())?;

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
        if asset_exists_for_seed_conn(conn, asset_code.as_str(), computer_name.as_str())? {
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

    Ok(rows)
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

fn asset_exists_for_seed_conn(
    conn: &Connection,
    asset_code: &str,
    computer_name: &str,
) -> Result<bool, String> {
    conn.query_row(
        r#"
        SELECT 1
        FROM assets
        WHERE asset_code = ? COLLATE NOCASE
           OR computer_name = ? COLLATE NOCASE
        LIMIT 1
        "#,
        params![asset_code, computer_name],
        |_| Ok(true),
    )
    .optional()
    .map(|value| value.unwrap_or(false))
    .map_err(|err| format!("failed to inspect existing seeded assets: {err}"))
}

fn asset_exists_for_seed_tx(
    tx: &rusqlite::Transaction<'_>,
    asset_code: &str,
    computer_name: &str,
) -> Result<bool, String> {
    tx.query_row(
        r#"
        SELECT 1
        FROM assets
        WHERE asset_code = ? COLLATE NOCASE
           OR computer_name = ? COLLATE NOCASE
        LIMIT 1
        "#,
        params![asset_code, computer_name],
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
    tx: &rusqlite::Transaction<'_>,
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

fn active_actor_ref(conn: &Connection) -> Result<Option<String>, String> {
    Ok(auth::get_active_local_account_id(conn)?.map(|id| id.to_string()))
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use crate::db::{apply_migrations, configure_connection};

    use super::{
        import_employee_asset_seed_conn, preview_employee_asset_seed_conn, EmployeeAssetSeedInput,
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
        let conn = open_test_connection();
        seed_employee(
            &conn,
            "ASWVN1302",
            "Nguyen Van A",
            Some("ASWVNLAP293"),
            "employee_list",
        );

        let preview = preview_employee_asset_seed_conn(
            &conn,
            &EmployeeAssetSeedInput {
                staff_group: Some("employee_list".to_string()),
                ..EmployeeAssetSeedInput::default()
            },
        )
        .expect("preview employee asset seed");

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
        let conn = open_test_connection();
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
            &conn,
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

        let report = import_employee_asset_seed_conn(
            &mut conn,
            &EmployeeAssetSeedInput {
                staff_group: Some("employee_list".to_string()),
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
}
