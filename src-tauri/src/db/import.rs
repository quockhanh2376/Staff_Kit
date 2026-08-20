use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::{fs, str};

use calamine::{open_workbook_auto, Data, DataType, Reader};
use chrono::{Duration as ChronoDuration, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::column::upsert_dynamic_field_definitions_tx;
use super::employee::{
    load_employee_by_employee_id, load_employees_by_email_normalized, upsert_employee_from_payload,
    EmployeePayload, UpsertAction,
};
use super::schema::{
    COMPUTER_NAME_2_FIELD_KEY, COMPUTER_NAME_2_FIELD_LABEL, CORE_COLUMN_DEFINITIONS,
    STAFF_GROUP_EMPLOYEE_LIST, STAFF_GROUP_INTERNAL_MOVEMENT, STAFF_GROUP_OFFBOARDING,
    STAFF_GROUP_ONBOARDING,
};
use super::{
    ensure_database_ready, normalize_dynamic_field_key, normalize_optional_text,
    normalize_staff_group, open_runtime_connection,
};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportExcelInput {
    pub file_path: Option<String>,
    pub file_paths: Option<Vec<String>>,
    pub selected_column_keys: Option<Vec<String>>,
    pub target_staff_group: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportErrorItem {
    pub row: u32,
    pub employee_id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub source_file: String,
    pub source_files: Vec<String>,
    pub sheet_name: String,
    pub header_row: u32,
    pub processed_sheets: Vec<String>,
    pub total_rows: u32,
    pub inserted: u32,
    pub updated: u32,
    pub skipped: u32,
    pub failed: u32,
    pub errors: Vec<ImportErrorItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportColumnOption {
    pub key: String,
    pub label: String,
    pub source: String,
    pub required: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportColumnsPreview {
    pub source_files: Vec<String>,
    pub detected_columns: Vec<ImportColumnOption>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldChange {
    pub field_key: String,
    pub field_label: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreviewRow {
    pub row_number: u32,
    pub employee_id: String,
    pub full_name: String,
    pub is_update: bool,
    pub changes: Vec<FieldChange>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreviewResult {
    pub source_files: Vec<String>,
    pub sheet_name: String,
    pub preview_rows: Vec<ImportPreviewRow>,
    pub total_changes: u32,
    pub total_new: u32,
    pub total_updates: u32,
    pub errors: Vec<ImportErrorItem>,
}

#[derive(Debug, Clone)]
pub struct DynamicImportColumn {
    pub index: usize,
    pub field_key: String,
    pub field_label: String,
}

struct ImportColumns {
    pub employee_id: Option<usize>,
    pub full_name: Option<usize>,
    pub nick_name: Option<usize>,
    pub client_pmd: Option<usize>,
    pub project: Option<usize>,
    pub job_title: Option<usize>,
    pub email: Option<usize>,
    pub cellphone: Option<usize>,
    pub date_of_birth: Option<usize>,
    pub gender: Option<usize>,
    pub asw_start_date: Option<usize>,
    pub client_start_date: Option<usize>,
    pub contract_end_date: Option<usize>,
    pub client_year_of_services: Option<usize>,
    pub computer_name: Option<usize>,
    pub computer_name_secondary: Option<usize>,
    pub notes: Option<usize>,
    pub dynamic_columns: Vec<DynamicImportColumn>,
}

pub(crate) struct EmployeeHeaderEvidence {
    pub header_row: usize,
    pub match_key_headers: Vec<String>,
    pub full_name_headers: Vec<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn inspect_import_columns(
    app: &AppHandle,
    payload: ImportExcelInput,
) -> Result<ImportColumnsPreview, String> {
    ensure_database_ready(app)?;
    let source_paths = resolve_import_source_paths(payload.file_path, payload.file_paths)?;

    let mut detected_map: HashMap<String, ImportColumnOption> = HashMap::new();
    let mut source_files = Vec::new();

    for source_path in source_paths {
        source_files.push(source_path.to_string_lossy().to_string());

        let mut workbook = match open_workbook_auto(&source_path) {
            Ok(workbook) => workbook,
            Err(_) => continue,
        };

        let sheet_names = workbook.sheet_names().to_vec();
        for sheet_name in sheet_names {
            let range = match workbook.worksheet_range(&sheet_name) {
                Ok(range) => range,
                Err(_) => continue,
            };

            let (_, columns) = match detect_import_columns(&range) {
                Ok(value) => value,
                Err(_) => continue,
            };

            for option in collect_import_column_options(&columns) {
                detected_map.entry(option.key.clone()).or_insert(option);
            }
        }
    }

    if !detected_map.contains_key("employeeId") && !detected_map.contains_key("email") {
        return Err(
            "failed to detect required match column: need EE.ID or Working Email".to_string(),
        );
    }

    let mut detected_columns = detected_map.into_values().collect::<Vec<_>>();
    detected_columns.sort_by(|a, b| {
        b.required
            .cmp(&a.required)
            .then_with(|| a.source.cmp(&b.source))
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });

    Ok(ImportColumnsPreview {
        source_files,
        detected_columns,
    })
}

pub fn import_excel(app: &AppHandle, payload: ImportExcelInput) -> Result<ImportReport, String> {
    ensure_database_ready(app)?;
    let ImportExcelInput {
        file_path,
        file_paths,
        selected_column_keys,
        target_staff_group,
    } = payload;

    let source_paths = resolve_import_source_paths(file_path, file_paths)?;
    let selected_column_keys = normalize_selected_column_keys(selected_column_keys);
    let forced_staff_group = match normalize_optional_text(target_staff_group) {
        Some(raw_group) => Some(
            normalize_staff_group(raw_group.as_str())
                .ok_or_else(|| format!("invalid import target group: {raw_group}"))?,
        ),
        None => None,
    };

    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start import transaction: {err}"))?;

    let mut report = ImportReport {
        source_file: source_paths
            .first()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        source_files: source_paths
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>(),
        sheet_name: String::new(),
        header_row: 0,
        processed_sheets: Vec::new(),
        total_rows: 0,
        inserted: 0,
        updated: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
    };

    for source_path in source_paths {
        let source_name = source_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("workbook")
            .to_string();

        let mut workbook = open_workbook_auto(&source_path)
            .map_err(|err| format!("failed to open workbook '{}': {err}", source_path.display()))?;

        let sheet_names = workbook.sheet_names().to_vec();
        for sheet_name in sheet_names {
            let sheet_staff_group =
                infer_staff_group_from_source(source_name.as_str(), sheet_name.as_str());

            let range = match workbook.worksheet_range(&sheet_name) {
                Ok(range) => range,
                Err(_) => continue,
            };

            let (header_row_index, columns) = match detect_import_columns(&range) {
                Ok(value) => value,
                Err(_) => continue,
            };

            if report.sheet_name.is_empty() {
                report.sheet_name = format!("{source_name}::{sheet_name}");
                report.header_row = (header_row_index + 1) as u32;
            }
            report
                .processed_sheets
                .push(format!("{source_name}::{sheet_name}"));

            let selected_dynamic_columns = columns
                .dynamic_columns
                .iter()
                .filter(|column| column_selected(&selected_column_keys, column.field_key.as_str()))
                .cloned()
                .collect::<Vec<_>>();

            if !selected_dynamic_columns.is_empty() {
                upsert_dynamic_field_definitions_tx(&tx, &selected_dynamic_columns)?;
            }

            for (row_index, row) in range.rows().enumerate().skip(header_row_index + 1) {
                if row_is_empty(row) {
                    continue;
                }

                report.total_rows += 1;
                let row_number = (row_index + 1) as u32;
                let sheet_ref = format!("{source_name}::{sheet_name}");

                let parsed_employee_id = extract_optional_value(row, columns.employee_id)
                    .and_then(|value| normalize_employee_id(value).ok());
                let import_email = normalize_email(extract_optional_value(row, columns.email));

                let existing_by_id = if let Some(employee_id) = parsed_employee_id.as_deref() {
                    load_employee_by_employee_id(&tx, employee_id)?
                } else {
                    None
                };
                let matched_by_id = existing_by_id.is_some();

                let existing = if existing_by_id.is_some() {
                    existing_by_id
                } else if let Some(email) = import_email.as_deref() {
                    let matched_employees = load_employees_by_email_normalized(&tx, email)?;
                    if matched_employees.len() > 1 {
                        report.skipped += 1;
                        report.errors.push(ImportErrorItem {
                            row: row_number,
                            employee_id: None,
                            reason: format!(
                                "[{sheet_ref}] Working Email '{email}' matches multiple employees, please clean duplicated emails before import"
                            ),
                        });
                        continue;
                    }
                    matched_employees.into_iter().next()
                } else {
                    report.skipped += 1;
                    report.errors.push(ImportErrorItem {
                        row: row_number,
                        employee_id: None,
                        reason: format!(
                            "[{sheet_ref}] missing EE.ID and Working Email, cannot match existing employee"
                        ),
                    });
                    continue;
                };

                let existing_ref = existing.as_ref();
                let used_email_fallback = !matched_by_id && existing_ref.is_some();
                let employee_id = if used_email_fallback {
                    existing_ref
                        .map(|employee| employee.employee_id.clone())
                        .unwrap_or_default()
                } else if let Some(value) = parsed_employee_id.clone() {
                    value
                } else if let Some(existing_employee) = existing_ref {
                    existing_employee.employee_id.clone()
                } else {
                    report.skipped += 1;
                    report.errors.push(ImportErrorItem {
                        row: row_number,
                        employee_id: None,
                        reason: format!(
                            "[{sheet_ref}] EE.ID not found and no existing employee matches Working Email"
                        ),
                    });
                    continue;
                };

                let target_staff_group = if let Some(group) = forced_staff_group {
                    group
                } else {
                    existing_ref
                        .and_then(|employee| normalize_staff_group(employee.staff_group.as_str()))
                        .unwrap_or(sheet_staff_group)
                };

                let full_name_index = if column_selected(&selected_column_keys, "fullName") {
                    columns.full_name
                } else {
                    None
                };

                let full_name = extract_optional_value(row, full_name_index)
                    .or_else(|| existing_ref.map(|item| item.full_name.clone()));

                let Some(full_name) = full_name else {
                    report.skipped += 1;
                    report.errors.push(ImportErrorItem {
                        row: row_number,
                        employee_id: Some(employee_id),
                        reason: format!(
                            "[{sheet_ref}] missing full name and EE. ID does not exist in current table"
                        ),
                    });
                    continue;
                };

                let mut dynamic_fields = existing_ref
                    .map(|employee| employee.dynamic_fields.clone())
                    .unwrap_or_default();

                for column in &selected_dynamic_columns {
                    if let Some(value) = extract_optional_value(row, Some(column.index)) {
                        dynamic_fields.insert(column.field_key.clone(), value);
                    }
                }

                let existing_computer_1 = existing_ref.and_then(|item| item.computer_name.clone());
                let existing_computer_2 = existing_ref
                    .and_then(|item| item.dynamic_fields.get(COMPUTER_NAME_2_FIELD_KEY).cloned());

                let incoming_computer_1_raw = extract_optional_value(
                    row,
                    selected_column_index(
                        &selected_column_keys,
                        "computerName",
                        columns.computer_name,
                    ),
                );
                let incoming_computer_2_raw = extract_optional_value(
                    row,
                    selected_column_index(
                        &selected_column_keys,
                        COMPUTER_NAME_2_FIELD_KEY,
                        columns.computer_name_secondary,
                    ),
                );
                let (incoming_computer_1, incoming_computer_2) =
                    normalize_computer_name_slots(incoming_computer_1_raw, incoming_computer_2_raw);

                // If employee already has a computer and the new row brings a different value,
                // preserve slot 1 and push the new one to slot 2 (handles asset-list files where
                // one employee appears on multiple rows, each with a different machine name).
                let (mut computer_name_primary, mut computer_name_secondary) =
                    if let (Some(ref existing1), Some(ref incoming1)) =
                        (&existing_computer_1, &incoming_computer_1)
                    {
                        if existing1 != incoming1
                            && existing_computer_2.is_none()
                            && incoming_computer_2.is_none()
                        {
                            // Second machine for this employee: keep slot 1, put new one in slot 2
                            (existing_computer_1.clone(), Some(incoming1.clone()))
                        } else {
                            // Normal merge: prefer incoming value over existing
                            (
                                incoming_computer_1.or(existing_computer_1),
                                incoming_computer_2.or(existing_computer_2),
                            )
                        }
                    } else {
                        (
                            incoming_computer_1.or(existing_computer_1),
                            incoming_computer_2.or(existing_computer_2),
                        )
                    };

                if used_email_fallback {
                    if let Some(device_candidate) = parsed_employee_id.clone() {
                        let first = computer_name_primary.as_deref();
                        let second = computer_name_secondary.as_deref();
                        let already_exists = first == Some(device_candidate.as_str())
                            || second == Some(device_candidate.as_str());
                        if !already_exists {
                            if computer_name_primary.is_none() {
                                computer_name_primary = Some(device_candidate);
                            } else if computer_name_secondary.is_none() {
                                computer_name_secondary = Some(device_candidate);
                            } else {
                                computer_name_primary = Some(device_candidate);
                            }
                        }
                    }
                }

                let (computer_name, computer_name_2) =
                    normalize_computer_name_slots(computer_name_primary, computer_name_secondary);

                if let Some(value) = computer_name_2 {
                    dynamic_fields.insert(COMPUTER_NAME_2_FIELD_KEY.to_string(), value);
                }

                let payload = EmployeePayload {
                    employee_id: employee_id.clone(),
                    full_name,
                    nick_name: merge_import_text(
                        existing_ref.and_then(|item| item.nick_name.clone()),
                        row,
                        selected_column_index(&selected_column_keys, "nickName", columns.nick_name),
                    ),
                    team_name: merge_import_team_name(
                        existing_ref.and_then(|item| item.team_name.clone()),
                        row,
                        if column_selected(&selected_column_keys, "teamName") {
                            columns.client_pmd
                        } else {
                            None
                        },
                        if column_selected(&selected_column_keys, "teamName") {
                            columns.project
                        } else {
                            None
                        },
                    ),
                    project: merge_import_text(
                        existing_ref.and_then(|item| item.project.clone()),
                        row,
                        selected_column_index(&selected_column_keys, "project", columns.project),
                    ),
                    job_title: merge_import_text(
                        existing_ref.and_then(|item| item.job_title.clone()),
                        row,
                        selected_column_index(&selected_column_keys, "jobTitle", columns.job_title),
                    ),
                    email: merge_import_text(
                        existing_ref.and_then(|item| item.email.clone()),
                        row,
                        selected_column_index(&selected_column_keys, "email", columns.email),
                    ),
                    cellphone: merge_import_text(
                        existing_ref.and_then(|item| item.cellphone.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "cellphone",
                            columns.cellphone,
                        ),
                    ),
                    date_of_birth: merge_import_date(
                        existing_ref.and_then(|item| item.date_of_birth.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "dateOfBirth",
                            columns.date_of_birth,
                        ),
                    ),
                    gender: merge_import_text(
                        existing_ref.and_then(|item| item.gender.clone()),
                        row,
                        selected_column_index(&selected_column_keys, "gender", columns.gender),
                    ),
                    asw_start_date: merge_import_date(
                        existing_ref.and_then(|item| item.asw_start_date.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "aswStartDate",
                            columns.asw_start_date,
                        ),
                    ),
                    client_start_date: merge_import_date(
                        existing_ref.and_then(|item| item.client_start_date.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "clientStartDate",
                            columns.client_start_date,
                        ),
                    ),
                    contract_end_date: merge_import_optional_or_date(
                        existing_ref.and_then(|item| item.contract_end_date.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "contractEndDate",
                            columns.contract_end_date,
                        ),
                    ),
                    client_year_of_services: merge_import_text(
                        existing_ref.and_then(|item| item.client_year_of_services.clone()),
                        row,
                        selected_column_index(
                            &selected_column_keys,
                            "clientYearOfServices",
                            columns.client_year_of_services,
                        ),
                    ),
                    computer_name,
                    notes: merge_import_text(
                        existing_ref.and_then(|item| item.notes.clone()),
                        row,
                        selected_column_index(&selected_column_keys, "notes", columns.notes),
                    ),
                    staff_group: None,
                    dynamic_fields: if dynamic_fields.is_empty() {
                        None
                    } else {
                        Some(dynamic_fields)
                    },
                };

                match upsert_employee_from_payload(&tx, payload, target_staff_group) {
                    Ok(UpsertAction::Inserted) => report.inserted += 1,
                    Ok(UpsertAction::Updated) => report.updated += 1,
                    Err(err) => {
                        report.failed += 1;
                        report.errors.push(ImportErrorItem {
                            row: row_number,
                            employee_id: Some(employee_id),
                            reason: format!("[{sheet_ref}] {err}"),
                        });
                    }
                }
            }
        }
    }

    if report.processed_sheets.is_empty() {
        return Err(
            "failed to detect valid import sheets: need EE.ID or Working Email column".to_string(),
        );
    }

    tx.commit()
        .map_err(|err| format!("failed to commit import transaction: {err}"))?;

    Ok(report)
}

pub fn preview_import_excel(
    app: &AppHandle,
    payload: ImportExcelInput,
) -> Result<ImportPreviewResult, String> {
    ensure_database_ready(app)?;
    let ImportExcelInput {
        file_path,
        file_paths,
        selected_column_keys,
        target_staff_group,
    } = payload;

    let source_paths = resolve_import_source_paths(file_path, file_paths)?;
    let selected_column_keys = normalize_selected_column_keys(selected_column_keys);
    let forced_staff_group = match normalize_optional_text(target_staff_group) {
        Some(raw_group) => Some(
            normalize_staff_group(raw_group.as_str())
                .ok_or_else(|| format!("invalid import target group: {raw_group}"))?,
        ),
        None => None,
    };

    let conn = open_runtime_connection(app)?;
    let mut preview_result = ImportPreviewResult {
        source_files: source_paths
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
        sheet_name: String::new(),
        preview_rows: Vec::new(),
        total_changes: 0,
        total_new: 0,
        total_updates: 0,
        errors: Vec::new(),
    };

    for source_path in source_paths {
        let source_name = source_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("workbook")
            .to_string();

        let mut workbook = open_workbook_auto(&source_path)
            .map_err(|err| format!("failed to open workbook '{}': {err}", source_path.display()))?;

        let sheet_names = workbook.sheet_names().to_vec();
        for sheet_name in sheet_names {
            let sheet_staff_group =
                infer_staff_group_from_source(source_name.as_str(), sheet_name.as_str());

            let range = match workbook.worksheet_range(&sheet_name) {
                Ok(range) => range,
                Err(_) => continue,
            };

            let (header_row_index, columns) = match detect_import_columns(&range) {
                Ok(value) => value,
                Err(_) => continue,
            };

            let selected_dynamic_columns = columns
                .dynamic_columns
                .iter()
                .filter(|column| column_selected(&selected_column_keys, column.field_key.as_str()))
                .collect::<Vec<_>>();

            if preview_result.sheet_name.is_empty() {
                preview_result.sheet_name = sheet_name.clone();
            }

            let data_rows = range.rows().skip(header_row_index as usize + 1);
            let _target_staff_group = forced_staff_group.clone().or(Some(sheet_staff_group));

            for (row_index, row) in data_rows.enumerate() {
                let row_number = (header_row_index as usize + 2 + row_index) as u32;
                let mut changes = Vec::new();

                let parsed_employee_id = extract_optional_value(row, columns.employee_id)
                    .and_then(|value| normalize_optional_text(Some(value)));

                let existing = if let Some(id_value) = parsed_employee_id.clone() {
                    load_employee_by_employee_id(&conn, &id_value)
                        .ok()
                        .flatten()
                } else {
                    let email = extract_optional_value(row, columns.email)
                        .and_then(|value| normalize_optional_text(Some(value)))
                        .map(|e| e.to_lowercase());

                    if let Some(email_value) = email {
                        load_employees_by_email_normalized(&conn, &email_value)
                            .ok()
                            .and_then(|mut emps| emps.pop())
                    } else {
                        None
                    }
                };

                if existing.is_none() && parsed_employee_id.is_none() {
                    preview_result.errors.push(ImportErrorItem {
                        row: row_number,
                        employee_id: None,
                        reason: "missing EE.ID and Working Email".to_string(),
                    });
                    continue;
                }

                let matched_by_id = existing.is_some() && parsed_employee_id.is_some();
                let existing_ref = existing.as_ref();
                let used_email_fallback = !matched_by_id && existing_ref.is_some();
                let employee_id = if used_email_fallback {
                    existing_ref
                        .map(|e| e.employee_id.clone())
                        .unwrap_or_default()
                } else if let Some(id) = parsed_employee_id {
                    id
                } else {
                    preview_result.errors.push(ImportErrorItem {
                        row: row_number,
                        employee_id: None,
                        reason: "no matching employee found".to_string(),
                    });
                    continue;
                };

                let full_name = extract_optional_value(row, columns.full_name)
                    .or_else(|| existing_ref.map(|e| e.full_name.clone()));

                let Some(full_name) = full_name else {
                    preview_result.errors.push(ImportErrorItem {
                        row: row_number,
                        employee_id: Some(employee_id),
                        reason: "missing full name".to_string(),
                    });
                    continue;
                };

                let is_update = existing_ref.is_some();

                let (new_computer_name, new_computer_2) = normalize_computer_name_slots(
                    extract_optional_value(row, columns.computer_name),
                    extract_optional_value(row, columns.computer_name_secondary),
                );
                let old_computer_name = existing_ref.and_then(|e| e.computer_name.clone());
                let old_computer_2 = existing_ref
                    .and_then(|e| e.dynamic_fields.get(COMPUTER_NAME_2_FIELD_KEY).cloned());

                match (&old_computer_name, &new_computer_name) {
                    // Employee already has a computer, incoming is different and slot 2 is empty →
                    // this is a SECOND machine, not an overwrite of the first one.
                    (Some(old1), Some(new1)) if old1 != new1 && old_computer_2.is_none() => {
                        changes.push(FieldChange {
                            field_key: COMPUTER_NAME_2_FIELD_KEY.to_string(),
                            field_label: "Computer Name (2)".to_string(),
                            old_value: None,
                            new_value: new_computer_name,
                        });
                    }
                    // Normal case: new value is different from old, just report the change.
                    (old, Some(_)) if old != &new_computer_name => {
                        changes.push(FieldChange {
                            field_key: "computerName".to_string(),
                            field_label: "Computer Name (1)".to_string(),
                            old_value: old_computer_name,
                            new_value: new_computer_name,
                        });
                    }
                    _ => {}
                }

                if let Some(new2) = new_computer_2 {
                    if old_computer_2.as_deref() != Some(new2.as_str()) {
                        changes.push(FieldChange {
                            field_key: COMPUTER_NAME_2_FIELD_KEY.to_string(),
                            field_label: "Computer Name (2)".to_string(),
                            old_value: old_computer_2,
                            new_value: Some(new2),
                        });
                    }
                }

                let new_team = extract_optional_value(row, columns.client_pmd)
                    .and_then(|value| normalize_optional_text(Some(value)));
                let old_team = existing_ref.and_then(|e| e.team_name.clone());
                if new_team != old_team && new_team.is_some() {
                    changes.push(FieldChange {
                        field_key: "teamName".to_string(),
                        field_label: "Team".to_string(),
                        old_value: old_team,
                        new_value: new_team,
                    });
                }

                let new_project = extract_optional_value(row, columns.project)
                    .and_then(|value| normalize_optional_text(Some(value)));
                let old_project = existing_ref.and_then(|e| e.project.clone());
                if new_project != old_project && new_project.is_some() {
                    changes.push(FieldChange {
                        field_key: "project".to_string(),
                        field_label: "Project".to_string(),
                        old_value: old_project,
                        new_value: new_project,
                    });
                }

                for column in &selected_dynamic_columns {
                    let new_value = extract_optional_value(row, Some(column.index));
                    let old_value = existing_ref.and_then(|employee| {
                        employee.dynamic_fields.get(&column.field_key).cloned()
                    });
                    if new_value.is_some() && new_value != old_value {
                        changes.push(FieldChange {
                            field_key: column.field_key.clone(),
                            field_label: column.field_label.clone(),
                            old_value,
                            new_value,
                        });
                    }
                }

                let has_changes = !changes.is_empty();
                let change_count = changes.len() as u32;

                preview_result.preview_rows.push(ImportPreviewRow {
                    row_number,
                    employee_id,
                    full_name,
                    is_update,
                    changes,
                });

                if is_update && has_changes {
                    preview_result.total_updates += 1;
                    preview_result.total_changes += change_count;
                } else if !is_update {
                    preview_result.total_new += 1;
                }
            }
        }
    }

    Ok(preview_result)
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn normalize_selected_column_keys(input: Option<Vec<String>>) -> HashSet<String> {
    let mut keys = HashSet::new();
    let Some(items) = input else {
        return keys;
    };

    for item in items {
        let trimmed = item.trim();
        if !trimmed.is_empty() {
            keys.insert(trimmed.to_string());
        }
    }

    keys
}

fn column_selected(selected_keys: &HashSet<String>, key: &str) -> bool {
    selected_keys.is_empty() || selected_keys.contains(key)
}

fn selected_column_index(
    selected_keys: &HashSet<String>,
    key: &str,
    index: Option<usize>,
) -> Option<usize> {
    if column_selected(selected_keys, key) {
        index
    } else {
        None
    }
}

fn collect_import_column_options(columns: &ImportColumns) -> Vec<ImportColumnOption> {
    let mut options = Vec::new();

    if columns.employee_id.is_some() {
        options.push(ImportColumnOption {
            key: "employeeId".to_string(),
            label: "EE. ID".to_string(),
            source: "required".to_string(),
            required: true,
        });
    }

    if columns.employee_id.is_none() && columns.email.is_some() {
        options.push(ImportColumnOption {
            key: "email".to_string(),
            label: core_column_label("email"),
            source: "required".to_string(),
            required: true,
        });
    }

    let mut push_core = |key: &str, present: bool| {
        if !present {
            return;
        }
        options.push(ImportColumnOption {
            key: key.to_string(),
            label: core_column_label(key),
            source: "core".to_string(),
            required: false,
        });
    };

    push_core("fullName", columns.full_name.is_some());
    push_core("nickName", columns.nick_name.is_some());
    push_core(
        "teamName",
        columns.client_pmd.is_some() || columns.project.is_some(),
    );
    push_core("project", columns.project.is_some());
    push_core("jobTitle", columns.job_title.is_some());
    push_core("email", columns.email.is_some());
    push_core("cellphone", columns.cellphone.is_some());
    push_core("dateOfBirth", columns.date_of_birth.is_some());
    push_core("gender", columns.gender.is_some());
    push_core("aswStartDate", columns.asw_start_date.is_some());
    push_core("clientStartDate", columns.client_start_date.is_some());
    push_core("contractEndDate", columns.contract_end_date.is_some());
    push_core(
        "clientYearOfServices",
        columns.client_year_of_services.is_some(),
    );
    push_core("computerName", columns.computer_name.is_some());
    push_core("notes", columns.notes.is_some());

    if columns.computer_name_secondary.is_some() {
        options.push(ImportColumnOption {
            key: COMPUTER_NAME_2_FIELD_KEY.to_string(),
            label: COMPUTER_NAME_2_FIELD_LABEL.to_string(),
            source: "dynamic".to_string(),
            required: false,
        });
    }

    for dynamic in &columns.dynamic_columns {
        if dynamic.field_key == COMPUTER_NAME_2_FIELD_KEY {
            continue;
        }
        options.push(ImportColumnOption {
            key: dynamic.field_key.clone(),
            label: dynamic.field_label.clone(),
            source: "dynamic".to_string(),
            required: false,
        });
    }

    options
}

fn core_column_label(key: &str) -> String {
    CORE_COLUMN_DEFINITIONS
        .iter()
        .find_map(|(item_key, item_label)| {
            if *item_key == key {
                Some((*item_label).to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| key.to_string())
}

fn merge_import_text(
    existing: Option<String>,
    row: &[Data],
    index: Option<usize>,
) -> Option<String> {
    match index {
        Some(_) => extract_optional_value(row, index).or(existing),
        None => existing,
    }
}

fn merge_import_date(
    existing: Option<String>,
    row: &[Data],
    index: Option<usize>,
) -> Option<String> {
    match index {
        Some(_) => extract_date_value(row, index).or(existing),
        None => existing,
    }
}

fn merge_import_optional_or_date(
    existing: Option<String>,
    row: &[Data],
    index: Option<usize>,
) -> Option<String> {
    match index {
        Some(_) => extract_optional_or_date_value(row, index).or(existing),
        None => existing,
    }
}

fn merge_import_team_name(
    existing: Option<String>,
    row: &[Data],
    client_pmd_index: Option<usize>,
    project_index: Option<usize>,
) -> Option<String> {
    let imported = extract_optional_value(row, client_pmd_index)
        .or_else(|| extract_optional_value(row, project_index));
    imported.or(existing)
}

fn normalize_computer_name_slots(
    computer_name_1: Option<String>,
    computer_name_2: Option<String>,
) -> (Option<String>, Option<String>) {
    let mut slots = Vec::new();
    for value in [computer_name_1, computer_name_2].into_iter().flatten() {
        for token in value.split(|ch| matches!(ch, ',' | '\n' | '\r')) {
            let Some(token) = normalize_optional_text(Some(token.to_string())) else {
                continue;
            };
            if !slots
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(&token))
            {
                slots.push(token);
            }
            if slots.len() == 2 {
                break;
            }
        }
        if slots.len() == 2 {
            break;
        }
    }

    (slots.first().cloned(), slots.get(1).cloned())
}

fn detect_import_columns(range: &calamine::Range<Data>) -> Result<(usize, ImportColumns), String> {
    const HEADER_EMPLOYEE_ID: &[&str] = &[
        "eeid",
        "emid",
        "employeeid",
        "employeecode",
        "staffid",
        "staffcode",
        "staffcodeid",
        "manhanvien",
        "m\u{00E3}nh\u{00E2}nvi\u{00EA}n",
        "m\u{00E3}nv",
        "manv",
    ];
    const HEADER_FULL_NAME: &[&str] = &[
        "vietnamesename",
        "fullname",
        "yourfullname",
        "hoten",
        "h\u{1ECD}t\u{00EA}n",
        "name",
        "englishname",
    ];
    const HEADER_ASW_START_DATE: &[&str] = &["aswstartdate", "aswigstartdate", "startdate"];
    const HEADER_CLIENT_START_DATE: &[&str] = &[
        "clientstartdate",
        "currentclientstartdate",
        "newclientpmdjoindate",
        "lastclientstartdate",
    ];
    const HEADER_NICK_NAME: &[&str] = &["nickname", "nick"];
    const HEADER_CLIENT_PMD: &[&str] = &[
        "clientpmd",
        "newclientpmd",
        "formerclientpmd",
        "formerclient",
        "newclient",
        "client",
        "team",
        "department",
    ];
    const HEADER_PROJECT: &[&str] = &["project", "projectdeprt", "projectdept"];
    const HEADER_JOB_TITLE: &[&str] = &[
        "currentjobtitle",
        "offeredjobtitle",
        "lastedjobtitle",
        "newjobtitle",
        "startingtitlee",
        "jobtitle",
        "title",
    ];
    const HEADER_EMAIL: &[&str] = &[
        "workingemail",
        "formerworkingemail",
        "newworkingemail",
        "personalemail",
        "email",
    ];
    const HEADER_CELLPHONE: &[&str] = &[
        "cellphone",
        "phone",
        "mobilenumber",
        "mobile",
        "phonenumber",
    ];
    const HEADER_DOB: &[&str] = &["dob", "dateofbirth", "birthday", "yob", "yearofbirth"];
    const HEADER_GENDER: &[&str] = &["gender", "sex"];
    const HEADER_CONTRACT_END: &[&str] = &[
        "contractenddate",
        "aswlwd",
        "aswenddate",
        "formerenddate",
        "enddate",
    ];
    const HEADER_CLIENT_YOS: &[&str] = &[
        "clientyearofservices",
        "yearofservices",
        "services",
        "formerservices",
    ];
    const HEADER_COMPUTER_NAME: &[&str] = &[
        "computername",
        "computer",
        "tenmay",
        "computername1",
        "computer1",
    ];
    const HEADER_COMPUTER_NAME_SECONDARY: &[&str] = &[
        "computername2",
        "computer2",
        "computername02",
        "computername2nd",
    ];
    const HEADER_NOTES: &[&str] = &["notes", "note", "ghichu", "remark", "remarkdetails"];

    for (row_index, row) in range.rows().enumerate().take(120) {
        let mut headers: HashMap<String, usize> = HashMap::new();
        let mut header_entries: Vec<(usize, String, String)> = Vec::new();

        for (column_index, cell) in row.iter().enumerate() {
            let raw_label = cell_to_string(cell);
            if raw_label.trim().is_empty() {
                continue;
            }

            let key = normalize_header_key(&raw_label);
            if key.is_empty() {
                continue;
            }

            headers.insert(key.clone(), column_index);
            header_entries.push((column_index, raw_label.trim().to_string(), key));
        }

        let employee_id = find_column_index(&headers, HEADER_EMPLOYEE_ID);
        let email = find_column_index(&headers, HEADER_EMAIL);

        if employee_id.is_none() && email.is_none() {
            continue;
        }

        let full_name = find_column_index(&headers, HEADER_FULL_NAME);
        let asw_start_date = find_column_index(&headers, HEADER_ASW_START_DATE);
        let client_start_date = find_column_index(&headers, HEADER_CLIENT_START_DATE);
        let nick_name = find_column_index(&headers, HEADER_NICK_NAME);
        let client_pmd = find_column_index(&headers, HEADER_CLIENT_PMD);
        let project = find_column_index(&headers, HEADER_PROJECT);
        let job_title = find_column_index(&headers, HEADER_JOB_TITLE);
        let cellphone = find_column_index(&headers, HEADER_CELLPHONE);
        let date_of_birth = find_column_index(&headers, HEADER_DOB);
        let gender = find_column_index(&headers, HEADER_GENDER);
        let contract_end_date = find_column_index(&headers, HEADER_CONTRACT_END);
        let client_year_of_services = find_column_index(&headers, HEADER_CLIENT_YOS);
        let computer_name = find_column_index(&headers, HEADER_COMPUTER_NAME);
        let computer_name_secondary = find_column_index(&headers, HEADER_COMPUTER_NAME_SECONDARY);
        let notes = find_column_index(&headers, HEADER_NOTES);

        let known_indexes = [
            employee_id,
            full_name,
            asw_start_date,
            client_start_date,
            nick_name,
            client_pmd,
            project,
            job_title,
            email,
            cellphone,
            date_of_birth,
            gender,
            contract_end_date,
            client_year_of_services,
            computer_name,
            computer_name_secondary,
            notes,
        ]
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>();

        let mut seen_dynamic_keys = HashSet::new();
        let mut dynamic_columns = Vec::new();
        for (index, label, header_key) in &header_entries {
            if known_indexes.contains(index) {
                continue;
            }

            if should_skip_dynamic_import_column(header_key) {
                continue;
            }

            let key = normalize_dynamic_field_key(label);
            if key.is_empty() || !seen_dynamic_keys.insert(key.clone()) {
                continue;
            }

            dynamic_columns.push(DynamicImportColumn {
                index: *index,
                field_key: key,
                field_label: label.clone(),
            });
        }

        let columns = ImportColumns {
            asw_start_date,
            employee_id,
            client_start_date,
            full_name,
            nick_name,
            client_pmd,
            project,
            job_title,
            email,
            cellphone,
            date_of_birth,
            gender,
            contract_end_date,
            client_year_of_services,
            computer_name,
            computer_name_secondary,
            notes,
            dynamic_columns,
        };

        return Ok((row_index, columns));
    }

    Err("failed to detect import header row: need EE.ID or Working Email column".to_string())
}

pub(crate) fn detect_employee_header_evidence(
    range: &calamine::Range<Data>,
) -> Option<EmployeeHeaderEvidence> {
    let (header_row, columns) = detect_import_columns(range).ok()?;
    let headers = range
        .rows()
        .nth(header_row)
        .map(|row| row.iter().map(cell_to_string).collect::<Vec<_>>())?;

    let match_key_indexes = [columns.employee_id, columns.email];
    let match_key_headers = match_key_indexes
        .into_iter()
        .flatten()
        .filter_map(|index| headers.get(index).cloned())
        .collect::<Vec<_>>();
    let full_name_headers = columns
        .full_name
        .and_then(|index| headers.get(index).cloned())
        .into_iter()
        .collect::<Vec<_>>();

    if match_key_headers.is_empty() || full_name_headers.is_empty() {
        return None;
    }

    Some(EmployeeHeaderEvidence {
        header_row,
        match_key_headers,
        full_name_headers,
    })
}

pub(crate) fn infer_employee_staff_group(sheet_name: &str, source_name: &str) -> &'static str {
    let normalized_sheet = normalize_header_key(sheet_name);
    if normalized_sheet.contains("onboarding") {
        return STAFF_GROUP_ONBOARDING;
    }
    if normalized_sheet.contains("offboarding") {
        return STAFF_GROUP_OFFBOARDING;
    }
    if normalized_sheet.contains("internalmovement")
        || normalized_sheet.contains("internalmovent")
        || normalized_sheet.contains("internalmove")
    {
        return STAFF_GROUP_INTERNAL_MOVEMENT;
    }
    if normalized_sheet.contains("employeelist") || normalized_sheet.contains("eelist") {
        return STAFF_GROUP_EMPLOYEE_LIST;
    }

    infer_staff_group_from_source(source_name, "")
}

fn should_skip_dynamic_import_column(header_key: &str) -> bool {
    header_key == "question" || header_key.contains("ctyaswhitevn")
}

fn infer_staff_group_from_source(source_name: &str, sheet_name: &str) -> &'static str {
    let normalized = normalize_header_key(format!("{source_name} {sheet_name}").as_str());

    if normalized.contains("onboarding") {
        return STAFF_GROUP_ONBOARDING;
    }
    if normalized.contains("offboarding") {
        return STAFF_GROUP_OFFBOARDING;
    }
    if normalized.contains("internalmovement")
        || normalized.contains("internalmovent")
        || normalized.contains("internalmove")
    {
        return STAFF_GROUP_INTERNAL_MOVEMENT;
    }
    if normalized.contains("employeelist") || normalized.contains("eelist") {
        return STAFF_GROUP_EMPLOYEE_LIST;
    }

    STAFF_GROUP_EMPLOYEE_LIST
}

fn find_column_index(headers: &HashMap<String, usize>, aliases: &[&str]) -> Option<usize> {
    aliases
        .iter()
        .find_map(|alias| headers.get(*alias).copied())
}

fn normalize_header_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect::<String>()
}

fn resolve_import_source_paths(
    requested: Option<String>,
    requested_many: Option<Vec<String>>,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    let mut seen_paths: HashSet<String> = HashSet::new();

    if let Some(raw_paths) = requested_many {
        for raw_path in raw_paths {
            let Some(path_text) = normalize_optional_text(Some(raw_path)) else {
                continue;
            };

            let candidate = PathBuf::from(path_text);
            if !candidate.exists() {
                return Err(format!(
                    "import source file does not exist: {}",
                    candidate.display()
                ));
            }

            if !is_excel_file(&candidate) {
                continue;
            }

            let key = candidate.to_string_lossy().to_string();
            if seen_paths.insert(key) {
                paths.push(candidate);
            }
        }
    }

    if let Some(path_text) = normalize_optional_text(requested) {
        let candidate = PathBuf::from(path_text);
        if !candidate.exists() {
            return Err(format!(
                "import source file does not exist: {}",
                candidate.display()
            ));
        }

        if is_excel_file(&candidate) {
            let key = candidate.to_string_lossy().to_string();
            if seen_paths.insert(key) {
                paths.push(candidate);
            }
        }
    }

    if !paths.is_empty() {
        return Ok(paths);
    }

    let cwd = std::env::current_dir()
        .map_err(|err| format!("failed to resolve current directory for import: {err}"))?;

    let mut candidate_dirs = vec![cwd.join("ExSource"), cwd.join("Exsource")];
    if let Some(parent) = cwd.parent() {
        candidate_dirs.push(parent.join("ExSource"));
        candidate_dirs.push(parent.join("Exsource"));
    }

    let mut excel_files: Vec<PathBuf> = Vec::new();

    for dir in candidate_dirs {
        if !dir.exists() {
            continue;
        }

        let entries = fs::read_dir(&dir)
            .map_err(|err| format!("failed to read import directory '{}': {err}", dir.display()))?;

        for entry in entries {
            let entry =
                entry.map_err(|err| format!("failed to read import directory entry: {err}"))?;
            let path = entry.path();
            if is_excel_file(&path) && seen_paths.insert(path.to_string_lossy().to_string()) {
                excel_files.push(path);
            }
        }
    }

    if excel_files.is_empty() {
        return Err(
            "no Excel source found under ExSource/ or Exsource/. Provide filePath(s) or place .xlsx in that folder"
                .to_string(),
        );
    }

    excel_files.sort_by_key(|path| {
        fs::metadata(path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
    });

    Ok(vec![excel_files.pop().ok_or_else(|| {
        "no Excel source found under ExSource/".to_string()
    })?])
}

fn is_excel_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };

    matches!(ext.to_ascii_lowercase().as_str(), "xlsx" | "xlsm" | "xls")
}

fn row_is_empty(row: &[Data]) -> bool {
    row.iter().all(|cell| cell_to_string(cell).is_empty())
}

fn extract_optional_value(row: &[Data], index: Option<usize>) -> Option<String> {
    index
        .and_then(|idx| row.get(idx))
        .map(cell_to_string)
        .and_then(|value| normalize_optional_text(Some(value)))
}

fn extract_date_value(row: &[Data], index: Option<usize>) -> Option<String> {
    let cell = index.and_then(|idx| row.get(idx))?;

    if let Some(number) = cell.get_float() {
        if let Some(date_value) = excel_serial_to_iso(number) {
            return Some(date_value);
        }
    }

    if let Some(number) = cell.get_int() {
        if let Some(date_value) = excel_serial_to_iso(number as f64) {
            return Some(date_value);
        }
    }

    normalize_date_value_for_import(Some(cell_to_string(cell)))
}

fn extract_optional_or_date_value(row: &[Data], index: Option<usize>) -> Option<String> {
    let value = extract_optional_value(row, index)?;
    normalize_date_text_for_import(&value).or(Some(value))
}

fn normalize_employee_id(value: String) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "0" {
        return Err("employeeId is required".to_string());
    }
    Ok(trimmed.to_uppercase())
}

fn normalize_email(value: Option<String>) -> Option<String> {
    normalize_optional_text(value).map(|email| email.to_lowercase())
}

fn normalize_date_value_for_import(value: Option<String>) -> Option<String> {
    let normalized = normalize_optional_text(value)?;
    normalize_date_text_for_import(&normalized).or(Some(normalized))
}

fn normalize_date_text_for_import(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let date_patterns = [
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%d/%m/%Y",
        "%d-%m-%Y",
        "%m/%d/%Y",
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
    ];

    for pattern in date_patterns {
        if let Ok(date) = NaiveDate::parse_from_str(trimmed, pattern) {
            return Some(date.format("%Y-%m-%d").to_string());
        }
        if let Ok(date_time) = NaiveDateTime::parse_from_str(trimmed, pattern) {
            return Some(date_time.date().format("%Y-%m-%d").to_string());
        }
    }

    NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|date_time| date_time.date().format("%Y-%m-%d").to_string())
}

fn excel_serial_to_iso(serial: f64) -> Option<String> {
    if !serial.is_finite() {
        return None;
    }

    let days = serial.trunc();
    if days < 1.0 {
        return None;
    }

    let base = NaiveDate::from_ymd_opt(1899, 12, 30)?.and_hms_opt(0, 0, 0)?;
    let seconds = ((serial - days) * 86_400.0).round() as i64;
    let date_time = base + ChronoDuration::days(days as i64) + ChronoDuration::seconds(seconds);

    Some(date_time.date().format("%Y-%m-%d").to_string())
}

fn cell_to_string(cell: &Data) -> String {
    if let Some(value) = cell.get_string() {
        return value.trim().to_string();
    }

    if let Some(value) = cell.get_float() {
        return format_numeric(value);
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

fn format_numeric(value: f64) -> String {
    let rounded = value.round();
    if (value - rounded).abs() < f64::EPSILON {
        return format!("{rounded:.0}");
    }

    let mut out = format!("{value}");
    if out.contains('.') {
        while out.ends_with('0') {
            out.pop();
        }
        if out.ends_with('.') {
            out.pop();
        }
    }

    out
}
