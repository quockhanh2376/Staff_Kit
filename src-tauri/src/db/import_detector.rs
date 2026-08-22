use std::path::Path;

use calamine::{open_workbook_auto, Data, Range, Reader};
use serde::Serialize;

use super::asset_import::detect_asset_header_evidence;
use super::import::{detect_employee_header_evidence, infer_employee_staff_group};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportDetectionType {
    Employee,
    Asset,
    Unknown,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportDetectionSubtype {
    EmployeeList,
    Onboarding,
    Offboarding,
    InternalMovement,
    Serialized,
    Quantity,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDetectionResult {
    #[serde(rename = "type")]
    pub kind: ImportDetectionType,
    pub subtype: Option<ImportDetectionSubtype>,
    pub confidence: f32,
    pub sheet_name: Option<String>,
    pub header_row: Option<u32>,
    pub row_count: u32,
    pub evidence_headers: Vec<String>,
    pub reason: String,
    pub warnings: Vec<String>,
    pub candidate_types: Vec<String>,
}

struct SheetDetection {
    result: ImportDetectionResult,
    employee_evidence: bool,
    canonical_serialized: bool,
    quantity: bool,
    legacy_manual: bool,
}

pub fn detect_import_file(path: &Path) -> Result<ImportDetectionResult, String> {
    if !path.exists() {
        return Err(format!(
            "import source file does not exist: {}",
            path.display()
        ));
    }

    let mut workbook = open_workbook_auto(path)
        .map_err(|err| format!("failed to open import source '{}': {err}", path.display()))?;
    let source_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workbook");
    let sheets = workbook
        .sheet_names()
        .to_vec()
        .into_iter()
        .filter_map(|sheet_name| {
            workbook
                .worksheet_range(&sheet_name)
                .ok()
                .map(|range| (sheet_name, range))
        })
        .collect::<Vec<_>>();

    detect_import_ranges(source_name, sheets)
}

pub(crate) fn detect_import_ranges(
    source_name: &str,
    sheets: Vec<(String, Range<Data>)>,
) -> Result<ImportDetectionResult, String> {
    let detections = sheets
        .iter()
        .filter_map(|(sheet_name, range)| detect_sheet(source_name, sheet_name, range))
        .collect::<Vec<_>>();

    if detections.is_empty() {
        return Ok(unknown_result(
            None,
            "No recognizable Employee or Asset import columns were found. Provide Staff ID/EE.ID with Full Name, or Asset Tag with Asset Name and Category.",
        ));
    }

    let employee = detections.iter().find(|item| item.employee_evidence);
    let serialized = detections.iter().find(|item| item.canonical_serialized);
    let quantity = detections.iter().find(|item| item.quantity);

    let mut candidate_types = Vec::new();
    if employee.is_some() {
        candidate_types.push("employee".to_string());
    }
    if serialized.is_some() {
        candidate_types.push("asset:serialized".to_string());
    }
    if quantity.is_some() {
        candidate_types.push("asset:quantity".to_string());
    }
    if detections.iter().any(|item| item.legacy_manual) {
        candidate_types.push("asset:serialized_legacy_manual_confirmation".to_string());
    }

    if candidate_types.len() > 1 {
        let evidence = employee
            .or(serialized)
            .or(quantity)
            .expect("candidate exists");
        return Ok(ImportDetectionResult {
            kind: ImportDetectionType::Ambiguous,
            subtype: None,
            confidence: 0.0,
            sheet_name: evidence.result.sheet_name.clone(),
            header_row: evidence.result.header_row,
            row_count: evidence.result.row_count,
            evidence_headers: evidence.result.evidence_headers.clone(),
            reason: "The workbook contains evidence for more than one import type; choose the destination explicitly after review.".to_string(),
            warnings: vec!["No import route was selected automatically.".to_string()],
            candidate_types,
        });
    }

    if let Some(detection) = employee.or(serialized).or(quantity) {
        return Ok(detection.result.clone());
    }

    let legacy = detections.iter().find(|item| {
        item.result
            .warnings
            .iter()
            .any(|warning| warning.contains("manual"))
    });
    if let Some(detection) = legacy {
        return Ok(detection.result.clone());
    }

    Ok(unknown_result(
        detections.first().and_then(|item| item.result.sheet_name.clone()),
        "The workbook has partial import-like columns but is missing canonical required evidence. Check the required Employee or Asset headers before continuing.",
    ))
}

fn detect_sheet(
    source_name: &str,
    sheet_name: &str,
    range: &Range<Data>,
) -> Option<SheetDetection> {
    let employee = detect_employee_header_evidence(range);
    let asset = detect_asset_header_evidence(range);

    let employee_evidence = employee.is_some();
    let canonical_serialized = asset.as_ref().is_some_and(|e| {
        e.explicit_display_name
            && e.mapping.asset_code.is_some()
            && e.mapping.asset_type.is_some()
            && e.mapping.display_name.is_some()
    });
    let quantity = asset.as_ref().is_some_and(|e| {
        e.mapping.asset_code.is_none()
            && e.explicit_display_name
            && e.mapping.asset_type.is_some()
            && e.mapping.display_name.is_some()
            && e.mapping.quantity.is_some()
    });
    let legacy_manual = asset.as_ref().is_some_and(|e| {
        e.mapping.asset_type.is_some()
            && e.mapping.display_name.is_some()
            && (e.mapping.computer_name.is_some() || e.legacy_asset_id_header.is_some())
            && !canonical_serialized
            && !quantity
    });

    if employee_evidence {
        let evidence = employee.expect("employee evidence exists");
        let subtype = infer_employee_subtype(sheet_name, source_name);
        let mut evidence_headers = evidence.match_key_headers;
        evidence_headers.extend(evidence.full_name_headers);
        return Some(SheetDetection {
            result: ImportDetectionResult {
                kind: ImportDetectionType::Employee,
                subtype: Some(subtype),
                confidence: 0.95,
                sheet_name: Some(sheet_name.to_string()),
                header_row: Some((evidence.header_row + 1) as u32),
                row_count: data_row_count(range, evidence.header_row),
                evidence_headers,
                reason: "Employee match-key and name headers were detected.".to_string(),
                warnings: Vec::new(),
                candidate_types: vec!["employee".to_string()],
            },
            employee_evidence,
            canonical_serialized,
            quantity,
            legacy_manual,
        });
    }

    let asset = asset?;
    let mut evidence_headers = Vec::new();
    if let Some(header) = asset.mapping.asset_code.clone() {
        evidence_headers.push(header);
    }
    if let Some(header) = asset.mapping.display_name.clone() {
        evidence_headers.push(header);
    }
    if let Some(header) = asset.mapping.asset_type.clone() {
        evidence_headers.push(header);
    }
    if let Some(header) = asset.mapping.quantity.clone() {
        evidence_headers.push(header);
    }
    if let Some(header) = asset.mapping.computer_name.clone() {
        evidence_headers.push(header);
    }

    if canonical_serialized {
        return Some(SheetDetection {
            result: ImportDetectionResult {
                kind: ImportDetectionType::Asset,
                subtype: Some(ImportDetectionSubtype::Serialized),
                confidence: 0.98,
                sheet_name: Some(sheet_name.to_string()),
                header_row: Some((asset.header_row + 1) as u32),
                row_count: data_row_count(range, asset.header_row),
                evidence_headers,
                reason: "Canonical serialized asset headers Asset Tag, Asset Name and Category were detected.".to_string(),
                warnings: Vec::new(),
                candidate_types: vec!["asset:serialized".to_string()],
            },
            employee_evidence,
            canonical_serialized,
            quantity,
            legacy_manual,
        });
    }

    if quantity {
        return Some(SheetDetection {
            result: ImportDetectionResult {
                kind: ImportDetectionType::Asset,
                subtype: Some(ImportDetectionSubtype::Quantity),
                confidence: 0.95,
                sheet_name: Some(sheet_name.to_string()),
                header_row: Some((asset.header_row + 1) as u32),
                row_count: data_row_count(range, asset.header_row),
                evidence_headers,
                reason: "Quantity asset headers Category, Item Name/Asset Name and Quantity were detected.".to_string(),
                warnings: Vec::new(),
                candidate_types: vec!["asset:quantity".to_string()],
            },
            employee_evidence,
            canonical_serialized,
            quantity,
            legacy_manual,
        });
    }

    let legacy = asset.mapping.asset_type.is_some()
        && asset.mapping.display_name.is_some()
        && (asset.mapping.computer_name.is_some() || asset.legacy_asset_id_header.is_some());
    if legacy {
        return Some(SheetDetection {
            result: ImportDetectionResult {
                kind: ImportDetectionType::Ambiguous,
                subtype: None,
                confidence: 0.45,
                sheet_name: Some(sheet_name.to_string()),
                header_row: Some((asset.header_row + 1) as u32),
                row_count: data_row_count(range, asset.header_row),
                evidence_headers,
                reason: "Legacy asset columns were detected, but canonical Asset Tag evidence is missing.".to_string(),
                warnings: vec!["Legacy serialized asset candidate requires manual confirmation; it will not be auto-routed.".to_string()],
                candidate_types: vec!["asset:serialized_legacy_manual_confirmation".to_string()],
            },
            employee_evidence,
            canonical_serialized,
            quantity,
            legacy_manual,
        });
    }

    None
}

fn data_row_count(range: &Range<Data>, header_row: usize) -> u32 {
    range
        .rows()
        .skip(header_row + 1)
        .filter(|row| row.iter().any(|cell| !cell.to_string().trim().is_empty()))
        .count() as u32
}

fn infer_employee_subtype(sheet_name: &str, source_name: &str) -> ImportDetectionSubtype {
    let group = infer_employee_staff_group(sheet_name, source_name);

    match group {
        "onboarding" => ImportDetectionSubtype::Onboarding,
        "offboarding" => ImportDetectionSubtype::Offboarding,
        "internal_movement" => ImportDetectionSubtype::InternalMovement,
        _ => ImportDetectionSubtype::EmployeeList,
    }
}

fn unknown_result(sheet_name: Option<String>, reason: &str) -> ImportDetectionResult {
    ImportDetectionResult {
        kind: ImportDetectionType::Unknown,
        subtype: None,
        confidence: 0.0,
        sheet_name,
        header_row: None,
        row_count: 0,
        evidence_headers: Vec::new(),
        reason: reason.to_string(),
        warnings: Vec::new(),
        candidate_types: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use calamine::{Cell, Data, Range};

    use super::{
        detect_import_file, detect_import_ranges, ImportDetectionSubtype, ImportDetectionType,
    };

    fn sheet(name: &str, rows: &[&[&str]]) -> (String, Range<Data>) {
        let cells = rows
            .iter()
            .enumerate()
            .flat_map(|(row_index, row)| {
                row.iter().enumerate().map(move |(column_index, value)| {
                    Cell::new(
                        (row_index as u32, column_index as u32),
                        Data::String((*value).to_string()),
                    )
                })
            })
            .collect();

        (name.to_string(), Range::from_sparse(cells))
    }

    fn detect(source: &str, rows: &[&[&str]]) -> super::ImportDetectionResult {
        detect_import_ranges(source, vec![sheet("Data", rows)]).expect("detect fixture workbook")
    }

    #[test]
    fn detects_onboarding_employee_workbook() {
        let result = detect(
            "Onboarding.xlsx",
            &[&["EE.ID", "Full Name"], &["ASWVN001", "A"]],
        );

        assert_eq!(result.kind, ImportDetectionType::Employee);
        assert_eq!(result.subtype, Some(ImportDetectionSubtype::Onboarding));
        assert_eq!(result.header_row, Some(1));
        assert!(result.evidence_headers.contains(&"EE.ID".to_string()));
        assert!(result.evidence_headers.contains(&"Full Name".to_string()));
    }

    #[test]
    fn selects_the_valid_employee_sheet_from_a_multi_sheet_workbook() {
        let result = detect_import_ranges(
            "employees.xlsx",
            vec![
                sheet("Notes", &[&["Comment"], &["ignore this sheet"]]),
                sheet(
                    "Onboarding",
                    &[&["EE.ID", "Full Name"], &["ASWVN001", "A"]],
                ),
            ],
        )
        .expect("detect multi-sheet workbook");

        assert_eq!(result.kind, ImportDetectionType::Employee);
        assert_eq!(result.sheet_name.as_deref(), Some("Onboarding"));
        assert_eq!(result.row_count, 1);
    }

    #[test]
    fn sheet_name_takes_precedence_over_filename_hint() {
        let workbook = vec![sheet(
            "EE List",
            &[&["EE.ID", "Full Name"], &["ASWVN001", "A"]],
        )];
        let result = detect_import_ranges("Onboarding.xlsx", workbook).expect("detect workbook");

        assert_eq!(result.kind, ImportDetectionType::Employee);
        assert_eq!(result.subtype, Some(ImportDetectionSubtype::EmployeeList));
    }

    #[test]
    fn detects_offboarding_and_internal_movement_employee_shapes() {
        let offboarding = detect_import_ranges(
            "staff.xlsx",
            vec![sheet(
                "Offboarding",
                &[&["Staff ID", "Name"], &["ASWVN001", "A"]],
            )],
        )
        .expect("detect offboarding workbook");
        assert_eq!(
            offboarding.subtype,
            Some(ImportDetectionSubtype::Offboarding)
        );

        let movement = detect(
            "Internal Movement.xlsx",
            &[&["Staff ID", "Name"], &["ASWVN001", "A"]],
        );
        assert_eq!(
            movement.subtype,
            Some(ImportDetectionSubtype::InternalMovement)
        );
    }

    #[test]
    fn detects_canonical_serialized_asset() {
        let result = detect(
            "AssetList.xlsx",
            &[
                &["Asset Tag", "Asset Name", "Category", "Serial Number"],
                &["VNLAP001", "Laptop A", "Laptop", ""],
            ],
        );

        assert_eq!(result.kind, ImportDetectionType::Asset);
        assert_eq!(result.subtype, Some(ImportDetectionSubtype::Serialized));
        assert!(result.confidence >= 0.9);
    }

    #[test]
    fn detects_quantity_asset_without_confusing_it_with_serialized() {
        let result = detect(
            "Stock.xlsx",
            &[
                &["Category", "Item Name", "Quantity"],
                &["Mouse", "Wireless Mouse", "10"],
            ],
        );

        assert_eq!(result.kind, ImportDetectionType::Asset);
        assert_eq!(result.subtype, Some(ImportDetectionSubtype::Quantity));
    }

    #[test]
    fn employee_and_asset_evidence_are_not_cross_routed() {
        let employee = detect(
            "employees.xlsx",
            &[&["EE.ID", "Full Name"], &["ASWVN001", "A"]],
        );
        assert_ne!(employee.kind, ImportDetectionType::Asset);

        let asset = detect(
            "assets.xlsx",
            &[
                &["Asset Tag", "Asset Name", "Category"],
                &["VNLAP001", "A", "Laptop"],
            ],
        );
        assert_ne!(asset.kind, ImportDetectionType::Employee);
    }

    #[test]
    fn ambiguous_workbook_does_not_auto_route() {
        let result = detect(
            "mixed.xlsx",
            &[
                &["EE.ID", "Full Name", "Asset Tag", "Asset Name", "Category"],
                &["ASWVN001", "A", "VNLAP001", "Laptop A", "Laptop"],
            ],
        );

        assert_eq!(result.kind, ImportDetectionType::Ambiguous);
        assert!(result.candidate_types.len() >= 2);
    }

    #[test]
    fn unknown_workbook_has_actionable_reason() {
        let result = detect("notes.xlsx", &[&["Date", "Comment"], &["2026", "Nothing"]]);

        assert_eq!(result.kind, ImportDetectionType::Unknown);
        assert!(result.reason.to_lowercase().contains("employee"));
        assert!(!result.reason.to_lowercase().contains("database"));
    }

    #[test]
    fn missing_asset_tag_is_not_high_confidence_serialized_detection() {
        let result = detect(
            "legacy.xlsx",
            &[
                &["Asset Name", "Category", "Computer Name"],
                &["Laptop A", "Laptop", "ASWLAP001"],
            ],
        );

        assert!(
            result.kind != ImportDetectionType::Asset
                || result.subtype != Some(ImportDetectionSubtype::Serialized)
                || result.confidence < 0.9
        );
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.to_lowercase().contains("manual")));
    }

    #[test]
    fn computer_name_cannot_substitute_canonical_asset_name() {
        let result = detect(
            "legacy.xlsx",
            &[
                &["Asset Tag", "Category", "Computer Name"],
                &["VNLAP001", "Laptop", "ASWVNLAP001"],
            ],
        );

        assert!(
            result.kind != ImportDetectionType::Asset
                || result.subtype != Some(ImportDetectionSubtype::Serialized)
                || result.confidence < 0.9
        );
    }

    #[test]
    fn legacy_asset_id_is_manual_confirmation_candidate() {
        let result = detect(
            "legacy.xlsx",
            &[
                &["Asset ID", "Asset Name", "Category", "Computer Name"],
                &["504", "VNLAP504", "Laptop", "ASWVNLAP504"],
            ],
        );

        assert_eq!(result.kind, ImportDetectionType::Ambiguous);
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.to_lowercase().contains("manual")));
    }

    #[test]
    fn blank_serial_number_does_not_change_serialized_detection() {
        let result = detect(
            "assets.xlsx",
            &[
                &["Asset Tag", "Asset Name", "Category", "Serial Number"],
                &["VNLAP001", "Laptop A", "Laptop", ""],
            ],
        );

        assert_eq!(result.kind, ImportDetectionType::Asset);
        assert_eq!(result.subtype, Some(ImportDetectionSubtype::Serialized));
    }

    #[test]
    fn real_source_workbooks_are_checked_when_available_without_writing_them() {
        let sources = [
            ("EE List.xlsx", Some(ImportDetectionSubtype::EmployeeList)),
            ("Onboarding.xlsx", Some(ImportDetectionSubtype::Onboarding)),
            ("AssetList.xlsx", Some(ImportDetectionSubtype::Serialized)),
            ("NewAssetList.xlsx", None),
        ];

        for (file_name, expected_subtype) in sources {
            let path = Path::new(r"E:\Staff_Kit\00_ExSource").join(file_name);
            if !path.exists() {
                continue;
            }

            let result = detect_import_file(&path).expect("detect real workbook");
            assert_eq!(result.subtype, expected_subtype, "{file_name}");
            if file_name == "NewAssetList.xlsx" {
                assert_eq!(result.kind, ImportDetectionType::Ambiguous);
                assert!(result
                    .warnings
                    .iter()
                    .any(|warning| warning.to_lowercase().contains("manual")));
            }
        }
    }
}
