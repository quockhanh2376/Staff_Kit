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
    NewAssignment,
    AlreadyAssigned,
    MissingAsset,
    MissingEmployee,
    Conflict,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalAssignmentPreviewSummary {
    pub total_rows: usize,
    pub total_assignments: usize,
    pub new_assignments: usize,
    pub already_assigned: usize,
    pub missing_assets: usize,
    pub missing_employees: usize,
    pub conflicts: usize,
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
pub struct HistoricalAssignmentImportInput {
    pub file_path: String,
    pub sheet_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalAssignmentImportReport {
    pub total_processed: usize,
    pub created_assignments: usize,
    pub skipped_already_assigned: usize,
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

fn normalize_name_for_comparison(name: &str) -> String {
    name.trim().to_lowercase().replace([' ', '\t'], "")
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
            let val = cell_to_string(cell).to_lowercase().replace(' ', "");
            if val.contains("staffid") || val.contains("manhanvien") || val.contains("eeid") {
                staff_id_col = Some(c_idx);
            } else if val.contains("nhanvien") || val.contains("tennhanvien") || val == "name" || val == "fullname" {
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
            let normalized_code_key = asset_code.trim().to_uppercase();

            // 1. Resolve Employee by canonical Staff ID
            let emp_info: Option<(i64, String, String)> = conn
                .query_row(
                    "SELECT id, employee_id, full_name FROM employees WHERE UPPER(TRIM(employee_id)) = UPPER(TRIM(?))",
                    params![row.staff_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
                .map_err(|err| format!("Error querying employee for '{}': {err}", row.staff_id))?;

            // 2. Resolve Asset by canonical Asset Code
            // Try exact match, or with 'VN' prefix, or stripped 'VN' prefix
            let asset_info: Option<(i64, String)> = {
                let exact = conn
                    .query_row(
                        "SELECT id, asset_code FROM assets WHERE asset_code = ? COLLATE NOCASE",
                        params![asset_code],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()
                    .map_err(|err| format!("Error querying asset '{asset_code}': {err}"))?;

                if exact.is_some() {
                    exact
                } else if !normalized_code_key.starts_with("VN") {
                    let with_vn = format!("VN{normalized_code_key}");
                    conn.query_row(
                        "SELECT id, asset_code FROM assets WHERE asset_code = ? COLLATE NOCASE",
                        params![with_vn],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()
                    .map_err(|err| format!("Error querying asset with VN prefix: {err}"))?
                } else {
                    let without_vn = &normalized_code_key[2..];
                    conn.query_row(
                        "SELECT id, asset_code FROM assets WHERE asset_code = ? COLLATE NOCASE",
                        params![without_vn],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()
                    .map_err(|err| format!("Error querying asset without VN prefix: {err}"))?
                }
            };

            let mut item = HistoricalAssignmentItem {
                row_number: row.row_number,
                raw_staff_id: row.staff_id.clone(),
                raw_employee_name: row.employee_name.clone(),
                raw_asset_code: asset_code.clone(),
                location: location.to_string(),
                status: HistoricalAssignmentRowStatus::NewAssignment,
                status_reason: None,
                employee_id: emp_info.as_ref().map(|e| e.0),
                canonical_staff_id: emp_info.as_ref().map(|e| e.1.clone()),
                canonical_employee_name: emp_info.as_ref().map(|e| e.2.clone()),
                asset_id: asset_info.as_ref().map(|a| a.0),
                canonical_asset_code: asset_info.as_ref().map(|a| a.1.clone()),
                current_holder_staff_id: None,
                current_holder_name: None,
            };

            // Check Employee validity and identity match
            match &emp_info {
                None => {
                    item.status = HistoricalAssignmentRowStatus::MissingEmployee;
                    item.status_reason = Some(format!(
                        "Employee Staff ID '{}' not found in database",
                        row.staff_id
                    ));
                }
                Some((_, _, db_full_name)) => {
                    if !row.employee_name.is_empty() {
                        let norm_db = normalize_name_for_comparison(db_full_name);
                        let norm_raw = normalize_name_for_comparison(&row.employee_name);
                        if norm_db != norm_raw {
                            item.status = HistoricalAssignmentRowStatus::Conflict;
                            item.status_reason = Some(format!(
                                "Staff ID '{}' belongs to '{}' in database, but row lists '{}'",
                                row.staff_id, db_full_name, row.employee_name
                            ));
                        }
                    }
                }
            }

            // If employee had conflict or was missing, asset check still sets details
            match &asset_info {
                None => {
                    if item.status == HistoricalAssignmentRowStatus::NewAssignment {
                        item.status = HistoricalAssignmentRowStatus::MissingAsset;
                        item.status_reason = Some(format!("Asset '{}' not found in database", asset_code));
                    }
                }
                Some((asset_id, canonical_code)) => {
                    // Check active loan in asset_loans
                    let active_loan: Option<(i64, String, String)> = conn
                        .query_row(
                            r#"
                            SELECT al.employee_id_fk, e.employee_id, e.full_name
                            FROM asset_loans al
                            INNER JOIN employees e ON e.id = al.employee_id_fk
                            WHERE al.asset_id = ? AND al.returned_at IS NULL
                            "#,
                            params![asset_id],
                            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                        )
                        .optional()
                        .map_err(|err| format!("Error checking active loan for asset '{canonical_code}': {err}"))?;

                    if let Some((holder_emp_fk, holder_staff_id, holder_full_name)) = active_loan {
                        item.current_holder_staff_id = Some(holder_staff_id.clone());
                        item.current_holder_name = Some(holder_full_name.clone());

                        if let Some((target_emp_id, _, _)) = &emp_info {
                            if *target_emp_id == holder_emp_fk {
                                if item.status == HistoricalAssignmentRowStatus::NewAssignment {
                                    item.status = HistoricalAssignmentRowStatus::AlreadyAssigned;
                                    item.status_reason = Some(format!(
                                        "Asset '{}' is already assigned to this employee",
                                        canonical_code
                                    ));
                                }
                            } else {
                                item.status = HistoricalAssignmentRowStatus::Conflict;
                                item.status_reason = Some(format!(
                                    "Asset '{}' is currently assigned to another employee: '{} ({})'",
                                    canonical_code, holder_full_name, holder_staff_id
                                ));
                            }
                        }
                    }

                    // Check intra-file duplicate
                    let canon_key = canonical_code.to_uppercase();
                    if let Some((prior_staff_id, prior_row)) = file_asset_claimants.get(&canon_key) {
                        if prior_staff_id.to_uppercase() != row.staff_id.to_uppercase() {
                            item.status = HistoricalAssignmentRowStatus::Conflict;
                            item.status_reason = Some(format!(
                                "Asset '{}' claimed by multiple employees in file (row {} and row {})",
                                canonical_code, prior_row, row.row_number
                            ));
                        }
                    } else {
                        file_asset_claimants.insert(canon_key, (row.staff_id.clone(), row.row_number));
                    }
                }
            }

            items.push(item);
        }
    }

    let total_rows = raw_rows.len();
    let total_assignments = items.len();
    let mut new_assignments = 0;
    let mut already_assigned = 0;
    let mut missing_assets = 0;
    let mut missing_employees = 0;
    let mut conflicts = 0;

    for item in &items {
        match item.status {
            HistoricalAssignmentRowStatus::NewAssignment => new_assignments += 1,
            HistoricalAssignmentRowStatus::AlreadyAssigned => already_assigned += 1,
            HistoricalAssignmentRowStatus::MissingAsset => missing_assets += 1,
            HistoricalAssignmentRowStatus::MissingEmployee => missing_employees += 1,
            HistoricalAssignmentRowStatus::Conflict => conflicts += 1,
        }
    }

    let summary = HistoricalAssignmentPreviewSummary {
        total_rows,
        total_assignments,
        new_assignments,
        already_assigned,
        missing_assets,
        missing_employees,
        conflicts,
        has_blocking_conflicts: conflicts > 0,
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

    if preview.summary.has_blocking_conflicts {
        return Err(format!(
            "Cannot approve import: {} unresolved conflict(s) exist. Review preview and correct conflicting rows before approving.",
            preview.summary.conflicts
        ));
    }

    let tx = conn
        .transaction()
        .map_err(|err| format!("Failed to begin transaction: {err}"))?;

    let mut created_assignments = 0;
    let mut skipped_already_assigned = 0;
    let mut errors = Vec::new();

    for item in &preview.items {
        match item.status {
            HistoricalAssignmentRowStatus::AlreadyAssigned => {
                skipped_already_assigned += 1;
            }
            HistoricalAssignmentRowStatus::NewAssignment => {
                let employee_row_id = item.employee_id.ok_or_else(|| {
                    format!("Missing employee ID for valid item row {}", item.row_number)
                })?;
                let staff_id = item.canonical_staff_id.as_deref().unwrap_or(&item.raw_staff_id);
                let full_name = item.canonical_employee_name.as_deref().unwrap_or(&item.raw_employee_name);
                let asset_id = item.asset_id.ok_or_else(|| {
                    format!("Missing asset ID for valid item row {}", item.row_number)
                })?;
                let asset_code = item.canonical_asset_code.as_deref().unwrap_or(&item.raw_asset_code);

                apply_single_historical_assignment_tx(
                    &tx,
                    asset_id,
                    asset_code,
                    employee_row_id,
                    staff_id,
                    full_name,
                    &item.location,
                    reviewer_account_id,
                )?;
                created_assignments += 1;
            }
            _ => {
                // Missing asset or missing employee are skipped with a noted message
                errors.push(format!(
                    "Skipped row {} asset '{}': {}",
                    item.row_number,
                    item.raw_asset_code,
                    item.status_reason.as_deref().unwrap_or("Unresolved item")
                ));
            }
        }
    }

    tx.commit()
        .map_err(|err| format!("Failed to commit historical assignment import transaction: {err}"))?;

    Ok(HistoricalAssignmentImportReport {
        total_processed: preview.items.len(),
        created_assignments,
        skipped_already_assigned,
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
            reviewer_account_id
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
        params![asset_id, employee_row_id, request_id, reviewer_account_id],
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
    fn test_parse_exact_asigned_asset_workbook() {
        let file_path = "00_ExSource/Asigned_Asset.xlsx";
        if !Path::new(file_path).exists() {
            return;
        }

        let (file_name, rows) = parse_assignment_sheet(file_path, None).expect("parse sheet");
        assert_eq!(file_name, "Asigned_Asset.xlsx");
        assert_eq!(rows.len(), 157, "Should have exactly 157 data rows");

        let mut total_assignments = 0;
        let mut unique_codes = HashSet::new();
        for r in &rows {
            total_assignments += r.cty_codes.len() + r.nha_codes.len();
            for c in &r.cty_codes {
                unique_codes.insert(c.clone());
            }
            for c in &r.nha_codes {
                unique_codes.insert(c.clone());
            }
        }
        assert_eq!(total_assignments, 200, "Should have exactly 200 total monitor assignments");
        assert_eq!(unique_codes.len(), 200, "Should have 200 unique monitor codes");

        // Verify row 7 (Lê Minh Duy) has MON770 at CTY, MON768 at NHA
        let row7 = &rows[5]; // 0-based index 5 is excel row 7
        assert_eq!(row7.staff_id, "ASWVN214");
        assert_eq!(row7.employee_name, "LÊ MINH DUY");
        assert_eq!(row7.cty_codes, vec!["MON770"]);
        assert_eq!(row7.nha_codes, vec!["MON768"]);
    }

    #[test]
    fn test_identity_conflicts_detected_and_blocked() {
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

        // Attempting to approve must fail and roll back with zero writes
        let apply_res = apply_historical_assignment_import_conn(
            &mut conn,
            &HistoricalAssignmentImportInput {
                file_path: file_path.to_string(),
                sheet_name: None,
            },
            None,
        );
        assert!(apply_res.is_err(), "Must block approval when conflicts exist");
        let active_loans: i64 = conn.query_row("SELECT count(*) FROM asset_loans", [], |r| r.get(0)).unwrap();
        assert_eq!(active_loans, 0, "Zero writes must occur when conflicts block import");
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

        let isolated_db_path = Path::new("scratch/isolated_test.sqlite3");
        if !isolated_db_path.exists() {
            return;
        }

        // 1. Verify Preview on isolated test DB copy without modifying it
        let read_conn = open_encrypted_read_only_connection(isolated_db_path).expect("open read-only isolated db");
        let preview = preview_historical_assignment_import_conn(
            &read_conn,
            &HistoricalAssignmentImportInput {
                file_path: "00_ExSource/Asigned_Asset.xlsx".to_string(),
                sheet_name: None,
            },
        ).expect("preview historical assignment import");

        assert_eq!(preview.summary.total_rows, 157);
        assert_eq!(preview.summary.total_assignments, 200);
        assert!(preview.summary.has_blocking_conflicts);
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
}
