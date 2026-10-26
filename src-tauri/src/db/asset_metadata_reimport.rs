use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;

use crate::db::asset::canonicalize_asset_code;
use crate::db::audit::insert_audit_log_tx;
use crate::db::open_runtime_connection;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetMetadataDiff {
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AssetMetadataRowStatus {
    MetadataChanged,
    Unchanged,
    MissingAsset,
    OutOfScope,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetMetadataPreviewRow {
    pub row_number: usize,
    pub raw_asset_tag: String,
    pub canonical_asset_code: String,
    pub raw_category: String,
    pub status: AssetMetadataRowStatus,
    pub status_reason: Option<String>,
    pub asset_id: Option<i64>,
    pub current_display_name: Option<String>,
    pub current_brand: Option<String>,
    pub current_model: Option<String>,
    pub current_serial_number: Option<String>,
    pub proposed_display_name: Option<String>,
    pub proposed_brand: Option<String>,
    pub proposed_model: Option<String>,
    pub proposed_serial_number: Option<String>,
    pub diffs: Vec<AssetMetadataDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetMetadataPreviewSummary {
    pub total_rows: usize,
    pub monitor_rows: usize,
    pub metadata_changed: usize,
    pub unchanged: usize,
    pub missing_asset: usize,
    pub out_of_scope: usize,
    pub invalid: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetMetadataPreviewReport {
    pub file_name: String,
    pub sheet_name: String,
    pub summary: AssetMetadataPreviewSummary,
    pub rows: Vec<AssetMetadataPreviewRow>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetMetadataImportInput {
    pub file_path: String,
    pub sheet_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetMetadataImportReport {
    pub total_processed: usize,
    pub updated_count: usize,
    pub unchanged_count: usize,
    pub skipped_count: usize,
    pub errors: Vec<String>,
}

pub fn preview_asset_metadata_reimport(
    app: &AppHandle,
    payload: AssetMetadataImportInput,
) -> Result<AssetMetadataPreviewReport, String> {
    let conn = open_runtime_connection(app)?;
    preview_asset_metadata_reimport_conn(&conn, &payload)
}

pub fn apply_asset_metadata_reimport(
    app: &AppHandle,
    payload: AssetMetadataImportInput,
    reviewer_account_id: Option<i64>,
) -> Result<AssetMetadataImportReport, String> {
    let mut conn = open_runtime_connection(app)?;
    apply_asset_metadata_reimport_conn(&mut conn, &payload, reviewer_account_id)
}

#[derive(Debug, Clone)]
struct RawParsedRow {
    row_number: usize,
    asset_tag: String,
    display_name: Option<String>,
    category: String,
    brand: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{:.0}", f)
            } else {
                format!("{}", f)
            }
        }
        Data::Int(i) => format!("{}", i),
        Data::Bool(b) => format!("{}", b),
        Data::DateTime(dt) => format!("{}", dt),
        Data::DateTimeIso(s) => s.trim().to_string(),
        Data::DurationIso(s) => s.trim().to_string(),
        _ => String::new(),
    }
}

fn normalize_header(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn parse_metadata_sheet(
    file_path: &str,
    sheet_name: Option<&str>,
) -> Result<(String, String, Vec<RawParsedRow>), String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File '{}' does not exist", file_path));
    }

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path)
        .to_string();

    let mut workbook = open_workbook_auto(path)
        .map_err(|err| format!("Failed to open Excel workbook '{file_path}': {err}"))?;

    let sheet_to_read = if let Some(sn) = sheet_name {
        sn.to_string()
    } else {
        // Look for sheet named "Asset" or fallback to first sheet
        let sheets = workbook.sheet_names();
        sheets
            .iter()
            .find(|s| s.eq_ignore_ascii_case("Asset"))
            .cloned()
            .or_else(|| sheets.first().cloned())
            .ok_or_else(|| "Workbook has no sheets".to_string())?
    };

    let range = workbook
        .worksheet_range(&sheet_to_read)
        .map_err(|err| format!("Failed to read sheet '{sheet_to_read}': {err}"))?;

    let mut tag_col = None;
    let mut name_col = None;
    let mut category_col = None;
    let mut brand_col = None;
    let mut model_col = None;
    let mut serial_col = None;
    let mut header_row_idx = None;

    for (r_idx, row) in range.rows().enumerate() {
        for (c_idx, cell) in row.iter().enumerate() {
            let h = normalize_header(&cell_to_string(cell));
            if h == "assettag" || h == "assetcode" || h == "tag" {
                tag_col = Some(c_idx);
            } else if h == "assetname" || h == "displayname" || h == "name" {
                name_col = Some(c_idx);
            } else if h == "category" || h == "assettype" || h == "type" {
                category_col = Some(c_idx);
            } else if h == "brand" {
                brand_col = Some(c_idx);
            } else if h == "model" {
                model_col = Some(c_idx);
            } else if h == "serrialnumber" || h == "serialnumber" || h == "serial" {
                serial_col = Some(c_idx);
            }
        }

        if tag_col.is_some() && name_col.is_some() {
            header_row_idx = Some(r_idx);
            break;
        }
    }

    let header_idx = header_row_idx
        .ok_or_else(|| "Could not identify header row (required: Asset Tag, Asset Name)".to_string())?;
    let t_col = tag_col
        .ok_or_else(|| "Missing 'Asset Tag' column in sheet".to_string())?;

    let mut raw_rows = Vec::new();

    for (r_idx, row) in range.rows().enumerate().skip(header_idx + 1) {
        let tag = row
            .get(t_col)
            .map(cell_to_string)
            .unwrap_or_default()
            .trim()
            .to_string();

        let name = name_col
            .and_then(|col| row.get(col))
            .map(cell_to_string)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let category = category_col
            .and_then(|col| row.get(col))
            .map(cell_to_string)
            .unwrap_or_default()
            .trim()
            .to_string();

        let brand = brand_col
            .and_then(|col| row.get(col))
            .map(cell_to_string)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let model = model_col
            .and_then(|col| row.get(col))
            .map(cell_to_string)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let serial = serial_col
            .and_then(|col| row.get(col))
            .map(cell_to_string)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        // Skip completely empty rows
        if tag.is_empty() && name.is_none() && category.is_empty() && brand.is_none() && model.is_none() && serial.is_none() {
            continue;
        }

        raw_rows.push(RawParsedRow {
            row_number: r_idx + 1,
            asset_tag: tag,
            display_name: name,
            category,
            brand,
            model,
            serial_number: serial,
        });
    }

    Ok((file_name, sheet_to_read, raw_rows))
}

pub fn preview_asset_metadata_reimport_conn(
    conn: &Connection,
    payload: &AssetMetadataImportInput,
) -> Result<AssetMetadataPreviewReport, String> {
    let (file_name, sheet_name, raw_rows) = parse_metadata_sheet(&payload.file_path, payload.sheet_name.as_deref())?;

    let mut preview_rows = Vec::new();
    let mut total_rows = 0;
    let mut monitor_rows = 0;
    let mut metadata_changed = 0;
    let mut unchanged = 0;
    let mut missing_asset = 0;
    let mut out_of_scope = 0;
    let mut invalid = 0;

    for r in raw_rows {
        total_rows += 1;
        let is_monitor = r.category.trim().eq_ignore_ascii_case("Monitor")
            || (r.category.trim().is_empty() && r.asset_tag.to_ascii_uppercase().contains("MON"));

        if !is_monitor {
            out_of_scope += 1;
            preview_rows.push(AssetMetadataPreviewRow {
                row_number: r.row_number,
                raw_asset_tag: r.asset_tag,
                canonical_asset_code: String::new(),
                raw_category: r.category,
                status: AssetMetadataRowStatus::OutOfScope,
                status_reason: Some("Excluded from metadata update: scope is limited to Monitor assets only".to_string()),
                asset_id: None,
                current_display_name: None,
                current_brand: None,
                current_model: None,
                current_serial_number: None,
                proposed_display_name: r.display_name,
                proposed_brand: r.brand,
                proposed_model: r.model,
                proposed_serial_number: r.serial_number,
                diffs: Vec::new(),
            });
            continue;
        }

        monitor_rows += 1;
        let canonical_code = canonicalize_asset_code(&r.asset_tag);
        if canonical_code.is_empty() {
            invalid += 1;
            preview_rows.push(AssetMetadataPreviewRow {
                row_number: r.row_number,
                raw_asset_tag: r.asset_tag,
                canonical_asset_code: String::new(),
                raw_category: r.category,
                status: AssetMetadataRowStatus::Invalid,
                status_reason: Some("Asset tag is empty or cannot be canonicalized".to_string()),
                asset_id: None,
                current_display_name: None,
                current_brand: None,
                current_model: None,
                current_serial_number: None,
                proposed_display_name: r.display_name,
                proposed_brand: r.brand,
                proposed_model: r.model,
                proposed_serial_number: r.serial_number,
                diffs: Vec::new(),
            });
            continue;
        }

        // Query existing asset in database
        let db_asset: Option<(i64, String, Option<String>, Option<String>, Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT id, asset_code, display_name, brand, model, serial_number
                 FROM assets
                 WHERE UPPER(asset_code) = UPPER(?)",
                params![canonical_code],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| format!("Database error looking up asset '{canonical_code}': {e}"))?;

        let Some((asset_id, actual_code, cur_name, cur_brand, cur_model, cur_serial)) = db_asset else {
            missing_asset += 1;
            preview_rows.push(AssetMetadataPreviewRow {
                row_number: r.row_number,
                raw_asset_tag: r.asset_tag,
                canonical_asset_code: canonical_code.clone(),
                raw_category: r.category,
                status: AssetMetadataRowStatus::MissingAsset,
                status_reason: Some(format!("Asset '{canonical_code}' not found in database")),
                asset_id: None,
                current_display_name: None,
                current_brand: None,
                current_model: None,
                current_serial_number: None,
                proposed_display_name: r.display_name,
                proposed_brand: r.brand,
                proposed_model: r.model,
                proposed_serial_number: r.serial_number,
                diffs: Vec::new(),
            });
            continue;
        };

        let mut diffs = Vec::new();

        // Rule: Blank Excel fields must NEVER clear existing values.
        // Compare display_name
        if let Some(ref new_name) = r.display_name {
            if Some(new_name.as_str()) != cur_name.as_deref() {
                diffs.push(AssetMetadataDiff {
                    field: "display_name".to_string(),
                    before: cur_name.clone(),
                    after: Some(new_name.clone()),
                });
            }
        }

        // Compare brand
        if let Some(ref new_brand) = r.brand {
            if Some(new_brand.as_str()) != cur_brand.as_deref() {
                diffs.push(AssetMetadataDiff {
                    field: "brand".to_string(),
                    before: cur_brand.clone(),
                    after: Some(new_brand.clone()),
                });
            }
        }

        // Compare model
        if let Some(ref new_model) = r.model {
            if Some(new_model.as_str()) != cur_model.as_deref() {
                diffs.push(AssetMetadataDiff {
                    field: "model".to_string(),
                    before: cur_model.clone(),
                    after: Some(new_model.clone()),
                });
            }
        }

        // Compare serial_number
        if let Some(ref new_serial) = r.serial_number {
            if Some(new_serial.as_str()) != cur_serial.as_deref() {
                diffs.push(AssetMetadataDiff {
                    field: "serial_number".to_string(),
                    before: cur_serial.clone(),
                    after: Some(new_serial.clone()),
                });
            }
        }

        let status = if !diffs.is_empty() {
            metadata_changed += 1;
            AssetMetadataRowStatus::MetadataChanged
        } else {
            unchanged += 1;
            AssetMetadataRowStatus::Unchanged
        };

        let status_reason = if !diffs.is_empty() {
            Some(format!("{} field(s) to be updated", diffs.len()))
        } else {
            Some("Metadata matches existing database record".to_string())
        };

        preview_rows.push(AssetMetadataPreviewRow {
            row_number: r.row_number,
            raw_asset_tag: r.asset_tag,
            canonical_asset_code: actual_code,
            raw_category: r.category,
            status,
            status_reason,
            asset_id: Some(asset_id),
            current_display_name: cur_name,
            current_brand: cur_brand,
            current_model: cur_model,
            current_serial_number: cur_serial,
            proposed_display_name: r.display_name,
            proposed_brand: r.brand,
            proposed_model: r.model,
            proposed_serial_number: r.serial_number,
            diffs,
        });
    }

    Ok(AssetMetadataPreviewReport {
        file_name,
        sheet_name,
        summary: AssetMetadataPreviewSummary {
            total_rows,
            monitor_rows,
            metadata_changed,
            unchanged,
            missing_asset,
            out_of_scope,
            invalid,
        },
        rows: preview_rows,
    })
}

pub fn apply_asset_metadata_reimport_conn(
    conn: &mut Connection,
    payload: &AssetMetadataImportInput,
    reviewer_account_id: Option<i64>,
) -> Result<AssetMetadataImportReport, String> {
    // 1. Generate preview inside transaction for revalidation
    let preview = preview_asset_metadata_reimport_conn(conn, payload)?;

    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start transaction: {e}"))?;

    let mut updated_count = 0;
    let mut unchanged_count = 0;
    let mut skipped_count = 0;
    let mut errors = Vec::new();

    for row in &preview.rows {
        match row.status {
            AssetMetadataRowStatus::MetadataChanged => {
                let Some(asset_id) = row.asset_id else {
                    errors.push(format!("Row {}: missing asset ID for update", row.row_number));
                    continue;
                };

                // Determine final values: proposed if non-blank, otherwise keep current
                let final_display_name = row
                    .proposed_display_name
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .or(row.current_display_name.as_deref())
                    .unwrap_or("");

                let final_brand = row
                    .proposed_brand
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .or(row.current_brand.as_deref());

                let final_model = row
                    .proposed_model
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .or(row.current_model.as_deref());

                let final_serial = row
                    .proposed_serial_number
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .or(row.current_serial_number.as_deref());

                let rows_affected = tx
                    .execute(
                        "UPDATE assets
                         SET display_name = ?,
                             brand = ?,
                             model = ?,
                             serial_number = ?,
                             updated_at = datetime('now')
                         WHERE id = ?",
                        params![
                            final_display_name,
                            final_brand,
                            final_model,
                            final_serial,
                            asset_id,
                        ],
                    )
                    .map_err(|e| format!("Failed to update asset ID {asset_id}: {e}"))?;

                if rows_affected == 1 {
                    updated_count += 1;

                    let payload_json = serde_json::to_string(&json!({
                        "asset_code": row.canonical_asset_code,
                        "diffs": row.diffs,
                    }))
                    .ok();

                    insert_audit_log_tx(
                        &tx,
                        "asset_metadata_reimport",
                        "user",
                        reviewer_account_id.map(|id| id.to_string()).as_deref(),
                        "asset",
                        &asset_id.to_string(),
                        payload_json.as_deref(),
                    )?;
                } else {
                    errors.push(format!(
                        "Row {}: update affected {} rows (expected 1)",
                        row.row_number, rows_affected
                    ));
                }
            }
            AssetMetadataRowStatus::Unchanged => {
                unchanged_count += 1;
            }
            AssetMetadataRowStatus::OutOfScope
            | AssetMetadataRowStatus::MissingAsset
            | AssetMetadataRowStatus::Invalid => {
                skipped_count += 1;
            }
        }
    }

    if !errors.is_empty() {
        return Err(format!("Aborting metadata update due to errors: {:?}", errors));
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit metadata update transaction: {e}"))?;

    Ok(AssetMetadataImportReport {
        total_processed: preview.summary.total_rows,
        updated_count,
        unchanged_count,
        skipped_count,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(test_name: &str) -> std::path::PathBuf {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("timestamp")
            .as_nanos();
        std::env::temp_dir().join(format!("staffkit-{test_name}-{timestamp}.sqlite3"))
    }

    #[test]
    fn test_preview_and_apply_asset_metadata_reimport_on_zon_asset_list() {
        let original_db = Path::new(r"D:\Staff_Kit\data\test_db\staff_kit.sqlite3");
        assert!(original_db.exists(), "original test db must exist");

        let temp_db = temp_db_path("metadata-reimport-zon");
        std::fs::copy(original_db, &temp_db).expect("copy test db to temp");
        let mut conn = crate::db::open_encrypted_connection(&temp_db).expect("open temp db");

        let workbook_path = r"D:\Staff_Kit\00_ExSource\AssetList.xlsx";
        assert!(Path::new(workbook_path).exists(), "AssetList.xlsx must exist");

        // Baseline counts before preview
        let initial_loans: i64 = conn.query_row("SELECT count(*) FROM asset_loans", [], |r| r.get(0)).unwrap();
        let initial_borrow_reqs: i64 = conn.query_row("SELECT count(*) FROM borrow_requests", [], |r| r.get(0)).unwrap();
        let initial_employees: i64 = conn.query_row("SELECT count(*) FROM employees", [], |r| r.get(0)).unwrap();
        let initial_assets: i64 = conn.query_row("SELECT count(*) FROM assets", [], |r| r.get(0)).unwrap();

        // 1. Preview
        let payload = AssetMetadataImportInput {
            file_path: workbook_path.to_string(),
            sheet_name: None,
        };
        let preview = preview_asset_metadata_reimport_conn(&conn, &payload).expect("preview must succeed");
        println!("PREVIEW_SUMMARY: {:?}", preview.summary);
        for row in &preview.rows {
            if row.status == AssetMetadataRowStatus::MissingAsset {
                println!("MISSING ASSET: Row {}: tag='{}', canonical='{}'", row.row_number, row.raw_asset_tag, row.canonical_asset_code);
            }
        }

        assert_eq!(preview.summary.total_rows, 219, "Total rows must be 219");
        assert_eq!(preview.summary.monitor_rows, 209, "Monitor rows must be 209");
        let monitor_tags: std::collections::HashSet<_> = preview.rows.iter()
            .filter(|r| r.status != AssetMetadataRowStatus::OutOfScope)
            .map(|r| &r.raw_asset_tag)
            .collect();
        assert_eq!(monitor_tags.len(), 209, "All 209 Monitor rows must have unique Asset Tags");
        assert_eq!(preview.summary.out_of_scope, 10, "OutOfScope must be 10 (6 Laptop + 4 H Phone)");
        assert_eq!(preview.summary.invalid, 0, "Invalid rows must be 0");

        // Zero DB writes during preview verification
        let post_preview_assets: i64 = conn.query_row("SELECT count(*) FROM assets", [], |r| r.get(0)).unwrap();
        assert_eq!(initial_assets, post_preview_assets, "Preview must perform ZERO writes to assets");
        let post_preview_loans: i64 = conn.query_row("SELECT count(*) FROM asset_loans", [], |r| r.get(0)).unwrap();
        assert_eq!(initial_loans, post_preview_loans, "Preview must perform ZERO writes to loans");

        // Verify non-monitor assets are OutOfScope and have no diffs
        let out_of_scope_rows: Vec<_> = preview.rows.iter().filter(|r| r.status == AssetMetadataRowStatus::OutOfScope).collect();
        assert_eq!(out_of_scope_rows.len(), 10);
        for row in &out_of_scope_rows {
            assert!(row.diffs.is_empty(), "OutOfScope rows must have zero diffs");
            assert!(row.raw_category == "Laptop" || row.raw_category == "H Phone");
        }

        // 2. Apply metadata update
        let report = apply_asset_metadata_reimport_conn(&mut conn, &payload, Some(1)).expect("apply must succeed");
        assert_eq!(report.total_processed, 219);
        assert_eq!(report.updated_count, preview.summary.metadata_changed);
        assert_eq!(report.unchanged_count, preview.summary.unchanged);
        assert_eq!(report.skipped_count, preview.summary.out_of_scope + preview.summary.missing_asset + preview.summary.invalid);
        assert!(report.errors.is_empty());

        // 3. Post-apply Invariants verification
        let post_apply_loans: i64 = conn.query_row("SELECT count(*) FROM asset_loans", [], |r| r.get(0)).unwrap();
        assert_eq!(initial_loans, post_apply_loans, "asset_loans count must remain exactly identical");
        let post_apply_reqs: i64 = conn.query_row("SELECT count(*) FROM borrow_requests", [], |r| r.get(0)).unwrap();
        assert_eq!(initial_borrow_reqs, post_apply_reqs, "borrow_requests count must remain exactly identical");
        let post_apply_employees: i64 = conn.query_row("SELECT count(*) FROM employees", [], |r| r.get(0)).unwrap();
        assert_eq!(initial_employees, post_apply_employees, "employees count must remain exactly identical");
        let post_apply_assets: i64 = conn.query_row("SELECT count(*) FROM assets", [], |r| r.get(0)).unwrap();
        assert_eq!(initial_assets, post_apply_assets, "assets count must remain exactly identical (no assets created)");

        // Verify non-monitor records were untouched
        let laptop_codes: Vec<String> = conn
            .prepare("SELECT asset_code FROM assets WHERE asset_type = 'Laptop' LIMIT 5")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        for code in laptop_codes {
            let audit_count: i64 = conn
                .query_row(
                    "SELECT count(*) FROM audit_logs WHERE event_type = 'asset_metadata_reimport' AND payload_json LIKE ?",
                    params![format!("%{code}%")],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(audit_count, 0, "Laptop {code} must NOT have been updated");
        }

        // 4. Idempotency verification: running again should result in 0 metadata changed
        let second_preview = preview_asset_metadata_reimport_conn(&conn, &payload).expect("second preview");
        assert_eq!(second_preview.summary.metadata_changed, 0, "Second preview must have 0 changed rows");
        assert_eq!(
            second_preview.summary.unchanged,
            preview.summary.metadata_changed + preview.summary.unchanged,
            "All previously changed rows must now be Unchanged"
        );

        let second_apply = apply_asset_metadata_reimport_conn(&mut conn, &payload, Some(1)).expect("second apply");
        assert_eq!(second_apply.updated_count, 0, "Second apply must update 0 rows");

        let _ = std::fs::remove_file(&temp_db);
    }

    #[test]
    fn test_blank_excel_fields_never_clear_existing_db_values() {
        let original_db = Path::new(r"D:\Staff_Kit\data\test_db\staff_kit.sqlite3");
        let temp_db = temp_db_path("blank-fields-safety");
        std::fs::copy(original_db, &temp_db).expect("copy test db to temp");
        let conn = crate::db::open_encrypted_connection(&temp_db).expect("open temp db");

        // Insert a monitor with existing brand, model, and serial
        conn.execute(
            "INSERT INTO assets (asset_code, asset_type, display_name, brand, model, serial_number, status)
             VALUES ('VNMON9999', 'Monitor', 'Original Monitor Name', 'Original Brand', 'Original Model', 'SN-ORIGINAL-999', 'in_stock')",
            [],
        ).unwrap();

        // Simulate a parsed row where brand, model, serial are blank/None, but display_name has a new value
        let row = RawParsedRow {
            row_number: 10,
            asset_tag: "MON9999".to_string(),
            display_name: Some("Updated Monitor Name".to_string()),
            category: "Monitor".to_string(),
            brand: None, // Blank in Excel
            model: None, // Blank in Excel
            serial_number: None, // Blank in Excel
        };

        let canonical_code = canonicalize_asset_code(&row.asset_tag);
        assert_eq!(canonical_code, "VNMON9999");

        // Lookup and compare
        let (asset_id, cur_name, cur_brand, cur_model, cur_serial): (i64, String, Option<String>, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT id, display_name, brand, model, serial_number FROM assets WHERE asset_code = ?",
                params![canonical_code],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();

        assert_eq!(cur_name, "Original Monitor Name");
        assert_eq!(cur_brand.as_deref(), Some("Original Brand"));
        assert_eq!(cur_model.as_deref(), Some("Original Model"));
        assert_eq!(cur_serial.as_deref(), Some("SN-ORIGINAL-999"));

        // Blank in Excel must NOT produce diffs for brand, model, serial
        let mut diffs = Vec::new();
        if let Some(ref new_name) = row.display_name {
            if new_name != &cur_name {
                diffs.push(AssetMetadataDiff {
                    field: "display_name".to_string(),
                    before: Some(cur_name.clone()),
                    after: Some(new_name.clone()),
                });
            }
        }
        if let Some(ref new_brand) = row.brand {
            if Some(new_brand.as_str()) != cur_brand.as_deref() {
                diffs.push(AssetMetadataDiff {
                    field: "brand".to_string(),
                    before: cur_brand.clone(),
                    after: Some(new_brand.clone()),
                });
            }
        }
        if let Some(ref new_model) = row.model {
            if Some(new_model.as_str()) != cur_model.as_deref() {
                diffs.push(AssetMetadataDiff {
                    field: "model".to_string(),
                    before: cur_model.clone(),
                    after: Some(new_model.clone()),
                });
            }
        }
        if let Some(ref new_serial) = row.serial_number {
            if Some(new_serial.as_str()) != cur_serial.as_deref() {
                diffs.push(AssetMetadataDiff {
                    field: "serial_number".to_string(),
                    before: cur_serial.clone(),
                    after: Some(new_serial.clone()),
                });
            }
        }

        // Only display_name should be diffed
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].field, "display_name");
        assert_eq!(diffs[0].after.as_deref(), Some("Updated Monitor Name"));

        // Simulate apply with rule: proposed if non-blank, otherwise preserve existing
        let final_display_name = row.display_name.as_deref().unwrap_or(&cur_name);
        let final_brand = row.brand.as_deref().or(cur_brand.as_deref());
        let final_model = row.model.as_deref().or(cur_model.as_deref());
        let final_serial = row.serial_number.as_deref().or(cur_serial.as_deref());

        conn.execute(
            "UPDATE assets SET display_name = ?, brand = ?, model = ?, serial_number = ? WHERE id = ?",
            params![final_display_name, final_brand, final_model, final_serial, asset_id],
        ).unwrap();

        // Verify that brand, model, and serial were 100% PRESERVED, not erased to NULL
        let (post_name, post_brand, post_model, post_serial): (String, Option<String>, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT display_name, brand, model, serial_number FROM assets WHERE id = ?",
                params![asset_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();

        assert_eq!(post_name, "Updated Monitor Name");
        assert_eq!(post_brand.as_deref(), Some("Original Brand"), "Brand must NOT be cleared by blank");
        assert_eq!(post_model.as_deref(), Some("Original Model"), "Model must NOT be cleared by blank");
        assert_eq!(post_serial.as_deref(), Some("SN-ORIGINAL-999"), "Serial must NOT be cleared by blank");

        let _ = std::fs::remove_file(&temp_db);
    }
}
