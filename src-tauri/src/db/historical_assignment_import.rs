use std::collections::HashMap;
use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;

use crate::db::{audit, open_runtime_connection};

pub fn preview_historical_assignment_import(
    app: &AppHandle,
    payload: HistoricalAssignmentImportInput,
) -> Result<HistoricalAssignmentPreview, String> {
    let conn = open_runtime_connection(app)?;
    preview_historical_assignment_import_conn(&conn, &payload)
}

pub fn apply_historical_assignment_import(
    app: &AppHandle,
    payload: HistoricalAssignmentImportInput,
    reviewer_account_id: Option<i64>,
) -> Result<HistoricalAssignmentImportReport, String> {
    let mut conn = open_runtime_connection(app)?;
    apply_historical_assignment_import_conn(&mut conn, &payload, reviewer_account_id)
}

pub fn sign_historical_borrow_request(
    app: &AppHandle,
    payload: SignHistoricalBorrowRequestInput,
) -> Result<(), String> {
    let mut conn = open_runtime_connection(app)?;
    sign_historical_borrow_request_conn(&mut conn, &payload)
}

pub const SIGNATURE_STATUS_PENDING: &str = "pending_signature";
pub const SIGNATURE_STATUS_SIGNED: &str = "signed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HistoricalAssignmentRowStatus {
    NewAsset,
    NewAssignment,
    AlreadyAssigned,
    LocationChanged,
    DifferentHolder,
    ExistingLoanMissingFromWorkbook,
    MissingAsset,
    MissingEmployee,
    Conflict,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalAssignmentItem {
    pub row_number: usize,
    pub raw_staff_id: String,
    pub raw_employee_name: String,
    pub raw_asset_code: String,
    pub location: String,
    pub status: HistoricalAssignmentRowStatus,
    pub status_reason: Option<String>,
    pub employee_id: Option<i64>,
    pub canonical_staff_id: Option<String>,
    pub canonical_employee_name: Option<String>,
    pub asset_id: Option<i64>,
    pub canonical_asset_code: Option<String>,
    pub current_holder_staff_id: Option<String>,
    pub current_holder_name: Option<String>,
    #[serde(default)]
    pub current_holder_employee_id: Option<i64>,
    #[serde(default)]
    pub current_location: Option<String>,
    #[serde(default)]
    pub current_loan_id: Option<i64>,
    #[serde(default)]
    pub suggested_action: Option<String>,
    #[serde(default)]
    pub selected_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalAssignmentPreviewSummary {
    pub total_rows: usize,
    pub total_assignments: usize,
    pub new_assets: usize,
    pub new_assignments: usize,
    pub existing_skipped: usize,
    #[serde(default)]
    pub location_changed: usize,
    #[serde(default)]
    pub different_holders: usize,
    #[serde(default)]
    pub existing_loans_missing: usize,
    pub excluded_conflicts: usize,
    pub errors: usize,
    pub can_approve_remainder: bool,
    #[serde(default)]
    pub already_assigned: usize,
    #[serde(default)]
    pub missing_assets: usize,
    #[serde(default)]
    pub missing_employees: usize,
    #[serde(default)]
    pub conflicts: usize,
    #[serde(default)]
    pub has_blocking_conflicts: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalAssignmentPreview {
    pub file_name: String,
    pub summary: HistoricalAssignmentPreviewSummary,
    pub items: Vec<HistoricalAssignmentItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalAssignmentRowOverride {
    pub row_number: usize,
    pub raw_asset_code: String,
    pub corrected_staff_id: Option<String>,
    pub corrected_asset_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalAssignmentActionDecision {
    pub row_number: usize,
    pub raw_asset_code: String,
    pub action: String, // "approve" | "update_location" | "confirm_transfer" | "quarantine" | "skip"
    pub corrected_staff_id: Option<String>,
    pub corrected_asset_code: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalAssignmentImportInput {
    pub file_path: String,
    pub sheet_name: Option<String>,
    #[serde(default)]
    pub row_overrides: Option<Vec<HistoricalAssignmentRowOverride>>,
    #[serde(default)]
    pub row_actions: Option<Vec<HistoricalAssignmentActionDecision>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalAssignmentImportReport {
    pub total_processed: usize,
    pub created_assets: usize,
    pub created_assignments: usize,
    pub skipped_already_assigned: usize,
    #[serde(default)]
    pub updated_locations: usize,
    #[serde(default)]
    pub transferred_assignments: usize,
    pub excluded_conflicts: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignHistoricalBorrowRequestInput {
    pub request_id: i64,
    pub signature_png_blob: Option<Vec<u8>>,
    pub typed_name: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedRawRow {
    pub(crate) row_number: usize,
    pub(crate) staff_id: String,
    pub(crate) employee_name: String,
    pub(crate) cty_codes: Vec<String>,
    pub(crate) nha_codes: Vec<String>,
}

pub fn canonicalize_monitor_code(raw: &str) -> String {
    let clean = raw.trim().to_uppercase();
    if clean.starts_with("VNMON") {
        clean
    } else if let Some(suffix) = clean.strip_prefix("MON") {
        format!("VNMON{}", suffix)
    } else {
        format!("VNMON{}", clean)
    }
}

pub(crate) fn get_or_create_monitor_category(tx: &Transaction<'_>) -> Result<i64, String> {
    let existing: Option<i64> = tx
        .query_row(
            "SELECT id FROM asset_categories WHERE category_code = 'monitor' COLLATE NOCASE OR category_name = 'Monitor' COLLATE NOCASE",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|err| format!("Error querying monitor category: {err}"))?;

    if let Some(id) = existing {
        return Ok(id);
    }

    tx.execute(
        r#"
        INSERT INTO asset_categories(category_code, category_name, tracking_mode, has_computer_name, is_active, created_at, updated_at)
        VALUES('monitor', 'Monitor', 'serialized', 0, 1, datetime('now'), datetime('now'))
        "#,
        [],
    )
    .map_err(|err| format!("Error creating monitor category: {err}"))?;

    let cat_id = tx.last_insert_rowid();

    let prefix_exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM asset_category_prefixes WHERE prefix_value = 'VNMON' COLLATE NOCASE)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);

    if !prefix_exists {
        let _ = tx.execute(
            r#"
            INSERT INTO asset_category_prefixes(category_id, prefix_value, is_primary, is_active, created_at, updated_at)
            VALUES(?, 'VNMON', 1, 1, datetime('now'), datetime('now'))
            "#,
            params![cat_id],
        );
    }

    Ok(cat_id)
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::String(s) => s.trim().to_string(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                (*f as i64).to_string()
            } else {
                f.to_string()
            }
        }
        Data::DateTime(dt) => dt.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::Error(_) | Data::Empty => String::new(),
        _ => String::new(),
    }
}

fn split_monitor_codes(cell_str: &str) -> Vec<String> {
    cell_str
        .split([',', ';', '/', '\n', '\r'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn name_matches(db_name: &str, raw_name: &str) -> bool {
    let raw_clean = raw_name.trim();
    if raw_clean.is_empty() {
        return true;
    }
    let norm_db = normalize_header_token(db_name);
    let norm_raw = normalize_header_token(raw_clean);
    if norm_db == norm_raw {
        return true;
    }
    // Check token subset (e.g., "Alan Magill" in "Alan Jeffrey Magill")
    let db_tokens: Vec<&str> = db_name.split_whitespace().collect();
    let raw_tokens: Vec<&str> = raw_clean.split_whitespace().collect();
    let db_norm_tokens: Vec<String> = db_tokens.iter().map(|s| normalize_header_token(s)).filter(|s| !s.is_empty()).collect();
    let raw_norm_tokens: Vec<String> = raw_tokens.iter().map(|s| normalize_header_token(s)).filter(|s| !s.is_empty()).collect();

    if raw_norm_tokens.iter().all(|t| db_norm_tokens.contains(t)) {
        return true;
    }
    if db_norm_tokens.iter().all(|t| raw_norm_tokens.contains(t)) {
        return true;
    }
    false
}

fn normalize_header_token(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut out = String::new();
    for c in lower.chars() {
        match c {
            'a' | 'à' | 'á' | 'ả' | 'ã' | 'ạ' | 'ă' | 'ằ' | 'ắ' | 'ẳ' | 'ẵ' | 'ặ' | 'â' | 'ầ' | 'ấ' | 'ẩ' | 'ẫ' | 'ậ' => out.push('a'),
            'e' | 'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' => out.push('e'),
            'i' | 'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' => out.push('i'),
            'o' | 'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ờ' | 'ớ' | 'ở' | 'ỡ' | 'ợ' => out.push('o'),
            'u' | 'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' => out.push('u'),
            'y' | 'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' => out.push('y'),
            'd' | 'đ' => out.push('d'),
            c if c.is_alphanumeric() => out.push(c),
            _ => {}
        }
    }
    out
}

pub fn normalize_location(loc: &str) -> &'static str {
    let lower = loc.trim().to_lowercase();
    if lower.contains("nhà") || lower.contains("nha") || lower.contains("home") {
        "home"
    } else if lower.contains("cty") || lower.contains("công ty") || lower.contains("cong ty") || lower.contains("office") {
        "office"
    } else {
        "unknown"
    }
}

pub fn parse_assignment_sheet(
    file_path: &str,
    sheet_name: Option<&str>,
) -> Result<(String, Vec<ParsedRawRow>), String> {
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
        workbook
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| "Workbook has no sheets".to_string())?
    };

    let range = workbook
        .worksheet_range(&sheet_to_read)
        .map_err(|err| format!("Failed to read sheet '{sheet_to_read}': {err}"))?;

    let mut staff_id_col = None;
    let mut name_col = None;
    let mut cty_col = None;
    let mut nha_col = None;
    let mut header_row_idx = None;

    // Detect header row
    for (r_idx, row) in range.rows().enumerate() {
        for (c_idx, cell) in row.iter().enumerate() {
            let val = normalize_header_token(&cell_to_string(cell));
            if val.contains("staffid") || val.contains("manhanvien") || val.contains("manv") || val.contains("eeid") {
                staff_id_col = Some(c_idx);
            } else if val.contains("nhanvien") || val.contains("hoten") || val == "name" || val == "fullname" || val == "ten" {
                name_col = Some(c_idx);
            } else if val.contains("cty") || val.contains("congty") || val.contains("office") {
                cty_col = Some(c_idx);
            } else if val.contains("nha") || val.contains("home") {
                nha_col = Some(c_idx);
            }
        }
        if staff_id_col.is_some() && (cty_col.is_some() || nha_col.is_some()) {
            header_row_idx = Some(r_idx);
            break;
        }
    }

    let header_idx = header_row_idx
        .ok_or_else(|| "Could not identify header row (required: Staff ID, Tại CTY / Tại NHÀ)".to_string())?;
    let staff_col = staff_id_col
        .ok_or_else(|| "Could not find 'Staff ID' column in sheet".to_string())?;

    let mut parsed_rows = Vec::new();

    for (r_idx, row) in range.rows().enumerate().skip(header_idx + 1) {
        let staff_id = row
            .get(staff_col)
            .map(cell_to_string)
            .unwrap_or_default()
            .trim()
            .to_string();

        let employee_name = name_col
            .and_then(|col| row.get(col))
            .map(cell_to_string)
            .unwrap_or_default()
            .trim()
            .to_string();

        let cty_str = cty_col
            .and_then(|col| row.get(col))
            .map(cell_to_string)
            .unwrap_or_default();

        let nha_str = nha_col
            .and_then(|col| row.get(col))
            .map(cell_to_string)
            .unwrap_or_default();

        let cty_codes = split_monitor_codes(&cty_str);
        let nha_codes = split_monitor_codes(&nha_str);

        if staff_id.is_empty() && employee_name.is_empty() && cty_codes.is_empty() && nha_codes.is_empty() {
            continue;
        }

        parsed_rows.push(ParsedRawRow {
            row_number: r_idx + 1, // 1-based row number
            staff_id,
            employee_name,
            cty_codes,
            nha_codes,
        });
    }

    Ok((file_name, parsed_rows))
}

pub fn preview_historical_assignment_import_conn(
    conn: &Connection,
    payload: &HistoricalAssignmentImportInput,
) -> Result<HistoricalAssignmentPreview, String> {
    let (file_name, raw_rows) = parse_assignment_sheet(&payload.file_path, payload.sheet_name.as_deref())?;

    let mut items = Vec::new();

    // Map overrides by (row_number, canonical_asset_code)
    let mut overrides: HashMap<(usize, String), (Option<String>, Option<String>)> = HashMap::new();
    if let Some(ovs) = &payload.row_overrides {
        for ov in ovs {
            let canon = canonicalize_monitor_code(&ov.raw_asset_code);
            overrides.insert((ov.row_number, canon.to_uppercase()), (ov.corrected_staff_id.clone(), ov.corrected_asset_code.clone()));
        }
    }

    // Map to check intra-file duplicate asset assignment across different staff IDs
    let mut file_asset_claimants: HashMap<String, (String, usize)> = HashMap::new();

    for row in &raw_rows {
        // Collect assignments for this row
        let mut row_assignments: Vec<(String, &str)> = Vec::new();
        for code in &row.cty_codes {
            row_assignments.push((code.clone(), "Tại công ty"));
        }
        for code in &row.nha_codes {
            row_assignments.push((code.clone(), "Tại nhà"));
        }

        for (asset_code, location) in row_assignments {
            let initial_canon = canonicalize_monitor_code(&asset_code);
            let (ov_staff, ov_asset) = overrides
                .get(&(row.row_number, initial_canon.to_uppercase()))
                .cloned()
                .unwrap_or((None, None));

            let effective_staff_id = ov_staff.unwrap_or_else(|| row.staff_id.clone());
            let effective_asset_code = ov_asset.unwrap_or_else(|| asset_code.clone());
            let canonical_code = canonicalize_monitor_code(&effective_asset_code);

            // 1. Resolve Employee by canonical Staff ID
            let emp_info: Option<(i64, String, String)> = conn
                .query_row(
                    "SELECT id, employee_id, full_name FROM employees WHERE UPPER(TRIM(employee_id)) = UPPER(TRIM(?))",
                    params![effective_staff_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
                .map_err(|err| format!("Error querying employee for '{effective_staff_id}': {err}"))?;

            // 2. Resolve Asset by canonical Asset Code
            let asset_info: Option<(i64, String)> = {
                let exact_canonical = conn
                    .query_row(
                        "SELECT id, asset_code FROM assets WHERE asset_code = ? COLLATE NOCASE",
                        params![canonical_code],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()
                    .map_err(|err| format!("Error querying asset '{canonical_code}': {err}"))?;

                if exact_canonical.is_some() {
                    exact_canonical
                } else {
                    conn.query_row(
                        "SELECT id, asset_code FROM assets WHERE asset_code = ? COLLATE NOCASE",
                        params![effective_asset_code],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()
                    .map_err(|err| format!("Error querying asset '{effective_asset_code}': {err}"))?
                }
            };

            let mut item = HistoricalAssignmentItem {
                row_number: row.row_number,
                raw_staff_id: effective_staff_id.clone(),
                raw_employee_name: row.employee_name.clone(),
                raw_asset_code: effective_asset_code.clone(),
                location: location.to_string(),
                status: HistoricalAssignmentRowStatus::NewAssignment,
                status_reason: None,
                employee_id: emp_info.as_ref().map(|e| e.0),
                canonical_staff_id: emp_info.as_ref().map(|e| e.1.clone()),
                canonical_employee_name: emp_info.as_ref().map(|e| e.2.clone()),
                asset_id: asset_info.as_ref().map(|a| a.0),
                canonical_asset_code: asset_info.as_ref().map(|a| a.1.clone()).or_else(|| Some(canonical_code.clone())),
                current_holder_staff_id: None,
                current_holder_name: None,
                current_holder_employee_id: None,
                current_location: None,
                current_loan_id: None,
                suggested_action: Some("approve".to_string()),
                selected_action: Some("approve".to_string()),
            };

            // Check Employee validity and identity match
            match &emp_info {
                None => {
                    item.status = HistoricalAssignmentRowStatus::MissingEmployee;
                    item.status_reason = Some(format!(
                        "Employee Staff ID '{}' not found in database",
                        effective_staff_id
                    ));
                    item.suggested_action = Some("quarantine".to_string());
                }
                Some((_, _, db_full_name)) => {
                    if !name_matches(db_full_name, &row.employee_name) {
                        item.status = HistoricalAssignmentRowStatus::Conflict;
                        item.status_reason = Some(format!(
                            "Staff ID '{}' belongs to '{}' in database, but row lists '{}'",
                            effective_staff_id, db_full_name, row.employee_name
                        ));
                        item.suggested_action = Some("quarantine".to_string());
                    }
                }
            }

            // Check intra-file duplicate asset claim
            let canon_key = canonical_code.to_uppercase();
            if let Some((prior_staff_id, prior_row)) = file_asset_claimants.get(&canon_key) {
                if prior_staff_id.to_uppercase() != effective_staff_id.to_uppercase() {
                    item.status = HistoricalAssignmentRowStatus::Conflict;
                    item.status_reason = Some(format!(
                        "Asset '{}' claimed by multiple employees in file (row {} and row {})",
                        canonical_code, prior_row, row.row_number
                    ));
                    item.suggested_action = Some("quarantine".to_string());
                }
            } else {
                file_asset_claimants.insert(canon_key, (effective_staff_id.clone(), row.row_number));
            }

            // Check asset status if employee did not already have a blocking error/conflict
            match &asset_info {
                None => {
                    if item.status == HistoricalAssignmentRowStatus::NewAssignment {
                        item.status = HistoricalAssignmentRowStatus::NewAsset;
                        item.status_reason = Some(format!(
                            "Asset '{}' will be auto-created as Monitor on approval",
                            canonical_code
                        ));
                        item.suggested_action = Some("approve".to_string());
                    }
                }
                Some((asset_id, db_asset_code)) => {
                    // Check active loan in asset_loans
                    let active_loan: Option<(i64, String, String, Option<String>, i64)> = conn
                        .query_row(
                            r#"
                            SELECT al.employee_id_fk, e.employee_id, e.full_name, a.usage_location, al.id
                            FROM asset_loans al
                            INNER JOIN employees e ON e.id = al.employee_id_fk
                            INNER JOIN assets a ON a.id = al.asset_id
                            WHERE al.asset_id = ? AND al.returned_at IS NULL
                            "#,
                            params![asset_id],
                            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                        )
                        .optional()
                        .map_err(|err| format!("Error checking active loan for asset '{db_asset_code}': {err}"))?;

                    if let Some((holder_emp_fk, holder_staff_id, holder_full_name, db_loc, loan_id)) = active_loan {
                        item.current_holder_staff_id = Some(holder_staff_id.clone());
                        item.current_holder_name = Some(holder_full_name.clone());
                        item.current_holder_employee_id = Some(holder_emp_fk);
                        item.current_location = db_loc.clone();
                        item.current_loan_id = Some(loan_id);

                        if let Some((target_emp_id, _, _)) = &emp_info {
                            if *target_emp_id == holder_emp_fk {
                                if item.status == HistoricalAssignmentRowStatus::NewAssignment {
                                    let db_norm = normalize_location(db_loc.as_deref().unwrap_or(""));
                                    let item_norm = normalize_location(&item.location);
                                    if db_norm == item_norm && !db_loc.as_deref().unwrap_or("").is_empty() {
                                        item.status = HistoricalAssignmentRowStatus::AlreadyAssigned;
                                        item.status_reason = Some(format!(
                                            "Asset '{}' is already assigned to this employee at the same location ({})",
                                            db_asset_code, item.location
                                        ));
                                        item.suggested_action = Some("skip".to_string());
                                    } else {
                                        item.status = HistoricalAssignmentRowStatus::LocationChanged;
                                        item.status_reason = Some(format!(
                                            "Asset '{}' is already assigned to this employee, but location changed from '{}' to '{}'",
                                            db_asset_code,
                                            db_loc.as_deref().unwrap_or("Unspecified"),
                                            item.location
                                        ));
                                        item.suggested_action = Some("update_location".to_string());
                                    }
                                }
                            } else {
                                item.status = HistoricalAssignmentRowStatus::DifferentHolder;
                                item.status_reason = Some(format!(
                                    "Asset '{}' is currently assigned in DB to '{} ({})', but workbook assigns to '{} ({})'",
                                    db_asset_code, holder_full_name, holder_staff_id, row.employee_name, effective_staff_id
                                ));
                                item.suggested_action = Some("quarantine".to_string());
                            }
                        }
                    }
                }
            }

            items.push(item);
        }
    }

    // Query active monitor loans in DB to detect any missing from the workbook
    let mut workbook_canonical_codes = std::collections::HashSet::new();
    for item in &items {
        if let Some(canon) = &item.canonical_asset_code {
            workbook_canonical_codes.insert(canon.to_uppercase());
        }
    }

    let mut stmt_missing = conn
        .prepare(
            r#"
            SELECT al.id, a.id, a.asset_code, a.usage_location, e.id, e.employee_id, e.full_name
            FROM asset_loans al
            JOIN assets a ON a.id = al.asset_id
            JOIN employees e ON e.id = al.employee_id_fk
            WHERE al.returned_at IS NULL
              AND (
                EXISTS (
                  SELECT 1 FROM asset_categories ac
                  WHERE ac.id = a.category_id
                    AND (ac.category_code = 'monitor' COLLATE NOCASE OR ac.category_name = 'Monitor' COLLATE NOCASE)
                )
                OR a.asset_type = 'Monitor' COLLATE NOCASE
                OR a.asset_code LIKE 'VNMON%'
                OR a.asset_code LIKE 'MON%'
              )
            "#,
        )
        .map_err(|err| format!("Error preparing active monitor query: {err}"))?;

    let missing_loans = stmt_missing
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
            ))
        })
        .map_err(|err| format!("Error querying active monitor loans: {err}"))?;

    for loan_res in missing_loans {
        let (loan_id, asset_id, asset_code, usage_loc, emp_fk, staff_id, full_name) =
            loan_res.map_err(|e| e.to_string())?;
        let canon = canonicalize_monitor_code(&asset_code);
        if !workbook_canonical_codes.contains(&canon.to_uppercase()) {
            items.push(HistoricalAssignmentItem {
                row_number: 0,
                raw_staff_id: staff_id.clone(),
                raw_employee_name: full_name.clone(),
                raw_asset_code: asset_code.clone(),
                location: usage_loc.clone().unwrap_or_else(|| "Tại công ty".to_string()),
                status: HistoricalAssignmentRowStatus::ExistingLoanMissingFromWorkbook,
                status_reason: Some(format!(
                    "Active monitor loan for asset '{}' held by '{} ({})' is not present in verified workbook",
                    asset_code, full_name, staff_id
                )),
                employee_id: Some(emp_fk),
                canonical_staff_id: Some(staff_id.clone()),
                canonical_employee_name: Some(full_name.clone()),
                asset_id: Some(asset_id),
                canonical_asset_code: Some(canon),
                current_holder_staff_id: Some(staff_id),
                current_holder_name: Some(full_name),
                current_holder_employee_id: Some(emp_fk),
                current_location: usage_loc,
                current_loan_id: Some(loan_id),
                suggested_action: Some("review".to_string()),
                selected_action: Some("review".to_string()),
            });
        }
    }

    let total_rows = raw_rows.len();
    let total_assignments = items.iter().filter(|i| i.row_number > 0).count();
    let mut new_assets = 0;
    let mut new_assignments = 0;
    let mut existing_skipped = 0;
    let mut location_changed = 0;
    let mut different_holders = 0;
    let mut existing_loans_missing = 0;
    let mut excluded_conflicts = 0;
    let mut errors = 0;
    let mut missing_employees = 0;
    let mut missing_assets = 0;

    for item in &items {
        match item.status {
            HistoricalAssignmentRowStatus::NewAsset => new_assets += 1,
            HistoricalAssignmentRowStatus::NewAssignment => new_assignments += 1,
            HistoricalAssignmentRowStatus::AlreadyAssigned => existing_skipped += 1,
            HistoricalAssignmentRowStatus::LocationChanged => location_changed += 1,
            HistoricalAssignmentRowStatus::DifferentHolder => different_holders += 1,
            HistoricalAssignmentRowStatus::ExistingLoanMissingFromWorkbook => existing_loans_missing += 1,
            HistoricalAssignmentRowStatus::Conflict => excluded_conflicts += 1,
            HistoricalAssignmentRowStatus::MissingEmployee => {
                errors += 1;
                missing_employees += 1;
            }
            HistoricalAssignmentRowStatus::MissingAsset => {
                errors += 1;
                missing_assets += 1;
            }
            HistoricalAssignmentRowStatus::Error => {
                errors += 1;
            }
        }
    }

    let can_approve_remainder = (new_assets + new_assignments + location_changed) > 0;
    let total_conflicts_and_diff = excluded_conflicts + different_holders;
    let summary = HistoricalAssignmentPreviewSummary {
        total_rows,
        total_assignments,
        new_assets,
        new_assignments,
        existing_skipped,
        location_changed,
        different_holders,
        existing_loans_missing,
        excluded_conflicts: total_conflicts_and_diff,
        errors,
        can_approve_remainder,
        already_assigned: existing_skipped,
        missing_assets,
        missing_employees,
        conflicts: total_conflicts_and_diff,
        has_blocking_conflicts: total_conflicts_and_diff > 0,
    };

    Ok(HistoricalAssignmentPreview {
        file_name,
        summary,
        items,
    })
}

pub fn apply_historical_assignment_import_conn(
    conn: &mut Connection,
    payload: &HistoricalAssignmentImportInput,
    reviewer_account_id: Option<i64>,
) -> Result<HistoricalAssignmentImportReport, String> {
    let preview = preview_historical_assignment_import_conn(conn, payload)?;

    let valid_count = preview.summary.new_assets
        + preview.summary.new_assignments
        + preview.summary.location_changed
        + preview.summary.different_holders;
    if valid_count == 0 {
        let mut errors = Vec::new();
        for item in &preview.items {
            if matches!(
                item.status,
                HistoricalAssignmentRowStatus::MissingEmployee
                    | HistoricalAssignmentRowStatus::Error
                    | HistoricalAssignmentRowStatus::MissingAsset
            ) {
                errors.push(format!(
                    "Excluded row {} (Staff ID: '{}', Asset: '{}'): {}",
                    item.row_number,
                    item.raw_staff_id,
                    item.raw_asset_code,
                    item.status_reason.as_deref().unwrap_or("Validation error")
                ));
            }
        }
        return Ok(HistoricalAssignmentImportReport {
            total_processed: preview.items.len(),
            created_assets: 0,
            created_assignments: 0,
            skipped_already_assigned: preview.summary.existing_skipped,
            updated_locations: 0,
            transferred_assignments: 0,
            excluded_conflicts: preview.summary.excluded_conflicts,
            errors,
        });
    }

    let tx = conn
        .transaction()
        .map_err(|err| format!("Failed to begin transaction: {err}"))?;

    let cat_id = get_or_create_monitor_category(&tx)?;

    let mut created_assets = 0;
    let mut created_assignments = 0;
    let mut skipped_already_assigned = 0;
    let mut updated_locations = 0;
    let mut transferred_assignments = 0;
    let mut excluded_conflicts = 0;
    let mut errors = Vec::new();

    let mut decisions: HashMap<(usize, String), String> = HashMap::new();
    if let Some(actions) = &payload.row_actions {
        for act in actions {
            let canon = canonicalize_monitor_code(&act.raw_asset_code);
            decisions.insert((act.row_number, canon.to_uppercase()), act.action.to_lowercase());
        }
    }

    for item in &preview.items {
        match item.status {
            HistoricalAssignmentRowStatus::AlreadyAssigned => {
                skipped_already_assigned += 1;
            }
            HistoricalAssignmentRowStatus::ExistingLoanMissingFromWorkbook => {
                // Informational only: do not automatically return or transfer
            }
            HistoricalAssignmentRowStatus::LocationChanged => {
                let canonical_code = item.canonical_asset_code.as_deref().unwrap_or(&item.raw_asset_code);
                let decision = decisions
                    .get(&(item.row_number, canonical_code.to_uppercase()))
                    .map(|s| s.as_str())
                    .unwrap_or("update_location");

                if decision == "update_location" || decision == "approve" {
                    let asset_id = item.asset_id.ok_or_else(|| {
                        format!("Missing asset ID for location update on row {}", item.row_number)
                    })?;
                    let employee_row_id = item.employee_id.ok_or_else(|| {
                        format!("Missing employee ID for row {}", item.row_number)
                    })?;
                    let staff_id = item.canonical_staff_id.as_deref().unwrap_or(&item.raw_staff_id);

                    tx.execute(
                        "UPDATE assets SET usage_location = ?, updated_at = datetime('now') WHERE id = ?",
                        params![item.location, asset_id],
                    )
                    .map_err(|err| format!("Failed to update location for '{canonical_code}': {err}"))?;

                    let audit_payload = json!({
                        "action": "historical_assignment_reconciliation.update_location",
                        "assetId": asset_id,
                        "assetCode": canonical_code,
                        "employeeId": employee_row_id,
                        "staffId": staff_id,
                        "oldLocation": item.current_location,
                        "newLocation": item.location,
                    })
                    .to_string();

                    audit::insert_audit_log_tx(
                        &tx,
                        "historical_assignment_reconciliation.update_location",
                        "account",
                        reviewer_account_id.map(|id| id.to_string()).as_deref(),
                        "assets",
                        &asset_id.to_string(),
                        Some(&audit_payload),
                    )?;
                    updated_locations += 1;
                } else {
                    skipped_already_assigned += 1;
                }
            }
            HistoricalAssignmentRowStatus::DifferentHolder => {
                let canonical_code = item.canonical_asset_code.as_deref().unwrap_or(&item.raw_asset_code);
                let decision = decisions
                    .get(&(item.row_number, canonical_code.to_uppercase()))
                    .map(|s| s.as_str())
                    .unwrap_or("quarantine");

                if decision == "confirm_transfer" {
                    let asset_id = item.asset_id.ok_or_else(|| {
                        format!("Missing asset ID for transfer on row {}", item.row_number)
                    })?;
                    let employee_row_id = item.employee_id.ok_or_else(|| {
                        format!("Missing employee ID for transfer on row {}", item.row_number)
                    })?;
                    let staff_id = item.canonical_staff_id.as_deref().unwrap_or(&item.raw_staff_id);
                    let full_name = item.canonical_employee_name.as_deref().unwrap_or(&item.raw_employee_name);

                    // 1. Close old active loan
                    tx.execute(
                        r#"
                        UPDATE asset_loans
                        SET returned_at = datetime('now')
                        WHERE asset_id = ? AND returned_at IS NULL
                        "#,
                        params![asset_id],
                    )
                    .map_err(|err| format!("Failed to close active loan for transfer on '{canonical_code}': {err}"))?;

                    // 2. Update asset status & location
                    tx.execute(
                        "UPDATE assets SET status = 'assigned', usage_location = ?, updated_at = datetime('now') WHERE id = ?",
                        params![item.location, asset_id],
                    )
                    .map_err(|err| format!("Failed to update asset location on '{canonical_code}': {err}"))?;

                    // 3. Create new loan for verified holder
                    apply_single_historical_assignment_tx(
                        &tx,
                        asset_id,
                        canonical_code,
                        employee_row_id,
                        staff_id,
                        full_name,
                        &item.location,
                        reviewer_account_id,
                    )?;

                    let audit_payload = json!({
                        "action": "historical_assignment_reconciliation.transfer",
                        "assetId": asset_id,
                        "assetCode": canonical_code,
                        "previousHolderStaffId": item.current_holder_staff_id,
                        "newHolderStaffId": staff_id,
                        "newLocation": item.location,
                    })
                    .to_string();

                    audit::insert_audit_log_tx(
                        &tx,
                        "historical_assignment_reconciliation.transfer",
                        "account",
                        reviewer_account_id.map(|id| id.to_string()).as_deref(),
                        "assets",
                        &asset_id.to_string(),
                        Some(&audit_payload),
                    )?;
                    transferred_assignments += 1;
                } else {
                    excluded_conflicts += 1;
                }
            }
            HistoricalAssignmentRowStatus::Conflict => {
                excluded_conflicts += 1;
            }
            HistoricalAssignmentRowStatus::MissingEmployee
            | HistoricalAssignmentRowStatus::Error
            | HistoricalAssignmentRowStatus::MissingAsset => {
                errors.push(format!(
                    "Excluded row {} (Staff ID: '{}', Asset: '{}'): {}",
                    item.row_number,
                    item.raw_staff_id,
                    item.raw_asset_code,
                    item.status_reason.as_deref().unwrap_or("Validation error")
                ));
            }
            HistoricalAssignmentRowStatus::NewAsset => {
                let employee_row_id = item.employee_id.ok_or_else(|| {
                    format!("Missing employee ID for item row {}", item.row_number)
                })?;
                let staff_id = item.canonical_staff_id.as_deref().unwrap_or(&item.raw_staff_id);
                let full_name = item.canonical_employee_name.as_deref().unwrap_or(&item.raw_employee_name);
                let canonical_code = item.canonical_asset_code.as_deref().unwrap_or(&item.raw_asset_code);

                // Revalidate immediately before committing:
                // Check if asset_code was created concurrently
                let existing_asset: Option<i64> = tx
                    .query_row(
                        "SELECT id FROM assets WHERE asset_code = ? COLLATE NOCASE",
                        params![canonical_code],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(|err| format!("Error revalidating asset '{canonical_code}': {err}"))?;

                let asset_id = if let Some(existing_id) = existing_asset {
                    // If already exists, check if already loaned
                    let active_loan: Option<i64> = tx
                        .query_row(
                            "SELECT employee_id_fk FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
                            params![existing_id],
                            |r| r.get(0),
                        )
                        .optional()
                        .map_err(|err| format!("Error checking concurrency for asset '{canonical_code}': {err}"))?;

                    if let Some(holder_id) = active_loan {
                        if holder_id == employee_row_id {
                            skipped_already_assigned += 1;
                            continue;
                        } else {
                            return Err(format!(
                                "Concurrency error: Asset '{}' was assigned to another employee during approval",
                                canonical_code
                            ));
                        }
                    }
                    existing_id
                } else {
                    // Create the new asset
                    tx.execute(
                        r#"
                        INSERT INTO assets(
                          asset_code,
                          category_id,
                          asset_type,
                          display_name,
                          display_name_short,
                          usage_location,
                          status,
                          created_at,
                          updated_at
                        )
                        VALUES(?, ?, 'Monitor', ?, ?, ?, 'in_stock', datetime('now'), datetime('now'))
                        "#,
                        params![
                            canonical_code,
                            cat_id,
                            item.raw_asset_code,
                            item.raw_asset_code,
                            item.location,
                        ],
                    )
                    .map_err(|err| format!("Failed to create asset '{canonical_code}': {err}"))?;
                    created_assets += 1;
                    tx.last_insert_rowid()
                };

                apply_single_historical_assignment_tx(
                    &tx,
                    asset_id,
                    canonical_code,
                    employee_row_id,
                    staff_id,
                    full_name,
                    &item.location,
                    reviewer_account_id,
                )?;
                created_assignments += 1;
            }
            HistoricalAssignmentRowStatus::NewAssignment => {
                let employee_row_id = item.employee_id.ok_or_else(|| {
                    format!("Missing employee ID for item row {}", item.row_number)
                })?;
                let staff_id = item.canonical_staff_id.as_deref().unwrap_or(&item.raw_staff_id);
                let full_name = item.canonical_employee_name.as_deref().unwrap_or(&item.raw_employee_name);
                let asset_id = item.asset_id.ok_or_else(|| {
                    format!("Missing asset ID for valid item row {}", item.row_number)
                })?;
                let canonical_code = item.canonical_asset_code.as_deref().unwrap_or(&item.raw_asset_code);

                // Revalidate active loan
                let active_loan: Option<i64> = tx
                    .query_row(
                        "SELECT employee_id_fk FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
                        params![asset_id],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(|err| format!("Error checking concurrency for asset '{canonical_code}': {err}"))?;

                if let Some(holder_id) = active_loan {
                    if holder_id == employee_row_id {
                        skipped_already_assigned += 1;
                        continue;
                    } else {
                        return Err(format!(
                            "Concurrency error: Asset '{}' was assigned to another employee during approval",
                            canonical_code
                        ));
                    }
                }

                apply_single_historical_assignment_tx(
                    &tx,
                    asset_id,
                    canonical_code,
                    employee_row_id,
                    staff_id,
                    full_name,
                    &item.location,
                    reviewer_account_id,
                )?;
                created_assignments += 1;
            }
        }
    }

    tx.commit()
        .map_err(|err| format!("Failed to commit historical assignment import transaction: {err}"))?;

    Ok(HistoricalAssignmentImportReport {
        total_processed: preview.items.len(),
        created_assets,
        created_assignments,
        skipped_already_assigned,
        updated_locations,
        transferred_assignments,
        excluded_conflicts,
        errors,
    })
}

fn apply_single_historical_assignment_tx(
    tx: &Transaction<'_>,
    asset_id: i64,
    asset_code: &str,
    employee_row_id: i64,
    staff_id: &str,
    full_name: &str,
    usage_location: &str,
    reviewer_account_id: Option<i64>,
) -> Result<(), String> {
    // Validate reviewer_account_id exists to satisfy foreign key constraint
    let valid_reviewer_id = if let Some(acc_id) = reviewer_account_id {
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM app_local_accounts WHERE id = ?)",
                params![acc_id],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if exists {
            Some(acc_id)
        } else {
            None
        }
    } else {
        None
    };

    // Generate unique request_key
    let request_key = format!("HAIR-{}", generate_random_suffix());

    tx.execute(
        r#"
        INSERT INTO borrow_requests(
          request_key,
          employee_id_fk,
          submitted_employee_id,
          submitted_full_name,
          status,
          request_type,
          signature_status,
          submit_source_ip,
          decision_note,
          decided_by_account_id,
          submitted_at,
          decided_at,
          imported_at,
          handover_date
        )
        VALUES(?, ?, ?, ?, 'approved', 'borrow', ?, NULL, 'Historical assignment import (verified inventory)', ?, datetime('now'), datetime('now'), datetime('now'), NULL)
        "#,
        params![
            request_key,
            employee_row_id,
            staff_id,
            full_name,
            SIGNATURE_STATUS_PENDING,
            valid_reviewer_id
        ],
    )
    .map_err(|err| format!("Failed to create borrow_request for asset '{asset_code}': {err}"))?;

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
    .map_err(|err| format!("Failed to create borrow_request_items for asset '{asset_code}': {err}"))?;

    // Update asset status to 'assigned' and update usage_location
    tx.execute(
        r#"
        UPDATE assets
        SET status = 'assigned',
            usage_location = ?
        WHERE id = ?
        "#,
        params![usage_location, asset_id],
    )
    .map_err(|err| format!("Failed to update asset status/location for '{asset_code}': {err}"))?;

    // Insert active loan
    tx.execute(
        r#"
        INSERT INTO asset_loans(
          asset_id,
          employee_id_fk,
          borrow_request_id,
          approved_by_account_id,
          borrowed_at,
          imported_at,
          handover_date
        )
        VALUES(?, ?, ?, ?, datetime('now'), datetime('now'), NULL)
        "#,
        params![asset_id, employee_row_id, request_id, valid_reviewer_id],
    )
    .map_err(|err| format!("Failed to create asset_loan for '{asset_code}': {err}"))?;

    let loan_id = tx.last_insert_rowid();

    let audit_payload = json!({
        "action": "historical_assignment_import",
        "assetId": asset_id,
        "assetCode": asset_code,
        "employeeId": employee_row_id,
        "staffId": staff_id,
        "location": usage_location,
        "signatureStatus": SIGNATURE_STATUS_PENDING,
    })
    .to_string();

    audit::insert_audit_log_tx(
        tx,
        "historical_assignment_import.apply",
        "account",
        reviewer_account_id.map(|id| id.to_string()).as_deref(),
        "asset_loans",
        &loan_id.to_string(),
        Some(&audit_payload),
    )?;

    Ok(())
}

pub fn sign_historical_borrow_request_conn(
    conn: &mut Connection,
    payload: &SignHistoricalBorrowRequestInput,
) -> Result<(), String> {
    if payload.signature_png_blob.is_none()
        && payload.typed_name.as_deref().map(str::trim).unwrap_or("").is_empty()
    {
        return Err("A valid signature image or typed name is required for genuine signing".to_string());
    }

    let tx = conn
        .transaction()
        .map_err(|err| format!("Failed to begin transaction: {err}"))?;

    let (_current_status, current_sig_status, req_key): (String, String, String) = tx
        .query_row(
            "SELECT status, COALESCE(signature_status, 'signed'), request_key FROM borrow_requests WHERE id = ?",
            params![payload.request_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|err| format!("Error querying borrow request {}: {err}", payload.request_id))?
        .ok_or_else(|| format!("Borrow request with ID {} not found", payload.request_id))?;

    if current_sig_status == SIGNATURE_STATUS_SIGNED {
        return Err("Borrow request is already signed".to_string());
    }

    let method = match (
        payload.signature_png_blob.is_some(),
        payload.typed_name.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_some(),
    ) {
        (true, true) => "both",
        (true, false) => "signature",
        (false, true) => "typed_name",
        (false, false) => return Err("Genuine signing requires signature or typed name".to_string()),
    };

    tx.execute(
        r#"
        INSERT INTO borrow_request_confirmations(
          borrow_request_id,
          policy_acknowledged,
          asset_codes_snapshot_json,
          confirmation_method,
          signature_png_blob,
          typed_name,
          confirmed_at
        )
        VALUES(?, 1, '[]', ?, ?, ?, datetime('now'))
        "#,
        params![
            payload.request_id,
            method,
            payload.signature_png_blob,
            payload.typed_name.as_deref().map(str::trim),
        ],
    )
    .map_err(|err| format!("Failed to save borrow confirmation: {err}"))?;

    tx.execute(
        "UPDATE borrow_requests SET signature_status = ? WHERE id = ?",
        params![SIGNATURE_STATUS_SIGNED, payload.request_id],
    )
    .map_err(|err| format!("Failed to update signature status: {err}"))?;

    let audit_payload = json!({
        "action": "genuine_signing",
        "requestId": payload.request_id,
        "requestKey": req_key,
        "method": method,
    })
    .to_string();

    audit::insert_audit_log_tx(
        &tx,
        "borrow_request.sign",
        "system",
        None,
        "borrow_requests",
        &payload.request_id.to_string(),
        Some(&audit_payload),
    )?;

    tx.commit()
        .map_err(|err| format!("Failed to commit signing transaction: {err}"))?;

    Ok(())
}

fn generate_random_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use crate::db::{apply_migrations, configure_connection};

    fn open_test_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open memory db");
        configure_connection(&conn).expect("configure pragmas");
        apply_migrations(&conn).expect("apply migrations");
        conn
    }

    #[test]
    fn test_revalidate_against_isolated_db() {
        use crate::db::open_encrypted_read_only_connection;

        let db_path = if Path::new("scratch/isolated_test.sqlite3").exists() {
            "scratch/isolated_test.sqlite3"
        } else if Path::new("../scratch/isolated_test.sqlite3").exists() {
            "../scratch/isolated_test.sqlite3"
        } else {
            return;
        };

        let file_path = if Path::new("00_ExSource/Asigned_Asset.xlsx").exists() {
            "00_ExSource/Asigned_Asset.xlsx"
        } else if Path::new("../00_ExSource/Asigned_Asset.xlsx").exists() {
            "../00_ExSource/Asigned_Asset.xlsx"
        } else {
            return;
        };

        let conn = open_encrypted_read_only_connection(Path::new(db_path)).expect("open isolated db");
        let preview = preview_historical_assignment_import_conn(
            &conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                ..Default::default()
            },
        ).expect("preview");

        // Verify summary
        assert_eq!(preview.summary.total_rows, 157);
        assert_eq!(preview.summary.total_assignments, 200);
        assert_eq!(preview.summary.new_assets, 193);
        assert_eq!(preview.summary.conflicts, 5);
        assert_eq!(preview.summary.missing_employees, 2);
        assert!(preview.summary.has_blocking_conflicts);
        assert!(preview.summary.can_approve_remainder);

        // Verify duplicate full name 'Lê Thanh Đạt' with different Staff IDs are separate employees, NOT conflicts
        let item405 = preview.items.iter().find(|i| i.raw_staff_id == "ASWVN405").expect("ASWVN405");
        assert_eq!(item405.status, HistoricalAssignmentRowStatus::NewAsset);
        assert_eq!(item405.employee_id, Some(39));
        assert_eq!(item405.canonical_staff_id.as_deref(), Some("ASWVN405"));

        let item1266 = preview.items.iter().find(|i| i.raw_staff_id == "ASWVN1266").expect("ASWVN1266");
        assert_eq!(item1266.status, HistoricalAssignmentRowStatus::NewAsset);
        assert_eq!(item1266.employee_id, Some(366));
        assert_eq!(item1266.canonical_staff_id.as_deref(), Some("ASWVN1266"));
    }

    #[test]
    fn test_parse_exact_asigned_asset_workbook() {
        let file_path = if Path::new("00_ExSource/Asigned_Asset.xlsx").exists() {
            "00_ExSource/Asigned_Asset.xlsx"
        } else if Path::new("../00_ExSource/Asigned_Asset.xlsx").exists() {
            "../00_ExSource/Asigned_Asset.xlsx"
        } else {
            return;
        };

        let (file_name, rows) = parse_assignment_sheet(file_path, None).expect("parse sheet");
        assert_eq!(file_name, "Asigned_Asset.xlsx");
        assert_eq!(rows.len(), 157, "Should have 157 data rows");

        let mut cty_total = 0;
        let mut nha_total = 0;
        let mut unique_codes = HashSet::new();
        for r in &rows {
            cty_total += r.cty_codes.len();
            nha_total += r.nha_codes.len();
            for c in &r.cty_codes {
                unique_codes.insert(c.clone());
            }
            for c in &r.nha_codes {
                unique_codes.insert(c.clone());
            }
        }
        let total_assignments = cty_total + nha_total;
        assert_eq!(cty_total, 141);
        assert_eq!(nha_total, 59);
        assert_eq!(total_assignments, 200);
        assert_eq!(unique_codes.len(), 200);
    }

    #[test]
    fn test_identity_conflicts_detected_and_quarantined() {
        let mut conn = open_test_conn();

        // Seed employee ASWVN214 as Trần Nhật Phong
        conn.execute(
            "INSERT INTO employees(employee_id, full_name, email, is_floating) VALUES('ASWVN214', 'Trần Nhật Phong', 'p.tran@eml.com.au', 0)",
            [],
        ).unwrap();

        // Seed monitor VNMON770 in stock
        conn.execute(
            "INSERT INTO assets(asset_code, asset_type, display_name, status) VALUES('VNMON770', 'Monitor', 'Dell Monitor', 'in_stock')",
            [],
        ).unwrap();

        let file_path = "00_ExSource/Asigned_Asset.xlsx";
        if !Path::new(file_path).exists() {
            return;
        }

        let preview = preview_historical_assignment_import_conn(
            &conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                ..Default::default()
            },
        ).unwrap();

        assert!(preview.summary.has_blocking_conflicts);
        assert!(preview.summary.conflicts >= 2, "Must detect ASWVN214 and ASWVN1261 mismatches");

        // Find the ASWVN214 Lê Minh Duy items
        let conf7 = preview.items.iter().find(|i| i.raw_staff_id == "ASWVN214" && i.raw_employee_name == "LÊ MINH DUY");
        assert!(conf7.is_some());
        let item = conf7.unwrap();
        assert_eq!(item.status, HistoricalAssignmentRowStatus::Conflict);
        assert!(item.status_reason.as_ref().unwrap().contains("belongs to 'Trần Nhật Phong'"));

        // Per D2, conflicting items are quarantined and never assigned
        let apply_report = apply_historical_assignment_import_conn(
            &mut conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                ..Default::default()
            },
            None,
        ).unwrap();

        assert!(apply_report.excluded_conflicts >= 2, "Conflicted rows must be excluded");
        // Verify NO loan was created for Trần Nhật Phong (ASWVN214)
        let conf_loans: i64 = conn.query_row(
            "SELECT count(*) FROM asset_loans al JOIN employees e ON e.id = al.employee_id_fk WHERE e.employee_id = 'ASWVN214'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(conf_loans, 0, "Conflicted employee ASWVN214 must have 0 loans created");
    }

    #[test]
    fn test_valid_assignments_create_loans_with_pending_signature() {
        let mut conn = open_test_conn();

        // Seed employee ASWVN1229 Trần Tấn Quý
        let emp_id: i64 = conn.query_row(
            "INSERT INTO employees(employee_id, full_name, email, is_floating) VALUES('ASWVN1229', 'TRẦN TẤN QUÝ', 'quy@example.com', 0) RETURNING id",
            [],
            |r| r.get(0),
        ).unwrap();

        // Seed 2 monitors for Quý: VNMON690 and VNMON554
        let a1: i64 = conn.query_row(
            "INSERT INTO assets(asset_code, asset_type, display_name, status) VALUES('VNMON690', 'Monitor', 'Dell 24', 'in_stock') RETURNING id",
            [],
            |r| r.get(0),
        ).unwrap();
        let a2: i64 = conn.query_row(
            "INSERT INTO assets(asset_code, asset_type, display_name, status) VALUES('VNMON554', 'Monitor', 'Dell 24', 'in_stock') RETURNING id",
            [],
            |r| r.get(0),
        ).unwrap();

        // Test single assignment application
        let tx = conn.transaction().unwrap();
        apply_single_historical_assignment_tx(
            &tx,
            a1,
            "VNMON690",
            emp_id,
            "ASWVN1229",
            "TRẦN TẤN QUÝ",
            "Tại công ty",
            Some(1),
        ).unwrap();
        apply_single_historical_assignment_tx(
            &tx,
            a2,
            "VNMON554",
            emp_id,
            "ASWVN1229",
            "TRẦN TẤN QUÝ",
            "Tại công ty",
            Some(1),
        ).unwrap();
        tx.commit().unwrap();

        // Verify asset_loans
        let loan_count: i64 = conn.query_row("SELECT count(*) FROM asset_loans WHERE employee_id_fk = ?", params![emp_id], |r| r.get(0)).unwrap();
        assert_eq!(loan_count, 2);

        // Verify borrow_requests: signature_status is 'pending_signature', submit_source_ip is NULL
        let (sig_status, ip, imported_at, handover_date): (String, Option<String>, Option<String>, Option<String>) = conn.query_row(
            "SELECT signature_status, submit_source_ip, imported_at, handover_date FROM borrow_requests LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ).unwrap();
        assert_eq!(sig_status, SIGNATURE_STATUS_PENDING);
        assert!(ip.is_none(), "submit_source_ip must never be repurposed");
        assert!(imported_at.is_some(), "imported_at must record inventory import date");
        assert!(handover_date.is_none(), "handover_date must be left unknown per D1");

        // Verify zero confirmations fabricated
        let conf_count: i64 = conn.query_row("SELECT count(*) FROM borrow_request_confirmations", [], |r| r.get(0)).unwrap();
        assert_eq!(conf_count, 0, "No signatures or confirmations must be fabricated");

        // Verify assets updated to 'assigned' and usage_location set
        let loc1: String = conn.query_row("SELECT usage_location FROM assets WHERE id = ?", params![a1], |r| r.get(0)).unwrap();
        assert_eq!(loc1, "Tại công ty");

        // Verify employee assigned assets query
        let assigned = crate::db::employee::get_employee_assigned_assets_conn(&conn, emp_id).unwrap();
        assert_eq!(assigned.len(), 2);
        assert_eq!(assigned[0].signature_status.as_deref(), Some(SIGNATURE_STATUS_PENDING));
        assert_eq!(assigned[0].usage_location.as_deref(), Some("Tại công ty"));

        // Genuine signing test: sign request 1 with typed name
        let req1_id: i64 = conn.query_row("SELECT id FROM borrow_requests LIMIT 1", [], |r| r.get(0)).unwrap();
        sign_historical_borrow_request_conn(
            &mut conn,
            &SignHistoricalBorrowRequestInput {
                request_id: req1_id,
                signature_png_blob: None,
                typed_name: Some("Trần Tấn Quý".to_string()),
            },
        ).unwrap();

        // Verify signature_status is now 'signed'
        let updated_sig: String = conn.query_row("SELECT signature_status FROM borrow_requests WHERE id = ?", params![req1_id], |r| r.get(0)).unwrap();
        assert_eq!(updated_sig, SIGNATURE_STATUS_SIGNED);

        let assigned_after = crate::db::employee::get_employee_assigned_assets_conn(&conn, emp_id).unwrap();
        let item1 = assigned_after.iter().find(|a| a.asset_id == a1).unwrap();
        assert_eq!(item1.signature_status.as_deref(), Some(SIGNATURE_STATUS_SIGNED));
    }

    #[test]
    fn test_isolated_db_full_workflow() {
        use crate::db::open_encrypted_read_only_connection;

        let isolated_db_path = if Path::new("scratch/isolated_test.sqlite3").exists() {
            Path::new("scratch/isolated_test.sqlite3").to_path_buf()
        } else if Path::new("../scratch/isolated_test.sqlite3").exists() {
            Path::new("../scratch/isolated_test.sqlite3").to_path_buf()
        } else {
            return;
        };

        // 1. Verify Preview on isolated test DB copy without modifying it
        let read_conn = open_encrypted_read_only_connection(&isolated_db_path).expect("open read-only isolated db");
        let file_path = if Path::new("00_ExSource/Asigned_Asset.xlsx").exists() {
            "00_ExSource/Asigned_Asset.xlsx".to_string()
        } else {
            "../00_ExSource/Asigned_Asset.xlsx".to_string()
        };
        let preview = preview_historical_assignment_import_conn(
            &read_conn,
            &HistoricalAssignmentImportInput {
                file_path,
                sheet_name: None,
                ..Default::default()
            },
        ).expect("preview historical assignment import");

        assert_eq!(preview.summary.total_rows, 157);
        assert_eq!(preview.summary.total_assignments, 200);
        println!("ISOLATED SUMMARY: {:#?}", preview.summary);
        assert!(preview.summary.has_blocking_conflicts);
        for name in &["Thiện", "Tiến", "Duy", "Anh"] {
            let matches: Vec<(i64, String, String)> = {
                let mut stmt = read_conn.prepare("SELECT id, employee_id, full_name FROM employees WHERE UPPER(full_name) LIKE UPPER(?)").unwrap();
                let rows = stmt.query_map(params![format!("%{}%", name)], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
                rows.filter_map(|r| r.ok()).collect()
            };
            println!("SEARCH NAME '{}' -> matches={:?}", name, matches);
        }
        assert!(preview.summary.conflicts >= 2);

        // Check specific identity conflict rows
        let conf_duy = preview.items.iter().find(|i| i.raw_staff_id == "ASWVN214" && i.raw_employee_name == "LÊ MINH DUY");
        assert!(conf_duy.is_some());
        assert_eq!(conf_duy.unwrap().status, HistoricalAssignmentRowStatus::Conflict);

        let conf_anh = preview.items.iter().find(|i| i.raw_staff_id == "ASWVN1261" && i.raw_employee_name == "LÊ TUẤN ANH");
        assert!(conf_anh.is_some());
        assert_eq!(conf_anh.unwrap().status, HistoricalAssignmentRowStatus::Conflict);

        // 2. In memory DB seeded from isolated patterns: verify full idempotent workflow
        let mut mem_conn = open_test_conn();
        // Seed employee
        let emp_id: i64 = mem_conn.query_row(
            "INSERT INTO employees(employee_id, full_name, email, is_floating) VALUES('ASWVN1229', 'TRẦN TẤN QUÝ', 'quy@example.com', 0) RETURNING id",
            [],
            |r| r.get(0),
        ).unwrap();
        let mon1: i64 = mem_conn.query_row(
            "INSERT INTO assets(asset_code, asset_type, display_name, status) VALUES('VNMON690', 'Monitor', 'Dell', 'in_stock') RETURNING id",
            [],
            |r| r.get(0),
        ).unwrap();

        let tx = mem_conn.transaction().unwrap();
        apply_single_historical_assignment_tx(
            &tx,
            mon1,
            "VNMON690",
            emp_id,
            "ASWVN1229",
            "TRẦN TẤN QUÝ",
            "Tại công ty",
            Some(1),
        ).unwrap();
        tx.commit().unwrap();

        // Check assigned asset query returns pending_signature
        let assigned = crate::db::employee::get_employee_assigned_assets_conn(&mem_conn, emp_id).unwrap();
        assert_eq!(assigned.len(), 1);
        assert_eq!(assigned[0].signature_status.as_deref(), Some(SIGNATURE_STATUS_PENDING));
        assert_eq!(assigned[0].usage_location.as_deref(), Some("Tại công ty"));

        // Check asset dashboard serialized query returns holder and location
        let dashboard_rows = crate::db::asset::list_asset_dashboard_serialized_conn(&mem_conn).unwrap();
        let mon1_row = dashboard_rows.iter().find(|r| r.asset_id == mon1).unwrap();
        assert_eq!(mon1_row.status, "assigned");
        assert_eq!(mon1_row.holder_employee_id.as_deref(), Some("ASWVN1229"));
        assert_eq!(mon1_row.holder_full_name.as_deref(), Some("TRẦN TẤN QUÝ"));
        assert_eq!(mon1_row.usage_location.as_deref(), Some("Tại công ty"));
        assert_eq!(mon1_row.signature_status.as_deref(), Some(SIGNATURE_STATUS_PENDING));

        // Genuine signing transitions to signed
        let req_id: i64 = mem_conn.query_row("SELECT id FROM borrow_requests LIMIT 1", [], |r| r.get(0)).unwrap();
        sign_historical_borrow_request_conn(
            &mut mem_conn,
            &SignHistoricalBorrowRequestInput {
                request_id: req_id,
                signature_png_blob: Some(vec![1, 2, 3]),
                typed_name: Some("Trần Tấn Quý".to_string()),
            },
        ).unwrap();

        let updated_assigned = crate::db::employee::get_employee_assigned_assets_conn(&mem_conn, emp_id).unwrap();
        assert_eq!(updated_assigned[0].signature_status.as_deref(), Some(SIGNATURE_STATUS_SIGNED));
    }

    #[test]
    fn test_isolated_db_apply_and_reimport_idempotence() {
        use crate::db::open_encrypted_connection;

        let isolated_db_path = if Path::new("scratch/isolated_test.sqlite3").exists() {
            Path::new("scratch/isolated_test.sqlite3").to_path_buf()
        } else if Path::new("../scratch/isolated_test.sqlite3").exists() {
            Path::new("../scratch/isolated_test.sqlite3").to_path_buf()
        } else {
            return;
        };

        // Create temporary copy of isolated test DB
        let temp_dir = std::env::temp_dir();
        let copy_path = temp_dir.join(format!("staffkit_isolated_apply_test_{}.sqlite3", std::process::id()));
        let _ = std::fs::copy(&isolated_db_path, &copy_path).expect("copy isolated test db");

        let file_path = if Path::new("00_ExSource/Asigned_Asset.xlsx").exists() {
            "00_ExSource/Asigned_Asset.xlsx".to_string()
        } else {
            "../00_ExSource/Asigned_Asset.xlsx".to_string()
        };

        let mut conn = open_encrypted_connection(&copy_path).expect("open encrypted copy of isolated test db");

        // First apply
        let report1 = apply_historical_assignment_import_conn(
            &mut conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.clone(),
                sheet_name: None,
                ..Default::default()
            },
            Some(1),
        ).expect("apply first historical assignment import");

        println!("FIRST APPLY REPORT: {:#?}", report1);
        assert_eq!(report1.total_processed, 200);
        assert_eq!(report1.created_assets, 193);
        assert_eq!(report1.created_assignments, 193);
        assert_eq!(report1.excluded_conflicts, 5);
        assert_eq!(report1.skipped_already_assigned, 0);
        assert_eq!(report1.errors.len(), 2);

        // Verify borrow_requests count
        let total_requests: i64 = conn.query_row("SELECT count(*) FROM borrow_requests WHERE decision_note LIKE 'Historical%'", [], |r| r.get(0)).unwrap();
        assert_eq!(total_requests, 193);

        // Second apply (idempotent reimport)
        let report2 = apply_historical_assignment_import_conn(
            &mut conn,
            &HistoricalAssignmentImportInput {
                file_path,
                sheet_name: None,
                ..Default::default()
            },
            Some(1),
        ).expect("reimport should succeed idempotently");

        println!("SECOND APPLY REPORT (IDEMPOTENT): {:#?}", report2);
        assert_eq!(report2.total_processed, 200);
        assert_eq!(report2.created_assets, 0, "No duplicate assets created on reimport");
        assert_eq!(report2.created_assignments, 0, "No duplicate assignments created on reimport");
        assert_eq!(report2.skipped_already_assigned, 193, "All 193 valid items skipped as already assigned");
        assert_eq!(report2.excluded_conflicts, 5, "Conflicts remain quarantined");
        assert_eq!(report2.errors.len(), 2);

        // Verify borrow_requests count is still 193 (no duplicates!)
        let total_requests_after: i64 = conn.query_row("SELECT count(*) FROM borrow_requests WHERE decision_note LIKE 'Historical%'", [], |r| r.get(0)).unwrap();
        assert_eq!(total_requests_after, 193);

        drop(conn);
        let _ = std::fs::remove_file(&copy_path);
    }

    #[test]
    fn test_print_7_row_exception_table() {
        use crate::db::open_encrypted_read_only_connection;

        let isolated_db_path = if Path::new("scratch/isolated_test.sqlite3").exists() {
            Path::new("scratch/isolated_test.sqlite3").to_path_buf()
        } else if Path::new("../scratch/isolated_test.sqlite3").exists() {
            Path::new("../scratch/isolated_test.sqlite3").to_path_buf()
        } else {
            return;
        };

        let read_conn = open_encrypted_read_only_connection(&isolated_db_path).expect("open read-only isolated db");
        let file_path = if Path::new("00_ExSource/Asigned_Asset.xlsx").exists() {
            "00_ExSource/Asigned_Asset.xlsx".to_string()
        } else {
            "../00_ExSource/Asigned_Asset.xlsx".to_string()
        };

        let preview = preview_historical_assignment_import_conn(
            &read_conn,
            &HistoricalAssignmentImportInput {
                file_path,
                sheet_name: None,
                ..Default::default()
            },
        ).expect("preview historical assignment import");

        let exceptions: Vec<&HistoricalAssignmentItem> = preview
            .items
            .iter()
            .filter(|i| {
                i.status == HistoricalAssignmentRowStatus::Conflict
                    || i.status == HistoricalAssignmentRowStatus::Error
                    || i.status == HistoricalAssignmentRowStatus::MissingEmployee
            })
            .collect();

        println!("=== 7-ASSIGNMENT EXCEPTION DETAILS ===");
        println!("Found {} exceptions in preview", exceptions.len());
        assert_eq!(exceptions.len(), 7, "Must find exactly 7 quarantined/error exceptions");

        for item in &exceptions {
            // Check DB for raw_staff_id
            let db_staff: Option<(i64, String, String, Option<String>, Option<String>)> = read_conn
                .query_row(
                    "SELECT id, employee_id, full_name, lifecycle_status, staff_group FROM employees WHERE employee_id = ?",
                    params![item.raw_staff_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )
                .optional()
                .unwrap();

            // Search DB by name
            let name_matches: Vec<(String, String, Option<String>)> = {
                let mut stmt = read_conn.prepare("SELECT employee_id, full_name, lifecycle_status FROM employees WHERE UPPER(full_name) LIKE UPPER(?) OR employee_id LIKE ?").unwrap();
                let last_word = item.raw_employee_name.split_whitespace().last().unwrap_or("");
                let rows = stmt.query_map(params![format!("%{}%", last_word), format!("%{}%", item.raw_staff_id.replace("ASWN", "ASWVN"))], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
                rows.filter_map(|r| r.ok()).collect()
            };

            println!(
                "EXCEL_ROW={} | STAFF_ID={} | NAME={} | ASSET={} | LOC={} | STATUS={:?} | REASON={:?} | DB_MATCH={:?} | NAME_MATCHES={:?}",
                item.row_number,
                item.raw_staff_id,
                item.raw_employee_name,
                item.raw_asset_code,
                item.location,
                item.status,
                item.status_reason,
                db_staff,
                name_matches
            );
        }
    }

    #[test]
    fn test_reconcile_new_asigned_asset_corrected_full_validation() {
        use crate::db::open_encrypted_read_only_connection;

        let db_path = if Path::new("scratch/isolated_reconcile.sqlite3").exists() {
            "scratch/isolated_reconcile.sqlite3"
        } else if Path::new("../scratch/isolated_reconcile.sqlite3").exists() {
            "../scratch/isolated_reconcile.sqlite3"
        } else {
            return;
        };

        let file_path = if Path::new("00_ExSource/New_Asigned_Asset_Corrected.xlsx").exists() {
            "00_ExSource/New_Asigned_Asset_Corrected.xlsx"
        } else if Path::new("../00_ExSource/New_Asigned_Asset_Corrected.xlsx").exists() {
            "../00_ExSource/New_Asigned_Asset_Corrected.xlsx"
        } else {
            return;
        };

        let conn = open_encrypted_read_only_connection(Path::new(db_path)).expect("open isolated db");
        let preview = preview_historical_assignment_import_conn(
            &conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                row_overrides: None,
                row_actions: None,
            },
        ).expect("preview");

        // Verify summary on the corrected workbook
        assert_eq!(preview.summary.total_rows, 156, "Must have exactly 156 rows");
        assert_eq!(preview.summary.total_assignments, 199, "Must have exactly 199 assignments");
        assert_eq!(preview.summary.new_assets, 199, "All 199 valid assignments without prior monitor loans");
        assert_eq!(preview.summary.conflicts, 0, "Zero conflicts on corrected workbook");
        assert_eq!(preview.summary.missing_employees, 0, "Zero missing employees");
        assert_eq!(preview.summary.errors, 0, "Zero errors");
        assert!(preview.summary.can_approve_remainder);
        assert!(!preview.summary.has_blocking_conflicts);

        // Verify Vũ Tuấn Khang (ASWVN1112) has both MON700 and MON644 mapped validly
        let khang_items: Vec<&HistoricalAssignmentItem> = preview.items.iter().filter(|i| i.raw_staff_id == "ASWVN1112").collect();
        assert_eq!(khang_items.len(), 2, "ASWVN1112 must hold exactly 2 monitors");
        let khang_codes: HashSet<String> = khang_items.iter().map(|i| i.raw_asset_code.clone()).collect();
        assert!(khang_codes.contains("MON700"));
        assert!(khang_codes.contains("MON644"));
        for item in khang_items {
            assert_eq!(item.status, HistoricalAssignmentRowStatus::NewAsset);
            assert_eq!(item.location, "Tại công ty");
        }

        // Verify Nguyễn Công Thiện (ASWVN027) holds MON667
        let thien_item = preview.items.iter().find(|i| i.raw_staff_id == "ASWVN027").expect("ASWVN027");
        assert_eq!(thien_item.raw_asset_code, "MON667");
        assert_eq!(thien_item.status, HistoricalAssignmentRowStatus::NewAsset);
    }

    #[test]
    fn test_reconcile_mon667_transfer_requires_explicit_confirmation() {
        let mut conn = open_test_conn();

        // 1. Seed existing holder A: ASWVN958 (Nguyễn Quốc Trọng Nghĩa)
        let emp_a_id: i64 = conn.query_row(
            "INSERT INTO employees(employee_id, full_name, email, is_floating) VALUES('ASWVN958', 'NGUYỄN QUỐC TRỌNG NGHĨA', 'nghia@example.com', 0) RETURNING id",
            [],
            |r| r.get(0),
        ).unwrap();

        // 2. Seed verified holder B: ASWVN027 (Nguyễn Công Thiện)
        let emp_b_id: i64 = conn.query_row(
            "INSERT INTO employees(employee_id, full_name, email, is_floating) VALUES('ASWVN027', 'NGUYỄN CÔNG THIỆN', 'thien@example.com', 0) RETURNING id",
            [],
            |r| r.get(0),
        ).unwrap();

        // 3. Seed asset VNMON667 with an active loan to employee A
        let cat_id = get_or_create_monitor_category(&conn.transaction().unwrap()).unwrap();
        let mon667_id: i64 = conn.query_row(
            r#"
            INSERT INTO assets(asset_code, category_id, asset_type, display_name, status, usage_location)
            VALUES('VNMON667', ?, 'Monitor', 'Dell 24', 'assigned', 'Tại công ty')
            RETURNING id
            "#,
            params![cat_id],
            |r| r.get(0),
        ).unwrap();

        let req_a_id: i64 = conn.query_row(
            r#"
            INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status, request_type, signature_status)
            VALUES('REQ-A-667', ?, 'ASWVN958', 'NGUYỄN QUỐC TRỌNG NGHĨA', 'approved', 'borrow', 'signed')
            RETURNING id
            "#,
            params![emp_a_id],
            |r| r.get(0),
        ).unwrap();

        conn.execute(
            r#"
            INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id, borrowed_at)
            VALUES(?, ?, ?, datetime('now', '-30 days'))
            "#,
            params![mon667_id, emp_a_id, req_a_id],
        ).unwrap();

        let file_path = if Path::new("00_ExSource/New_Asigned_Asset_Corrected.xlsx").exists() {
            "00_ExSource/New_Asigned_Asset_Corrected.xlsx"
        } else {
            "../00_ExSource/New_Asigned_Asset_Corrected.xlsx"
        };

        // Run Preview
        let preview = preview_historical_assignment_import_conn(
            &conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                row_overrides: None,
                row_actions: None,
            },
        ).unwrap();

        // MON667 is assigned to ASWVN027 in the workbook, but actively held by ASWVN958 in DB
        let item_667 = preview.items.iter().find(|i| i.canonical_asset_code.as_deref() == Some("VNMON667")).expect("VNMON667 in preview");
        assert_eq!(item_667.status, HistoricalAssignmentRowStatus::DifferentHolder);
        assert_eq!(item_667.raw_staff_id, "ASWVN027");
        assert_eq!(item_667.current_holder_staff_id.as_deref(), Some("ASWVN958"));
        assert_eq!(item_667.suggested_action.as_deref(), Some("quarantine"));
        assert!(item_667.status_reason.as_ref().unwrap().contains("currently assigned in DB to 'NGUYỄN QUỐC TRỌNG NGHĨA (ASWVN958)'"));

        // Case A: Apply without confirmation -> MON667 is quarantined, NO writes to DB
        let report_unconfirmed = apply_historical_assignment_import_conn(
            &mut conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                row_overrides: None,
                row_actions: None, // No confirm_transfer
            },
            Some(1),
        ).unwrap();

        assert_eq!(report_unconfirmed.transferred_assignments, 0);
        // Verify active loan for employee A remains unchanged
        let loan_a_active: bool = conn.query_row(
            "SELECT returned_at IS NULL FROM asset_loans WHERE asset_id = ? AND employee_id_fk = ?",
            params![mon667_id, emp_a_id],
            |r| r.get(0),
        ).unwrap();
        assert!(loan_a_active, "Without explicit confirmation, old loan must NEVER be closed");

        // Case B: Apply WITH explicit IT confirmation: action = "confirm_transfer"
        let report_confirmed = apply_historical_assignment_import_conn(
            &mut conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                row_overrides: None,
                row_actions: Some(vec![HistoricalAssignmentActionDecision {
                    row_number: item_667.row_number,
                    raw_asset_code: "MON667".to_string(),
                    action: "confirm_transfer".to_string(),
                    corrected_staff_id: None,
                    corrected_asset_code: None,
                }]),
            },
            Some(1),
        ).unwrap();

        assert_eq!(report_confirmed.transferred_assignments, 1, "Transfer must be recorded");

        // Verify old loan for A is now closed
        let returned_at: Option<String> = conn.query_row(
            "SELECT returned_at FROM asset_loans WHERE asset_id = ? AND employee_id_fk = ?",
            params![mon667_id, emp_a_id],
            |r| r.get(0),
        ).unwrap();
        assert!(returned_at.is_some(), "Old loan must have returned_at set");

        // Verify new loan exists for verified holder B (ASWVN027)
        let (new_loan_id, new_borrowed): (i64, String) = conn.query_row(
            "SELECT id, borrowed_at FROM asset_loans WHERE asset_id = ? AND employee_id_fk = ? AND returned_at IS NULL",
            params![mon667_id, emp_b_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert!(new_loan_id > 0);
        assert!(!new_borrowed.is_empty());

        // Verify borrow_request has pending_signature
        let sig_status: String = conn.query_row(
            "SELECT br.signature_status FROM borrow_requests br JOIN asset_loans al ON al.borrow_request_id = br.id WHERE al.id = ?",
            params![new_loan_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(sig_status, SIGNATURE_STATUS_PENDING);
    }

    #[test]
    fn test_same_holder_skip_and_location_update() {
        let mut conn = open_test_conn();

        let emp_id: i64 = conn.query_row(
            "INSERT INTO employees(employee_id, full_name, email, is_floating) VALUES('ASWVN1112', 'VŨ TUẤN KHANG', 'khang@example.com', 0) RETURNING id",
            [],
            |r| r.get(0),
        ).unwrap();

        let cat_id = get_or_create_monitor_category(&conn.transaction().unwrap()).unwrap();
        let mon700_id: i64 = conn.query_row(
            "INSERT INTO assets(asset_code, category_id, asset_type, display_name, status, usage_location) VALUES('VNMON700', ?, 'Monitor', 'Dell', 'assigned', 'Tại nhà') RETURNING id",
            params![cat_id],
            |r| r.get(0),
        ).unwrap();

        let req_id: i64 = conn.query_row(
            "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status, request_type, signature_status) VALUES('REQ-700', ?, 'ASWVN1112', 'VŨ TUẤN KHANG', 'approved', 'borrow', 'signed') RETURNING id",
            params![emp_id],
            |r| r.get(0),
        ).unwrap();

        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id, borrowed_at) VALUES(?, ?, ?, datetime('now'))",
            params![mon700_id, emp_id, req_id],
        ).unwrap();

        let file_path = if Path::new("00_ExSource/New_Asigned_Asset_Corrected.xlsx").exists() {
            "00_ExSource/New_Asigned_Asset_Corrected.xlsx"
        } else {
            "../00_ExSource/New_Asigned_Asset_Corrected.xlsx"
        };

        // Preview: DB is 'Tại nhà', workbook is 'Tại công ty' (Tại CTY)
        let preview = preview_historical_assignment_import_conn(
            &conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                row_overrides: None,
                row_actions: None,
            },
        ).unwrap();

        let item700 = preview.items.iter().find(|i| i.canonical_asset_code.as_deref() == Some("VNMON700")).expect("VNMON700");
        assert_eq!(item700.status, HistoricalAssignmentRowStatus::LocationChanged);
        assert_eq!(item700.suggested_action.as_deref(), Some("update_location"));
        assert!(item700.status_reason.as_ref().unwrap().contains("location changed from 'Tại nhà' to 'Tại công ty'"));

        // Apply with default update_location: updates location, preserves active loan
        let report = apply_historical_assignment_import_conn(
            &mut conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                row_overrides: None,
                row_actions: None,
            },
            Some(1),
        ).unwrap();

        assert_eq!(report.updated_locations, 1);
        let updated_loc: String = conn.query_row("SELECT usage_location FROM assets WHERE id = ?", params![mon700_id], |r| r.get(0)).unwrap();
        assert_eq!(updated_loc, "Tại công ty");

        // Verify active loan was NOT closed
        let active_loan_exists: bool = conn.query_row(
            "SELECT count(*) = 1 FROM asset_loans WHERE asset_id = ? AND employee_id_fk = ? AND returned_at IS NULL",
            params![mon700_id, emp_id],
            |r| r.get(0),
        ).unwrap();
        assert!(active_loan_exists, "Existing loan must be preserved on location update");

        // Re-run Preview now that location matches: status is AlreadyAssigned (Skip)
        let preview2 = preview_historical_assignment_import_conn(
            &conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                row_overrides: None,
                row_actions: None,
            },
        ).unwrap();
        let item700_after = preview2.items.iter().find(|i| i.canonical_asset_code.as_deref() == Some("VNMON700")).expect("VNMON700");
        assert_eq!(item700_after.status, HistoricalAssignmentRowStatus::AlreadyAssigned);
        assert_eq!(item700_after.suggested_action.as_deref(), Some("skip"));
    }

    #[test]
    fn test_existing_loan_missing_from_workbook_detected() {
        let mut conn = open_test_conn();

        let emp_id: i64 = conn.query_row(
            "INSERT INTO employees(employee_id, full_name, email, is_floating) VALUES('ASWVN9999', 'TEST USER', 'test@example.com', 0) RETURNING id",
            [],
            |r| r.get(0),
        ).unwrap();

        let cat_id = get_or_create_monitor_category(&conn.transaction().unwrap()).unwrap();
        let mon999_id: i64 = conn.query_row(
            "INSERT INTO assets(asset_code, category_id, asset_type, display_name, status, usage_location) VALUES('VNMON999', ?, 'Monitor', 'Dell 27', 'assigned', 'Tại công ty') RETURNING id",
            params![cat_id],
            |r| r.get(0),
        ).unwrap();

        let req_id: i64 = conn.query_row(
            "INSERT INTO borrow_requests(request_key, employee_id_fk, submitted_employee_id, submitted_full_name, status, request_type, signature_status) VALUES('REQ-999', ?, 'ASWVN9999', 'TEST USER', 'approved', 'borrow', 'signed') RETURNING id",
            params![emp_id],
            |r| r.get(0),
        ).unwrap();

        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id, borrowed_at) VALUES(?, ?, ?, datetime('now'))",
            params![mon999_id, emp_id, req_id],
        ).unwrap();

        let file_path = if Path::new("00_ExSource/New_Asigned_Asset_Corrected.xlsx").exists() {
            "00_ExSource/New_Asigned_Asset_Corrected.xlsx"
        } else {
            "../00_ExSource/New_Asigned_Asset_Corrected.xlsx"
        };

        let preview = preview_historical_assignment_import_conn(
            &conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                row_overrides: None,
                row_actions: None,
            },
        ).unwrap();

        assert_eq!(preview.summary.existing_loans_missing, 1);
        let missing_item = preview.items.iter().find(|i| i.raw_asset_code == "VNMON999").expect("VNMON999 in preview");
        assert_eq!(missing_item.status, HistoricalAssignmentRowStatus::ExistingLoanMissingFromWorkbook);
        assert_eq!(missing_item.suggested_action.as_deref(), Some("review"));
        assert!(missing_item.status_reason.as_ref().unwrap().contains("not present in verified workbook"));

        // Apply must NOT modify or return this loan
        let _ = apply_historical_assignment_import_conn(
            &mut conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                row_overrides: None,
                row_actions: None,
            },
            Some(1),
        ).unwrap();

        let is_still_active: bool = conn.query_row(
            "SELECT returned_at IS NULL FROM asset_loans WHERE asset_id = ?",
            params![mon999_id],
            |r| r.get(0),
        ).unwrap();
        assert!(is_still_active, "Existing loan missing from workbook must remain active and untouched");
    }

    #[test]
    fn test_preview_row_overrides_preview_only() {
        let conn = open_test_conn();

        let file_path = if Path::new("00_ExSource/New_Asigned_Asset_Corrected.xlsx").exists() {
            "00_ExSource/New_Asigned_Asset_Corrected.xlsx"
        } else {
            "../00_ExSource/New_Asigned_Asset_Corrected.xlsx"
        };

        // Row 2 is ASWVN030 MON540
        // Override row 2 with an unseeded staff id ASWVN9999
        let preview = preview_historical_assignment_import_conn(
            &conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
                row_overrides: Some(vec![HistoricalAssignmentRowOverride {
                    row_number: 2,
                    raw_asset_code: "MON540".to_string(),
                    corrected_staff_id: Some("ASWVN9999".to_string()),
                    corrected_asset_code: None,
                }]),
                row_actions: None,
            },
        ).unwrap();

        let row2_item = preview.items.iter().find(|i| i.row_number == 2).unwrap();
        assert_eq!(row2_item.raw_staff_id, "ASWVN9999");
        assert_eq!(row2_item.status, HistoricalAssignmentRowStatus::MissingEmployee);
    }

    #[test]
    fn test_inspect_new_asigned_asset_workbook() {
        let file_path = Path::new(r"D:\Staff_Kit\00_ExSource\New_Asigned_Asset.xlsx");
        assert!(file_path.exists(), "New_Asigned_Asset.xlsx must exist");

        let mut workbook = open_workbook_auto(file_path).expect("open workbook");
        let sheet_names = workbook.sheet_names();
        println!("SHEET_NAMES: {:?}", sheet_names);

        for sheet in &sheet_names {
            if let Ok(range) = workbook.worksheet_range(sheet) {
                println!("--- SHEET: {} (rows={}, cols={}) ---", sheet, range.height(), range.width());
                let mut total_cty = 0;
                let mut total_nha = 0;
                let mut all_tags = std::collections::HashSet::new();
                let mut data_rows = 0;

                for (_r_idx, row) in range.rows().enumerate().skip(1) {
                    let staff_id = cell_to_string(&row[0]);
                    let name = cell_to_string(&row[1]);
                    let cty_str = cell_to_string(&row[2]);
                    let nha_str = cell_to_string(&row[3]);

                    if staff_id.is_empty() && name.is_empty() && cty_str.is_empty() && nha_str.is_empty() {
                        continue;
                    }
                    data_rows += 1;

                    let cty_codes = split_monitor_codes(&cty_str);
                    let nha_codes = split_monitor_codes(&nha_str);
                    total_cty += cty_codes.len();
                    total_nha += nha_codes.len();

                    for c in &cty_codes {
                        all_tags.insert(canonicalize_monitor_code(c));
                    }
                    for c in &nha_codes {
                        all_tags.insert(canonicalize_monitor_code(c));
                    }
                }

                println!("DATA_ROWS: {}", data_rows);
                println!("TOTAL_CTY: {}", total_cty);
                println!("TOTAL_NHA: {}", total_nha);
                println!("TOTAL_ASSIGNMENTS: {}", total_cty + total_nha);
                println!("UNIQUE_TAGS: {}", all_tags.len());
            }
        }
    }

    #[test]
    fn test_preview_latest_new_asigned_asset_on_isolated_db() {
        let original_db = Path::new(r"D:\Staff_Kit\data\test_db\staff_kit.sqlite3");
        assert!(original_db.exists(), "original test db must exist");

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("timestamp")
            .as_nanos();
        let temp_db = std::env::temp_dir().join(format!("staffkit-reconcile-new-asigned-{timestamp}.sqlite3"));
        std::fs::copy(original_db, &temp_db).expect("copy test db to temp");
        let conn = crate::db::open_encrypted_connection(&temp_db).expect("open temp db");

        let file_path = r"D:\Staff_Kit\00_ExSource\New_Asigned_Asset.xlsx";

        // Check if the 5 new assets exist in original DB
        for tag in ["VNMON586", "VNMON571", "VNMON641", "VNMON690", "VNMON568"] {
            let found: Option<(i64, String, String)> = conn.query_row(
                "SELECT id, asset_code, status FROM assets WHERE asset_code = ? COLLATE NOCASE",
                params![tag],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ).optional().unwrap();
            println!("CHECK DB ASSET {}: {:?}", tag, found);
        }
        let payload = HistoricalAssignmentImportInput {
            file_path: file_path.to_string(),
            sheet_name: None,
            row_overrides: None,
            row_actions: None,
        };

        let preview = preview_historical_assignment_import_conn(&conn, &payload).expect("preview must succeed");
        println!("PREVIEW SUMMARY: {:?}", preview.summary);

        let mut status_counts = std::collections::HashMap::new();
        for item in &preview.items {
            *status_counts.entry(format!("{:?}", item.status)).or_insert(0) += 1;
        }
        println!("STATUS COUNTS: {:?}", status_counts);

        // Print items that are not AlreadyAssigned
        for item in &preview.items {
            if item.status == HistoricalAssignmentRowStatus::NewAssignment {
                println!(
                    "NEW ASSIGNMENT: row={}, staff='{}' ({:?}), asset='{}' ({:?}), location={}",
                    item.row_number, item.raw_staff_id, item.canonical_employee_name, item.raw_asset_code, item.canonical_asset_code, item.location
                );
            }
            if item.status != HistoricalAssignmentRowStatus::AlreadyAssigned
                && item.status != HistoricalAssignmentRowStatus::NewAssignment
                && item.status != HistoricalAssignmentRowStatus::LocationChanged
            {
                println!(
                    "ITEM EXCEPTION: row={}, staff='{}' ({:?}), asset='{}' ({:?}), status={:?}, reason={:?}",
                    item.row_number, item.raw_staff_id, item.canonical_employee_name, item.raw_asset_code, item.canonical_asset_code, item.status, item.status_reason
                );
            }
        }

        // Test preview with row_overrides for the 3 identity corrections
        let overrides_payload = HistoricalAssignmentImportInput {
            file_path: file_path.to_string(),
            sheet_name: None,
            row_overrides: Some(vec![
                HistoricalAssignmentRowOverride {
                    row_number: 65,
                    raw_asset_code: "MON667".to_string(),
                    corrected_staff_id: Some("ASWVN027".to_string()),
                    corrected_asset_code: None,
                },
                HistoricalAssignmentRowOverride {
                    row_number: 101,
                    raw_asset_code: "MON738".to_string(),
                    corrected_staff_id: Some("ASWVN910".to_string()),
                    corrected_asset_code: None,
                },
                HistoricalAssignmentRowOverride {
                    row_number: 153,
                    raw_asset_code: "MON556".to_string(),
                    corrected_staff_id: Some("ASWVN1323".to_string()),
                    corrected_asset_code: None,
                },
            ]),
            row_actions: None,
        };

        let preview_corrected = preview_historical_assignment_import_conn(&conn, &overrides_payload).expect("preview with overrides");
        println!("PREVIEW WITH OVERRIDES SUMMARY: {:?}", preview_corrected.summary);

        let _ = std::fs::remove_file(&temp_db);
    }

    #[test]
    fn test_reconcile_9_zon_verified_monitors() {
        let original_db = Path::new(r"D:\Staff_Kit\data\test_db\staff_kit.sqlite3");
        assert!(original_db.exists(), "original test db must exist");

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("timestamp")
            .as_nanos();
        let temp_db = std::env::temp_dir().join(format!("staffkit-reconcile-9-monitors-{timestamp}.sqlite3"));
        std::fs::copy(original_db, &temp_db).expect("copy test db to temp");
        let conn = crate::db::open_encrypted_connection(&temp_db).expect("open temp db");

        let target_pairs = [
            ("ASWVN1019", "MON586"),
            ("ASWVN1059", "MON578"),
            ("ASWVN1179", "MON571"),
            ("ASWVN1271", "MON568"),
            ("ASWVN1172", "MON610"),
            ("ASWVN1198", "MON641"),
            ("ASWVN974", "MON668"),
            ("ASWVN1233", "MON530"),
            ("ASWVN1320", "MON596"),
        ];

        // Also inspect workbook mappings
        let file_path = Path::new(r"D:\Staff_Kit\00_ExSource\New_Asigned_Asset.xlsx");
        let mut workbook_rows = Vec::new();
        if file_path.exists() {
            let mut wb = open_workbook_auto(file_path).expect("open workbook");
            if let Ok(range) = wb.worksheet_range("New") {
                for (r_idx, row) in range.rows().enumerate().skip(1) {
                    let s_id = cell_to_string(&row[0]);
                    let name = cell_to_string(&row[1]);
                    let cty = cell_to_string(&row[2]);
                    let nha = cell_to_string(&row[3]);
                    let cty_codes = split_monitor_codes(&cty);
                    let nha_codes = split_monitor_codes(&nha);
                    for c in cty_codes {
                        workbook_rows.push((r_idx + 1, s_id.clone(), name.clone(), canonicalize_monitor_code(&c), "Tại công ty".to_string()));
                    }
                    for c in nha_codes {
                        workbook_rows.push((r_idx + 1, s_id.clone(), name.clone(), canonicalize_monitor_code(&c), "Tại nhà".to_string()));
                    }
                }
            }
        }

        println!("==================== 9-ROW VERIFIED RECONCILIATION ====================");
        for (target_staff_id, target_mon) in &target_pairs {
            let canon_asset = canonicalize_monitor_code(target_mon);

            // 1. Employee info
            let target_emp: Option<(i64, String, String)> = conn.query_row(
                "SELECT id, employee_id, full_name FROM employees WHERE UPPER(TRIM(employee_id)) = UPPER(TRIM(?))",
                params![target_staff_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ).optional().unwrap();

            // 2. Asset info
            let asset_info: Option<(i64, String, String, Option<String>)> = conn.query_row(
                "SELECT id, asset_code, status, usage_location FROM assets WHERE asset_code = ? COLLATE NOCASE",
                params![canon_asset],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            ).optional().unwrap();

            // 3. Active loan info
            let active_loan: Option<(i64, String, String, String)> = if let Some((aid, _, _, _)) = &asset_info {
                conn.query_row(
                    r#"
                    SELECT al.id, e.employee_id, e.full_name, al.borrowed_at
                    FROM asset_loans al
                    JOIN employees e ON e.id = al.employee_id_fk
                    WHERE al.asset_id = ? AND al.returned_at IS NULL
                    "#,
                    params![aid],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                ).optional().unwrap()
            } else {
                None
            };

            // 4. Workbook info
            let wb_matches: Vec<_> = workbook_rows.iter().filter(|(_, _, _, c, _)| c == &canon_asset).collect();

            println!("--- Asset: {} ({}) ---", target_mon, canon_asset);
            println!("  Zon Target: {} ({:?}) at Tại CTY", target_staff_id, target_emp.as_ref().map(|e| &e.2));
            println!("  DB Asset Status: {:?}", asset_info);
            println!("  DB Active Loan: {:?}", active_loan);
            println!("  Workbook Entry: {:?}", wb_matches);

            let (classification, proposed_action) = match (&asset_info, &active_loan) {
                (None, _) => (
                    "Missing Asset in DB",
                    "Stage New Asset + Assignment (auto-create as Monitor, assign to Zon target, pending_signature)",
                ),
                (Some((_, _, _, db_loc)), Some((_, holder_id, _holder_name, _))) => {
                    if holder_id.eq_ignore_ascii_case(target_staff_id) {
                        let norm_db = normalize_location(db_loc.as_deref().unwrap_or(""));
                        let norm_target = normalize_location("Tại CTY");
                        if norm_db == norm_target {
                            ("Same Holder, Same Location", "Skip (already correctly assigned in DB)")
                        } else {
                            ("Same Holder, Different Location", "Explicit Location Update to 'Tại công ty'")
                        }
                    } else {
                        (
                            "Different Holder",
                            "Stage Confirm Transfer (close old loan, create new verified loan with pending_signature)",
                        )
                    }
                }
                (Some((_, _, _status, _)), None) => {
                    ("Asset in DB, No Active Loan", "Stage New Assignment (create loan for Zon target, pending_signature)")
                }
            };
            println!("  Classification: {}", classification);
            println!("  Proposed Action: {}", proposed_action);
        }
        println!("========================================================================");

        let _ = std::fs::remove_file(&temp_db);
    }

    #[test]
    fn test_apply_9_zon_verified_monitors_on_isolated_db() {
        let original_db = Path::new(r"D:\Staff_Kit\data\test_db\staff_kit.sqlite3");
        assert!(original_db.exists(), "original test db must exist");

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("timestamp")
            .as_nanos();
        let temp_db = std::env::temp_dir().join(format!("staffkit-apply-9-monitors-{timestamp}.sqlite3"));
        std::fs::copy(original_db, &temp_db).expect("copy test db to temp");
        let mut conn = crate::db::open_encrypted_connection(&temp_db).expect("open temp db");

        let target_pairs = [
            ("ASWVN1019", "MON586"),
            ("ASWVN1059", "MON578"),
            ("ASWVN1179", "MON571"),
            ("ASWVN1271", "MON568"),
            ("ASWVN1172", "MON610"),
            ("ASWVN1198", "MON641"),
            ("ASWVN974", "MON668"),
            ("ASWVN1233", "MON530"),
            ("ASWVN1320", "MON596"),
        ];

        let cat_id = {
            let tx = conn.transaction().expect("tx");
            let cid = get_or_create_monitor_category(&tx).expect("cat id");
            tx.commit().expect("commit");
            cid
        };

        let tx = conn.transaction().expect("transaction");

        for (target_staff_id, target_mon) in &target_pairs {
            let canon_asset = canonicalize_monitor_code(target_mon);

            let (emp_row_id, emp_staff_id, emp_full_name): (i64, String, String) = tx.query_row(
                "SELECT id, employee_id, full_name FROM employees WHERE UPPER(TRIM(employee_id)) = UPPER(TRIM(?))",
                params![target_staff_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ).expect("find target employee");

            let asset_opt: Option<i64> = tx.query_row(
                "SELECT id FROM assets WHERE asset_code = ? COLLATE NOCASE",
                params![canon_asset],
                |r| r.get(0),
            ).optional().expect("query asset");

            let asset_id = if let Some(aid) = asset_opt {
                // If exists and has active loan, close old loan (Transfer)
                let active_loan_id: Option<i64> = tx.query_row(
                    "SELECT id FROM asset_loans WHERE asset_id = ? AND returned_at IS NULL",
                    params![aid],
                    |r| r.get(0),
                ).optional().expect("query active loan");

                if let Some(old_loan_id) = active_loan_id {
                    tx.execute(
                        "UPDATE asset_loans SET returned_at = datetime('now') WHERE id = ?",
                        params![old_loan_id],
                    ).expect("close old loan");
                }

                tx.execute(
                    "UPDATE assets SET status = 'assigned', usage_location = 'Tại công ty', updated_at = datetime('now') WHERE id = ?",
                    params![aid],
                ).expect("update asset location");

                aid
            } else {
                // Auto-create new asset
                tx.execute(
                    r#"
                    INSERT INTO assets(asset_code, category_id, asset_type, display_name, display_name_short, usage_location, status, created_at, updated_at)
                    VALUES(?, ?, 'Monitor', ?, ?, 'Tại công ty', 'assigned', datetime('now'), datetime('now'))
                    "#,
                    params![canon_asset, cat_id, target_mon, target_mon],
                ).expect("insert asset");
                tx.last_insert_rowid()
            };

            // Apply assignment
            apply_single_historical_assignment_tx(
                &tx,
                asset_id,
                &canon_asset,
                emp_row_id,
                &emp_staff_id,
                &emp_full_name,
                "Tại công ty",
                Some(1),
            ).expect("apply single assignment");
        }

        tx.commit().expect("commit");

        // Verification assertions on isolated DB
        for (target_staff_id, target_mon) in &target_pairs {
            let canon_asset = canonicalize_monitor_code(target_mon);

            // Exactly 1 active loan per asset
            let active_loans: Vec<(i64, String, String)> = conn.prepare(
                r#"
                SELECT al.id, e.employee_id, al.borrowed_at
                FROM asset_loans al
                JOIN employees e ON e.id = al.employee_id_fk
                JOIN assets a ON a.id = al.asset_id
                WHERE a.asset_code = ? COLLATE NOCASE AND al.returned_at IS NULL
                "#
            ).unwrap().query_map(params![canon_asset], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap().map(|r| r.unwrap()).collect();

            assert_eq!(active_loans.len(), 1, "Must have exactly 1 active loan for {}", canon_asset);
            assert!(active_loans[0].1.eq_ignore_ascii_case(target_staff_id), "Holder must be {}", target_staff_id);

            // Asset status is assigned
            let (status, loc): (String, String) = conn.query_row(
                "SELECT status, usage_location FROM assets WHERE asset_code = ? COLLATE NOCASE",
                params![canon_asset],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ).unwrap();
            assert_eq!(status, "assigned");
            assert_eq!(loc, "Tại công ty");
        }

        // Integrity check: zero duplicate active loans across the entire DB
        let duplicates: i64 = conn.query_row(
            r#"
            SELECT count(*) FROM (
                SELECT asset_id, count(*) as c
                FROM asset_loans
                WHERE returned_at IS NULL
                GROUP BY asset_id
                HAVING count(*) > 1
            )
            "#,
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(duplicates, 0, "Zero assets may have multiple active loans");

        let _ = std::fs::remove_file(&temp_db);
    }
}

