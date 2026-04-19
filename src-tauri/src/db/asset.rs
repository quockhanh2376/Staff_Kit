use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::{
    humanize_sqlite_error, normalize_optional_text, open_runtime_connection, require_text,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetUpsertInput {
    pub asset_code: String,
    pub category_id: Option<i64>,
    pub asset_type: String,
    pub display_name: String,
    pub display_name_short: Option<String>,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub usage_location: Option<String>,
    pub adapter_number: Option<String>,
    pub warehouse: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetRecord {
    pub id: i64,
    pub asset_code: String,
    pub asset_type: String,
    pub display_name: String,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub notes: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetCategoryRecord {
    pub id: i64,
    pub category_code: String,
    pub category_name: String,
    pub tracking_mode: String,
    pub prefix_code: Option<String>,
    pub qr_required: bool,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetCategoryPrefixRecord {
    pub id: i64,
    pub prefix_value: String,
    pub is_primary: bool,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetCategoryDetailRecord {
    pub id: i64,
    pub category_code: String,
    pub category_name: String,
    pub tracking_mode: String,
    pub prefix_code: Option<String>,
    pub qr_required: bool,
    pub is_active: bool,
    pub asset_count: i64,
    pub stock_item_count: i64,
    pub prefixes: Vec<AssetCategoryPrefixRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetCategoryPrefixInput {
    pub prefix_value: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetCategoryUpsertInput {
    pub id: Option<i64>,
    pub category_code: String,
    pub category_name: String,
    pub tracking_mode: String,
    pub qr_required: bool,
    pub prefixes: Vec<AssetCategoryPrefixInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDashboardSummary {
    pub total_serialized_assets: i64,
    pub serialized_in_stock: i64,
    pub serialized_assigned: i64,
    pub total_quantity_on_hand: i64,
    pub total_quantity_assigned: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDashboardSerializedRecord {
    pub asset_id: i64,
    pub asset_code: String,
    pub category_code: Option<String>,
    pub category_name: Option<String>,
    pub computer_name: Option<String>,
    pub display_name: String,
    pub display_name_short: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub adapter_number: Option<String>,
    pub usage_location: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub holder_employee_id: Option<String>,
    pub holder_full_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDashboardQuantityRecord {
    pub stock_item_id: i64,
    pub category_code: String,
    pub category_name: String,
    pub item_name: String,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub warehouse: Option<String>,
    pub quantity_on_hand: i64,
    pub assigned_quantity: i64,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockItemQuantityUpdateInput {
    pub stock_item_id: i64,
    pub quantity_on_hand: i64,
    pub assigned_quantity: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct AssetCategoryLookupRecord {
    pub id: i64,
    pub tracking_mode: String,
}

#[derive(Debug, Clone)]
pub(crate) struct StockItemCreateInput {
    pub category_id: i64,
    pub item_name: String,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub warehouse: Option<String>,
    pub quantity_on_hand: i64,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AssetLookupRecord {
    pub id: i64,
    pub asset_code: String,
    pub status: String,
}

pub(crate) fn load_asset_by_code_tx(
    tx: &Transaction<'_>,
    asset_code: &str,
) -> Result<Option<AssetLookupRecord>, String> {
    tx.query_row(
        "SELECT id, asset_code, status FROM assets WHERE asset_code = ? COLLATE NOCASE",
        params![asset_code],
        |row| {
            Ok(AssetLookupRecord {
                id: row.get(0)?,
                asset_code: row.get(1)?,
                status: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to load asset by code: {err}"))
}

pub(crate) fn load_asset_by_id_tx(
    tx: &Transaction<'_>,
    asset_id: i64,
) -> Result<Option<AssetLookupRecord>, String> {
    tx.query_row(
        "SELECT id, asset_code, status FROM assets WHERE id = ?",
        params![asset_id],
        |row| {
            Ok(AssetLookupRecord {
                id: row.get(0)?,
                asset_code: row.get(1)?,
                status: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to load asset by id: {err}"))
}

pub(crate) fn set_asset_status_tx(
    tx: &Transaction<'_>,
    asset_id: i64,
    status: &str,
) -> Result<(), String> {
    let changed = tx
        .execute(
            "UPDATE assets SET status = ?, updated_at = datetime('now') WHERE id = ?",
            params![status, asset_id],
        )
        .map_err(humanize_sqlite_error)?;

    if changed == 0 {
        return Err(format!("asset with id {asset_id} was not found"));
    }

    Ok(())
}

fn insert_asset_stmt(
    executor: &Connection,
    asset_code: &str,
    category_id: Option<i64>,
    asset_type: &str,
    display_name: &str,
    display_name_short: Option<&str>,
    brand: Option<&str>,
    model: Option<&str>,
    serial_number: Option<&str>,
    usage_location: Option<&str>,
    adapter_number: Option<&str>,
    warehouse: Option<&str>,
    notes: Option<&str>,
) -> Result<i64, String> {
    executor
        .execute(
            r#"
            INSERT INTO assets(
              asset_code,
              category_id,
              asset_type,
              display_name,
              display_name_short,
              brand,
              model,
              serial_number,
              usage_location,
              adapter_number,
              warehouse,
              notes,
              status,
              created_at,
              updated_at
            )
            VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'in_stock', datetime('now'), datetime('now'))
            "#,
            params![
                asset_code,
                category_id,
                asset_type,
                display_name,
                display_name_short,
                brand,
                model,
                serial_number,
                usage_location,
                adapter_number,
                warehouse,
                notes
            ],
        )
        .map_err(humanize_sqlite_error)?;

    Ok(executor.last_insert_rowid())
}

fn load_asset_record_by_id_conn(
    executor: &Connection,
    asset_id: i64,
) -> Result<AssetRecord, String> {
    executor
        .query_row(
            r#"
            SELECT
              id,
              asset_code,
              asset_type,
              display_name,
              model,
              serial_number,
              notes,
              status
            FROM assets
            WHERE id = ?
            "#,
            params![asset_id],
            |row| {
                Ok(AssetRecord {
                    id: row.get(0)?,
                    asset_code: row.get(1)?,
                    asset_type: row.get(2)?,
                    display_name: row.get(3)?,
                    model: row.get(4)?,
                    serial_number: row.get(5)?,
                    notes: row.get(6)?,
                    status: row.get(7)?,
                })
            },
        )
        .map_err(|err| format!("failed to load asset with id {asset_id}: {err}"))
}

pub(crate) fn load_asset_category_by_code_or_name_tx(
    tx: &Transaction<'_>,
    value: &str,
) -> Result<Option<AssetCategoryLookupRecord>, String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Ok(None);
    }

    tx.query_row(
        r#"
        SELECT id, tracking_mode
        FROM asset_categories
        WHERE is_active = 1
          AND (
            category_code = ? COLLATE NOCASE
            OR category_name = ? COLLATE NOCASE
          )
        LIMIT 1
        "#,
        params![normalized, normalized],
        |row| {
            Ok(AssetCategoryLookupRecord {
                id: row.get(0)?,
                tracking_mode: row.get(1)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to load asset category '{normalized}': {err}"))
}

fn normalize_asset_category_prefix(value: &str) -> Option<String> {
    let normalized = value.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

#[allow(dead_code)]
pub(crate) fn load_asset_category_by_prefix_conn(
    conn: &Connection,
    asset_code: &str,
) -> Result<Option<AssetCategoryLookupRecord>, String> {
    let normalized_code = asset_code.trim().to_ascii_uppercase();
    if normalized_code.is_empty() {
        return Ok(None);
    }

    conn.query_row(
        r#"
        SELECT c.id, c.tracking_mode
        FROM asset_category_prefixes p
        INNER JOIN asset_categories c ON c.id = p.category_id
        WHERE p.is_active = 1
          AND c.is_active = 1
          AND substr(?, 1, length(p.prefix_value)) = p.prefix_value
        ORDER BY length(p.prefix_value) DESC, p.is_primary DESC, p.id ASC
        LIMIT 1
        "#,
        params![normalized_code],
        |row| {
            Ok(AssetCategoryLookupRecord {
                id: row.get(0)?,
                tracking_mode: row.get(1)?,
            })
        },
    )
    .optional()
    .map_err(|err| format!("failed to load asset category by prefix '{normalized_code}': {err}"))
}

pub(crate) fn upsert_asset_category_prefix_conn(
    conn: &Connection,
    category_id: i64,
    prefix_value: &str,
    is_primary: bool,
    is_active: bool,
) -> Result<(), String> {
    let Some(normalized_prefix) = normalize_asset_category_prefix(prefix_value) else {
        return Err("asset category prefix cannot be blank".to_string());
    };
    if normalized_prefix.contains('%') || normalized_prefix.contains('_') {
        return Err("asset category prefix cannot contain SQL wildcard characters (% or _)".to_string());
    }

    let existing_id = conn
        .query_row(
            r#"
            SELECT id
            FROM asset_category_prefixes
            WHERE category_id = ?
              AND prefix_value = ? COLLATE NOCASE
            LIMIT 1
            "#,
            params![category_id, normalized_prefix.as_str()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|err| format!("failed to inspect asset category prefix '{normalized_prefix}': {err}"))?;

    if let Some(prefix_id) = existing_id {
        conn.execute(
            r#"
            UPDATE asset_category_prefixes
            SET is_primary = ?,
                is_active = ?,
                updated_at = datetime('now')
            WHERE id = ?
            "#,
            params![if is_primary { 1_i64 } else { 0_i64 }, if is_active { 1_i64 } else { 0_i64 }, prefix_id],
        )
        .map_err(humanize_sqlite_error)?;
    } else {
        conn.execute(
            r#"
            INSERT INTO asset_category_prefixes(
              category_id,
              prefix_value,
              is_primary,
              is_active,
              created_at,
              updated_at
            )
            VALUES(?, ?, ?, ?, datetime('now'), datetime('now'))
            "#,
            params![
                category_id,
                normalized_prefix.as_str(),
                if is_primary { 1_i64 } else { 0_i64 },
                if is_active { 1_i64 } else { 0_i64 },
            ],
        )
        .map_err(humanize_sqlite_error)?;
    }

    Ok(())
}

pub(crate) fn create_asset_tx(
    tx: &Transaction<'_>,
    input: &AssetUpsertInput,
) -> Result<AssetRecord, String> {
    let asset_code = require_text(input.asset_code.clone(), "assetCode")?.to_uppercase();
    let category_id = input.category_id;
    let asset_type = require_text(input.asset_type.clone(), "assetType")?;
    let display_name = require_text(input.display_name.clone(), "displayName")?;
    let display_name_short = normalize_optional_text(input.display_name_short.clone());
    let brand = normalize_optional_text(input.brand.clone());
    let model = normalize_optional_text(input.model.clone());
    let serial_number = normalize_optional_text(input.serial_number.clone());
    let usage_location = normalize_optional_text(input.usage_location.clone());
    let adapter_number = normalize_optional_text(input.adapter_number.clone());
    let warehouse = normalize_optional_text(input.warehouse.clone());
    let notes = normalize_optional_text(input.notes.clone());

    let asset_id = insert_asset_stmt(
        tx,
        asset_code.as_str(),
        category_id,
        asset_type.as_str(),
        display_name.as_str(),
        display_name_short.as_deref(),
        brand.as_deref(),
        model.as_deref(),
        serial_number.as_deref(),
        usage_location.as_deref(),
        adapter_number.as_deref(),
        warehouse.as_deref(),
        notes.as_deref(),
    )?;

    load_asset_record_by_id_conn(tx, asset_id)
}

pub(crate) fn create_stock_item_tx(
    tx: &Transaction<'_>,
    input: &StockItemCreateInput,
) -> Result<i64, String> {
    let item_name = require_text(input.item_name.clone(), "itemName")?;
    if input.quantity_on_hand <= 0 {
        return Err("quantity must be a positive integer".to_string());
    }

    let brand = normalize_optional_text(input.brand.clone());
    let model = normalize_optional_text(input.model.clone());
    let warehouse = normalize_optional_text(input.warehouse.clone());
    let note = normalize_optional_text(input.note.clone());

    tx.execute(
        r#"
        INSERT INTO stock_items(
          category_id,
          item_name,
          brand,
          model,
          warehouse,
          quantity_on_hand,
          assigned_quantity,
          note,
          created_at,
          updated_at
        )
        VALUES(?, ?, ?, ?, ?, ?, 0, ?, datetime('now'), datetime('now'))
        "#,
        params![
            input.category_id,
            item_name,
            brand.as_deref(),
            model.as_deref(),
            warehouse.as_deref(),
            input.quantity_on_hand,
            note.as_deref(),
        ],
    )
    .map_err(humanize_sqlite_error)?;

    Ok(tx.last_insert_rowid())
}

pub(crate) fn create_asset_conn(
    conn: &mut Connection,
    input: AssetUpsertInput,
) -> Result<AssetRecord, String> {
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start asset create transaction: {err}"))?;

    let record = create_asset_tx(&tx, &input)?;

    tx.commit()
        .map_err(|err| format!("failed to commit asset create transaction: {err}"))?;

    Ok(record)
}

pub(crate) fn upsert_assets_conn(
    conn: &mut Connection,
    assets: Vec<AssetUpsertInput>,
) -> Result<Vec<AssetRecord>, String> {
    if assets.is_empty() {
        return Err("at least one asset is required".to_string());
    }

    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start asset upsert transaction: {err}"))?;

    let mut normalized_codes = Vec::new();

    for input in assets {
        let asset_code = require_text(input.asset_code, "assetCode")?.to_uppercase();
        let category_id = input.category_id;
        let asset_type = require_text(input.asset_type, "assetType")?;
        let display_name = require_text(input.display_name, "displayName")?;
        let display_name_short = normalize_optional_text(input.display_name_short);
        let brand = normalize_optional_text(input.brand);
        let model = normalize_optional_text(input.model);
        let serial_number = normalize_optional_text(input.serial_number);
        let usage_location = normalize_optional_text(input.usage_location);
        let adapter_number = normalize_optional_text(input.adapter_number);
        let warehouse = normalize_optional_text(input.warehouse);
        let notes = normalize_optional_text(input.notes);

        tx.execute(
            r#"
            INSERT INTO assets(
              asset_code,
              category_id,
              asset_type,
              display_name,
              display_name_short,
              brand,
              model,
              serial_number,
              usage_location,
              adapter_number,
              warehouse,
              notes,
              status,
              created_at,
              updated_at
            )
                        VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'in_stock', datetime('now'), datetime('now'))
            ON CONFLICT(asset_code) DO UPDATE SET
              category_id = excluded.category_id,
              asset_type = excluded.asset_type,
              display_name = excluded.display_name,
              display_name_short = excluded.display_name_short,
              brand = excluded.brand,
              model = excluded.model,
              serial_number = excluded.serial_number,
              usage_location = excluded.usage_location,
              adapter_number = excluded.adapter_number,
              warehouse = excluded.warehouse,
              notes = excluded.notes,
              updated_at = datetime('now')
            "#,
            params![
                asset_code.as_str(),
                category_id,
                asset_type.as_str(),
                display_name.as_str(),
                display_name_short.as_deref(),
                brand.as_deref(),
                model.as_deref(),
                serial_number.as_deref(),
                usage_location.as_deref(),
                adapter_number.as_deref(),
                warehouse.as_deref(),
                notes.as_deref(),
            ],
        )
        .map_err(humanize_sqlite_error)?;

        normalized_codes.push(asset_code);
    }

    let mut records = Vec::new();
    for asset_code in &normalized_codes {
        let record = tx
            .query_row(
                r#"
                SELECT
                  id,
                  asset_code,
                  asset_type,
                  display_name,
                  model,
                  serial_number,
                  notes,
                  status
                FROM assets
                WHERE asset_code = ? COLLATE NOCASE
                "#,
                params![asset_code],
                |row| {
                    Ok(AssetRecord {
                        id: row.get(0)?,
                        asset_code: row.get(1)?,
                        asset_type: row.get(2)?,
                        display_name: row.get(3)?,
                        model: row.get(4)?,
                        serial_number: row.get(5)?,
                        notes: row.get(6)?,
                        status: row.get(7)?,
                    })
                },
            )
            .map_err(|err| format!("failed to load upserted asset '{asset_code}': {err}"))?;
        records.push(record);
    }

    tx.commit()
        .map_err(|err| format!("failed to commit asset upsert transaction: {err}"))?;

    Ok(records)
}

pub(crate) fn search_in_stock_assets_conn(
    conn: &Connection,
    query: Option<&str>,
    limit: usize,
) -> Result<Vec<AssetRecord>, String> {
    let normalized_limit = limit.clamp(1, 50) as i64;
    let like_query = query
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{}%", value.to_uppercase()));

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              id,
              asset_code,
              asset_type,
              display_name,
              model,
              serial_number,
              notes,
              status
            FROM assets
            WHERE status = 'in_stock'
              AND (
                ? IS NULL
                OR UPPER(asset_code) LIKE ?
                OR UPPER(asset_type) LIKE ?
                OR UPPER(display_name) LIKE ?
                OR UPPER(COALESCE(model, '')) LIKE ?
                OR UPPER(COALESCE(serial_number, '')) LIKE ?
              )
            ORDER BY asset_code ASC, id ASC
            LIMIT ?
            "#,
        )
        .map_err(|err| format!("failed to prepare asset search query: {err}"))?;

    let rows = stmt
        .query_map(
            params![
                like_query.as_deref(),
                like_query.as_deref(),
                like_query.as_deref(),
                like_query.as_deref(),
                like_query.as_deref(),
                like_query.as_deref(),
                normalized_limit
            ],
            |row| {
                Ok(AssetRecord {
                    id: row.get(0)?,
                    asset_code: row.get(1)?,
                    asset_type: row.get(2)?,
                    display_name: row.get(3)?,
                    model: row.get(4)?,
                    serial_number: row.get(5)?,
                    notes: row.get(6)?,
                    status: row.get(7)?,
                })
            },
        )
        .map_err(|err| format!("failed to query assets for search: {err}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|err| format!("failed to read asset search row: {err}"))?);
    }

    Ok(items)
}

pub(crate) fn search_assigned_assets_conn(
    conn: &Connection,
    query: Option<&str>,
    limit: usize,
) -> Result<Vec<AssetRecord>, String> {
    let normalized_limit = limit.clamp(1, 50) as i64;
    let like_query = query
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{}%", value.to_uppercase()));

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              id,
              asset_code,
              asset_type,
              display_name,
              model,
              serial_number,
              notes,
              status
            FROM assets
            WHERE status = 'assigned'
              AND (
                ? IS NULL
                OR UPPER(asset_code) LIKE ?
                OR UPPER(asset_type) LIKE ?
                OR UPPER(display_name) LIKE ?
                OR UPPER(COALESCE(model, '')) LIKE ?
                OR UPPER(COALESCE(serial_number, '')) LIKE ?
              )
            ORDER BY asset_code ASC, id ASC
            LIMIT ?
            "#,
        )
        .map_err(|err| format!("failed to prepare assigned asset search query: {err}"))?;

    let rows = stmt
        .query_map(
            params![
                like_query.as_deref(),
                like_query.as_deref(),
                like_query.as_deref(),
                like_query.as_deref(),
                like_query.as_deref(),
                like_query.as_deref(),
                normalized_limit
            ],
            |row| {
                Ok(AssetRecord {
                    id: row.get(0)?,
                    asset_code: row.get(1)?,
                    asset_type: row.get(2)?,
                    display_name: row.get(3)?,
                    model: row.get(4)?,
                    serial_number: row.get(5)?,
                    notes: row.get(6)?,
                    status: row.get(7)?,
                })
            },
        )
        .map_err(|err| format!("failed to query assigned assets for search: {err}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|err| format!("failed to read assigned asset search row: {err}"))?);
    }

    Ok(items)
}

pub fn upsert_assets(
    app: &AppHandle,
    assets: Vec<AssetUpsertInput>,
) -> Result<Vec<AssetRecord>, String> {
    let mut conn = open_runtime_connection(app)?;
    upsert_assets_conn(&mut conn, assets)
}

pub fn list_asset_categories(app: &AppHandle) -> Result<Vec<AssetCategoryRecord>, String> {
    let conn = open_runtime_connection(app)?;
    list_asset_categories_conn(&conn)
}

pub fn list_asset_category_details(
    app: &AppHandle,
) -> Result<Vec<AssetCategoryDetailRecord>, String> {
    let conn = open_runtime_connection(app)?;
    list_asset_category_details_conn(&conn)
}

pub fn create_asset_category(
    app: &AppHandle,
    payload: AssetCategoryUpsertInput,
) -> Result<AssetCategoryDetailRecord, String> {
    if payload.id.is_some() {
        return Err("createAssetCategory does not accept an id".to_string());
    }
    let conn = open_runtime_connection(app)?;
    upsert_asset_category_conn(&conn, payload)
}

pub fn update_asset_category(
    app: &AppHandle,
    payload: AssetCategoryUpsertInput,
) -> Result<AssetCategoryDetailRecord, String> {
    if payload.id.is_none() {
        return Err("updateAssetCategory requires an id".to_string());
    }
    let conn = open_runtime_connection(app)?;
    upsert_asset_category_conn(&conn, payload)
}

pub fn deactivate_asset_category(
    app: &AppHandle,
    category_id: i64,
) -> Result<AssetCategoryDetailRecord, String> {
    let conn = open_runtime_connection(app)?;
    deactivate_asset_category_conn(&conn, category_id)
}

pub fn get_asset_dashboard_summary(app: &AppHandle) -> Result<AssetDashboardSummary, String> {
    let conn = open_runtime_connection(app)?;
    get_asset_dashboard_summary_conn(&conn)
}

pub fn list_asset_dashboard_serialized(
    app: &AppHandle,
) -> Result<Vec<AssetDashboardSerializedRecord>, String> {
    let conn = open_runtime_connection(app)?;
    list_asset_dashboard_serialized_conn(&conn)
}

pub fn list_asset_dashboard_quantity(
    app: &AppHandle,
) -> Result<Vec<AssetDashboardQuantityRecord>, String> {
    let conn = open_runtime_connection(app)?;
    list_asset_dashboard_quantity_conn(&conn)
}

pub fn update_stock_item_quantity(
    app: &AppHandle,
    payload: StockItemQuantityUpdateInput,
) -> Result<AssetDashboardQuantityRecord, String> {
    let mut conn = open_runtime_connection(app)?;
    update_stock_item_quantity_conn(&mut conn, payload)
}

pub(crate) fn list_asset_categories_conn(
    conn: &Connection,
) -> Result<Vec<AssetCategoryRecord>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              id,
              category_code,
              category_name,
              tracking_mode,
              prefix_code,
              qr_required,
              is_active
            FROM asset_categories
            ORDER BY
              CASE tracking_mode WHEN 'serialized' THEN 0 ELSE 1 END,
              category_name COLLATE NOCASE,
              id
            "#,
        )
        .map_err(|err| format!("failed to prepare asset category query: {err}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(AssetCategoryRecord {
                id: row.get(0)?,
                category_code: row.get(1)?,
                category_name: row.get(2)?,
                tracking_mode: row.get(3)?,
                prefix_code: row.get(4)?,
                qr_required: row.get::<_, i64>(5)? > 0,
                is_active: row.get::<_, i64>(6)? > 0,
            })
        })
        .map_err(|err| format!("failed to query asset categories: {err}"))?;

    let mut categories = Vec::new();
    for row in rows {
        categories.push(row.map_err(|err| format!("failed to read asset category row: {err}"))?);
    }

    Ok(categories)
}

fn load_asset_category_prefix_records_conn(
    conn: &Connection,
    category_id: i64,
) -> Result<Vec<AssetCategoryPrefixRecord>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              id,
              prefix_value,
              is_primary,
              is_active
            FROM asset_category_prefixes
            WHERE category_id = ?
              AND is_active = 1
            ORDER BY is_primary DESC, prefix_value COLLATE NOCASE ASC, id ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare asset category prefix detail query: {err}"))?;

    let rows = stmt
        .query_map(params![category_id], |row| {
            Ok(AssetCategoryPrefixRecord {
                id: row.get(0)?,
                prefix_value: row.get(1)?,
                is_primary: row.get::<_, i64>(2)? > 0,
                is_active: row.get::<_, i64>(3)? > 0,
            })
        })
        .map_err(|err| format!("failed to query asset category prefixes: {err}"))?;

    let mut prefixes = Vec::new();
    for row in rows {
        prefixes.push(row.map_err(|err| format!("failed to read asset category prefix row: {err}"))?);
    }

    Ok(prefixes)
}

fn load_asset_category_prefix_records_by_category_ids_conn(
    conn: &Connection,
    category_ids: &[i64],
) -> Result<HashMap<i64, Vec<AssetCategoryPrefixRecord>>, String> {
    if category_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = vec!["?"; category_ids.len()].join(", ");
    let sql = format!(
        r#"
        SELECT
          category_id,
          id,
          prefix_value,
          is_primary,
          is_active
        FROM asset_category_prefixes
        WHERE is_active = 1
          AND category_id IN ({placeholders})
        ORDER BY
          category_id ASC,
          is_primary DESC,
          prefix_value COLLATE NOCASE ASC,
          id ASC
        "#
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("failed to prepare batched asset category prefix detail query: {err}"))?;

    let rows = stmt
        .query_map(rusqlite::params_from_iter(category_ids.iter()), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                AssetCategoryPrefixRecord {
                    id: row.get(1)?,
                    prefix_value: row.get(2)?,
                    is_primary: row.get::<_, i64>(3)? > 0,
                    is_active: row.get::<_, i64>(4)? > 0,
                },
            ))
        })
        .map_err(|err| format!("failed to query batched asset category prefixes: {err}"))?;

    let mut prefixes_by_category_id: HashMap<i64, Vec<AssetCategoryPrefixRecord>> = HashMap::new();
    for row in rows {
        let (category_id, prefix) =
            row.map_err(|err| format!("failed to read batched asset category prefix row: {err}"))?;
        prefixes_by_category_id
            .entry(category_id)
            .or_default()
            .push(prefix);
    }

    Ok(prefixes_by_category_id)
}

fn load_asset_category_detail_by_id_conn(
    conn: &Connection,
    category_id: i64,
) -> Result<AssetCategoryDetailRecord, String> {
    let mut detail = conn
        .query_row(
            r#"
            SELECT
              c.id,
              c.category_code,
              c.category_name,
              c.tracking_mode,
              c.prefix_code,
              c.qr_required,
              c.is_active,
              COALESCE((SELECT COUNT(*) FROM assets a WHERE a.category_id = c.id), 0),
              COALESCE((SELECT COUNT(*) FROM stock_items si WHERE si.category_id = c.id), 0)
            FROM asset_categories c
            WHERE c.id = ?
            "#,
            params![category_id],
            |row| {
                Ok(AssetCategoryDetailRecord {
                    id: row.get(0)?,
                    category_code: row.get(1)?,
                    category_name: row.get(2)?,
                    tracking_mode: row.get(3)?,
                    prefix_code: row.get(4)?,
                    qr_required: row.get::<_, i64>(5)? > 0,
                    is_active: row.get::<_, i64>(6)? > 0,
                    asset_count: row.get(7)?,
                    stock_item_count: row.get(8)?,
                    prefixes: Vec::new(),
                })
            },
        )
        .optional()
        .map_err(|err| format!("failed to load asset category detail: {err}"))?
        .ok_or_else(|| format!("asset category with id {category_id} was not found"))?;

    detail.prefixes = load_asset_category_prefix_records_conn(conn, category_id)?;
    Ok(detail)
}

pub(crate) fn list_asset_category_details_conn(
    conn: &Connection,
) -> Result<Vec<AssetCategoryDetailRecord>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              c.id,
              c.category_code,
              c.category_name,
              c.tracking_mode,
              c.prefix_code,
              c.qr_required,
              c.is_active,
              COALESCE((SELECT COUNT(*) FROM assets a WHERE a.category_id = c.id), 0),
              COALESCE((SELECT COUNT(*) FROM stock_items si WHERE si.category_id = c.id), 0)
            FROM asset_categories
            c
            ORDER BY
              is_active DESC,
              CASE tracking_mode WHEN 'serialized' THEN 0 ELSE 1 END,
              category_name COLLATE NOCASE ASC,
              id ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare asset category detail list query: {err}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(AssetCategoryDetailRecord {
                id: row.get(0)?,
                category_code: row.get(1)?,
                category_name: row.get(2)?,
                tracking_mode: row.get(3)?,
                prefix_code: row.get(4)?,
                qr_required: row.get::<_, i64>(5)? > 0,
                is_active: row.get::<_, i64>(6)? > 0,
                asset_count: row.get(7)?,
                stock_item_count: row.get(8)?,
                prefixes: Vec::new(),
            })
        })
        .map_err(|err| format!("failed to query asset category details: {err}"))?;

    let mut details = Vec::new();
    let mut category_ids = Vec::new();
    for row in rows {
        let detail =
            row.map_err(|err| format!("failed to read asset category detail row: {err}"))?;
        category_ids.push(detail.id);
        details.push(detail);
    }

    let mut prefixes_by_category_id =
        load_asset_category_prefix_records_by_category_ids_conn(conn, &category_ids)?;
    for detail in &mut details {
        detail.prefixes = prefixes_by_category_id.remove(&detail.id).unwrap_or_default();
    }

    Ok(details)
}

pub(crate) fn upsert_asset_category_conn(
    conn: &Connection,
    payload: AssetCategoryUpsertInput,
) -> Result<AssetCategoryDetailRecord, String> {
    let normalized_code = normalize_asset_category_code(payload.category_code)?;
    let normalized_name = require_text(payload.category_name, "categoryName")?;
    let normalized_tracking_mode = normalize_asset_tracking_mode(payload.tracking_mode)?;
    let normalized_prefixes = normalize_asset_category_prefix_inputs(payload.prefixes)?;

    with_asset_category_savepoint(conn, |conn| {
        let category_id = if let Some(existing_id) = payload.id {
            let existing_tracking_mode: String = conn
                .query_row(
                    "SELECT tracking_mode FROM asset_categories WHERE id = ?",
                    params![existing_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|err| format!("failed to load asset category before update: {err}"))?
                .ok_or_else(|| format!("asset category with id {existing_id} was not found"))?;

            if existing_tracking_mode != normalized_tracking_mode {
                let asset_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM assets WHERE category_id = ?",
                        params![existing_id],
                        |row| row.get(0),
                    )
                    .map_err(|err| format!("failed to inspect asset category asset references: {err}"))?;
                let stock_item_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM stock_items WHERE category_id = ?",
                        params![existing_id],
                        |row| row.get(0),
                    )
                    .map_err(|err| format!("failed to inspect asset category stock references: {err}"))?;

                if asset_count > 0 || stock_item_count > 0 {
                    return Err("cannot change tracking mode for a category already referenced by assets or stock items".to_string());
                }
            }

            conn.execute(
                r#"
                UPDATE asset_categories
                SET
                  category_code = ?,
                  category_name = ?,
                  tracking_mode = ?,
                  qr_required = ?,
                  updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![
                    normalized_code.as_str(),
                    normalized_name.as_str(),
                    normalized_tracking_mode.as_str(),
                    if payload.qr_required { 1_i64 } else { 0_i64 },
                    existing_id
                ],
            )
            .map_err(humanize_sqlite_error)?;

            existing_id
        } else {
            conn.execute(
                r#"
                INSERT INTO asset_categories(
                  category_code,
                  category_name,
                  tracking_mode,
                  qr_required,
                  is_active,
                  created_at,
                  updated_at
                )
                VALUES(?, ?, ?, ?, 1, datetime('now'), datetime('now'))
                "#,
                params![
                    normalized_code.as_str(),
                    normalized_name.as_str(),
                    normalized_tracking_mode.as_str(),
                    if payload.qr_required { 1_i64 } else { 0_i64 }
                ],
            )
            .map_err(humanize_sqlite_error)?;

            conn.last_insert_rowid()
        };

        conn.execute(
            r#"
            UPDATE asset_category_prefixes
            SET
              is_primary = 0,
              is_active = 0,
              updated_at = datetime('now')
            WHERE category_id = ?
            "#,
            params![category_id],
        )
        .map_err(humanize_sqlite_error)?;

        for prefix in &normalized_prefixes {
            upsert_asset_category_prefix_conn(
                conn,
                category_id,
                prefix.prefix_value.as_str(),
                prefix.is_primary,
                true,
            )?;
        }

        let primary_prefix = normalized_prefixes
            .iter()
            .find(|prefix| prefix.is_primary)
            .map(|prefix| prefix.prefix_value.as_str());

        conn.execute(
            "UPDATE asset_categories SET prefix_code = ?, is_active = 1, updated_at = datetime('now') WHERE id = ?",
            params![primary_prefix, category_id],
        )
        .map_err(humanize_sqlite_error)?;

        load_asset_category_detail_by_id_conn(conn, category_id)
    })
}

pub(crate) fn deactivate_asset_category_conn(
    conn: &Connection,
    category_id: i64,
) -> Result<AssetCategoryDetailRecord, String> {
    with_asset_category_savepoint(conn, |conn| {
        let changed = conn
            .execute(
                r#"
                UPDATE asset_categories
                SET
                  is_active = 0,
                  prefix_code = NULL,
                  updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![category_id],
            )
            .map_err(humanize_sqlite_error)?;

        if changed == 0 {
            return Err(format!("asset category with id {category_id} was not found"));
        }

        conn.execute(
            r#"
            UPDATE asset_category_prefixes
            SET
              is_primary = 0,
              is_active = 0,
              updated_at = datetime('now')
            WHERE category_id = ?
            "#,
            params![category_id],
        )
        .map_err(humanize_sqlite_error)?;

        load_asset_category_detail_by_id_conn(conn, category_id)
    })
}

pub(crate) fn get_asset_dashboard_summary_conn(
    conn: &Connection,
) -> Result<AssetDashboardSummary, String> {
    conn.query_row(
        r#"
        SELECT
          COALESCE((SELECT COUNT(*) FROM assets), 0),
          COALESCE((SELECT COUNT(*) FROM assets WHERE status = 'in_stock'), 0),
          COALESCE((SELECT COUNT(*) FROM assets WHERE status = 'assigned'), 0),
          COALESCE((SELECT SUM(quantity_on_hand) FROM stock_items), 0),
          COALESCE((SELECT SUM(assigned_quantity) FROM stock_items), 0)
        "#,
        [],
        |row| {
            Ok(AssetDashboardSummary {
                total_serialized_assets: row.get(0)?,
                serialized_in_stock: row.get(1)?,
                serialized_assigned: row.get(2)?,
                total_quantity_on_hand: row.get(3)?,
                total_quantity_assigned: row.get(4)?,
            })
        },
    )
    .map_err(|err| format!("failed to load asset dashboard summary: {err}"))
}

pub(crate) fn list_asset_dashboard_serialized_conn(
    conn: &Connection,
) -> Result<Vec<AssetDashboardSerializedRecord>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              a.id,
              a.asset_code,
              c.category_code,
              c.category_name,
                            CASE
                                WHEN COALESCE(c.has_computer_name, 0) = 1 THEN 'ASW' || UPPER(a.asset_code)
                                ELSE NULL
                            END AS computer_name,
              a.display_name,
              a.display_name_short,
              a.model,
              a.serial_number,
              a.adapter_number,
              a.usage_location,
              a.notes,
              a.status,
              e.employee_id,
              e.full_name
            FROM assets a
            LEFT JOIN asset_categories c ON c.id = a.category_id
            LEFT JOIN asset_loans al
              ON al.asset_id = a.id
             AND al.returned_at IS NULL
            LEFT JOIN employees e ON e.id = al.employee_id_fk
            ORDER BY
              COALESCE(c.category_name, a.asset_type) COLLATE NOCASE ASC,
              a.asset_code COLLATE NOCASE ASC,
              a.id ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare serialized dashboard query: {err}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(AssetDashboardSerializedRecord {
                asset_id: row.get(0)?,
                asset_code: row.get(1)?,
                category_code: row.get(2)?,
                category_name: row.get(3)?,
                computer_name: row.get(4)?,
                display_name: row.get(5)?,
                display_name_short: row.get(6)?,
                model: row.get(7)?,
                serial_number: row.get(8)?,
                adapter_number: row.get(9)?,
                usage_location: row.get(10)?,
                notes: row.get(11)?,
                status: row.get(12)?,
                holder_employee_id: row.get(13)?,
                holder_full_name: row.get(14)?,
            })
        })
        .map_err(|err| format!("failed to query serialized dashboard rows: {err}"))?;

    let mut records = Vec::new();
    for row in rows {
        records.push(row.map_err(|err| format!("failed to read serialized dashboard row: {err}"))?);
    }

    Ok(records)
}

pub(crate) fn list_asset_dashboard_quantity_conn(
    conn: &Connection,
) -> Result<Vec<AssetDashboardQuantityRecord>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
              si.id,
              c.category_code,
              c.category_name,
              si.item_name,
              si.brand,
              si.model,
              si.warehouse,
              si.quantity_on_hand,
              si.assigned_quantity,
              si.note
            FROM stock_items si
            INNER JOIN asset_categories c ON c.id = si.category_id
            ORDER BY
              c.category_name COLLATE NOCASE ASC,
              si.item_name COLLATE NOCASE ASC,
              si.id ASC
            "#,
        )
        .map_err(|err| format!("failed to prepare quantity dashboard query: {err}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(AssetDashboardQuantityRecord {
                stock_item_id: row.get(0)?,
                category_code: row.get(1)?,
                category_name: row.get(2)?,
                item_name: row.get(3)?,
                brand: row.get(4)?,
                model: row.get(5)?,
                warehouse: row.get(6)?,
                quantity_on_hand: row.get(7)?,
                assigned_quantity: row.get(8)?,
                note: row.get(9)?,
            })
        })
        .map_err(|err| format!("failed to query quantity dashboard rows: {err}"))?;

    let mut records = Vec::new();
    for row in rows {
        records.push(row.map_err(|err| format!("failed to read quantity dashboard row: {err}"))?);
    }

    Ok(records)
}

pub(crate) fn update_stock_item_quantity_conn(
    conn: &mut Connection,
    payload: StockItemQuantityUpdateInput,
) -> Result<AssetDashboardQuantityRecord, String> {
    if payload.quantity_on_hand < 0 {
        return Err("quantityOnHand must be zero or greater".to_string());
    }
    if payload.assigned_quantity < 0 {
        return Err("assignedQuantity must be zero or greater".to_string());
    }

    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start stock item quantity transaction: {err}"))?;

    let changed = tx
        .execute(
            r#"
            UPDATE stock_items
            SET
              quantity_on_hand = ?,
              assigned_quantity = ?,
              updated_at = datetime('now')
            WHERE id = ?
            "#,
            params![
                payload.quantity_on_hand,
                payload.assigned_quantity,
                payload.stock_item_id,
            ],
        )
        .map_err(humanize_sqlite_error)?;

    if changed == 0 {
        return Err(format!(
            "stock item with id {} was not found",
            payload.stock_item_id
        ));
    }

    let record = tx
        .query_row(
            r#"
            SELECT
              si.id,
              c.category_code,
              c.category_name,
              si.item_name,
              si.brand,
              si.model,
              si.warehouse,
              si.quantity_on_hand,
              si.assigned_quantity,
              si.note
            FROM stock_items si
            INNER JOIN asset_categories c ON c.id = si.category_id
            WHERE si.id = ?
            "#,
            params![payload.stock_item_id],
            |row| {
                Ok(AssetDashboardQuantityRecord {
                    stock_item_id: row.get(0)?,
                    category_code: row.get(1)?,
                    category_name: row.get(2)?,
                    item_name: row.get(3)?,
                    brand: row.get(4)?,
                    model: row.get(5)?,
                    warehouse: row.get(6)?,
                    quantity_on_hand: row.get(7)?,
                    assigned_quantity: row.get(8)?,
                    note: row.get(9)?,
                })
            },
        )
        .map_err(|err| format!("failed to load updated stock item {}: {err}", payload.stock_item_id))?;

    tx.commit()
        .map_err(|err| format!("failed to commit stock item quantity update: {err}"))?;

    Ok(record)
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use crate::db::{apply_migrations, configure_connection};

    use super::*;

    fn open_test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        configure_connection(&conn).expect("configure sqlite pragmas");
        apply_migrations(&conn).expect("apply migrations");
        conn
    }

    fn load_category_prefix_values(conn: &Connection, category_code: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT p.prefix_value
                FROM asset_category_prefixes p
                INNER JOIN asset_categories c ON c.id = p.category_id
                WHERE c.category_code = ?
                  AND p.is_active = 1
                ORDER BY p.is_primary DESC, p.prefix_value COLLATE NOCASE ASC, p.id ASC
                "#,
            )
            .expect("prepare prefix lookup");

        stmt.query_map(params![category_code], |row| row.get::<_, String>(0))
            .expect("query category prefixes")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect category prefixes")
    }

    fn category_id(conn: &Connection, category_code: &str) -> i64 {
        conn.query_row(
            "SELECT id FROM asset_categories WHERE category_code = ?",
            params![category_code],
            |row| row.get::<_, i64>(0),
        )
        .expect("load category id")
    }

    fn seed_team(conn: &Connection, team_name: &str) -> i64 {
        conn.execute(
            "INSERT INTO teams(name) VALUES(?)",
            params![team_name],
        )
        .expect("insert team");
        conn.last_insert_rowid()
    }

    fn seed_employee(conn: &Connection, employee_id: &str, full_name: &str, team_id: i64) -> i64 {
        conn.execute(
            r#"
            INSERT INTO employees(employee_id, full_name, team_id, staff_group, updated_at)
            VALUES(?, ?, ?, 'employee_list', datetime('now'))
            "#,
            params![employee_id, full_name, team_id],
        )
        .expect("insert employee");
        conn.last_insert_rowid()
    }

    fn seed_borrow_request(conn: &Connection, employee_row_id: i64, employee_id: &str, full_name: &str) -> i64 {
        conn.execute(
            r#"
            INSERT INTO borrow_requests(
              request_key,
              employee_id_fk,
              submitted_employee_id,
              submitted_full_name,
              status,
              request_type,
              submitted_at
            )
            VALUES(?, ?, ?, ?, 'approved', 'borrow', datetime('now'))
            "#,
            params![format!("REQ-{employee_id}"), employee_row_id, employee_id, full_name],
        )
        .expect("insert borrow request");
        conn.last_insert_rowid()
    }

    #[test]
    fn upsert_assets_conn_inserts_new_assets_as_in_stock() {
        let mut conn = open_test_connection();

        let records = upsert_assets_conn(
            &mut conn,
            vec![AssetUpsertInput {
                asset_code: "asset-001".to_string(),
                category_id: None,
                asset_type: "Laptop".to_string(),
                display_name: "Dell Latitude".to_string(),
                display_name_short: None,
                brand: None,
                model: Some("7440".to_string()),
                serial_number: Some("SN-001".to_string()),
                usage_location: None,
                adapter_number: None,
                warehouse: None,
                notes: Some("Seed import".to_string()),
            }],
        )
        .expect("upsert assets");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].asset_code, "ASSET-001");
        assert_eq!(records[0].status, "in_stock");
    }

    #[test]
    fn upsert_assets_conn_updates_metadata_without_resetting_existing_status() {
        let mut conn = open_test_connection();

        let inserted = upsert_assets_conn(
            &mut conn,
            vec![AssetUpsertInput {
                asset_code: "ASSET-001".to_string(),
                category_id: None,
                asset_type: "Laptop".to_string(),
                display_name: "Dell Latitude".to_string(),
                display_name_short: None,
                brand: None,
                model: None,
                serial_number: None,
                usage_location: None,
                adapter_number: None,
                warehouse: None,
                notes: None,
            }],
        )
        .expect("insert initial asset");

        conn.execute(
            "UPDATE assets SET status = 'assigned', updated_at = datetime('now') WHERE id = ?",
            params![inserted[0].id],
        )
        .expect("mark asset borrowed");

        let updated = upsert_assets_conn(
            &mut conn,
            vec![AssetUpsertInput {
                asset_code: "ASSET-001".to_string(),
                category_id: None,
                asset_type: "Laptop".to_string(),
                display_name: "Dell Latitude 7450".to_string(),
                display_name_short: None,
                brand: None,
                model: Some("7450".to_string()),
                serial_number: Some("SN-001".to_string()),
                usage_location: None,
                adapter_number: None,
                warehouse: None,
                notes: Some("Updated metadata".to_string()),
            }],
        )
        .expect("update existing asset");

        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].asset_code, "ASSET-001");
        assert_eq!(updated[0].display_name, "Dell Latitude 7450");
        assert_eq!(updated[0].model.as_deref(), Some("7450"));
        assert_eq!(updated[0].status, "assigned");
    }

    #[test]
    fn assets_table_persists_dashboard_metadata_columns() {
        let conn = open_test_connection();

        conn.execute(
            r#"
            INSERT INTO assets(
              asset_code,
              category_id,
              asset_type,
              display_name,
              display_name_short,
              usage_location,
              adapter_number,
              status,
              created_at,
              updated_at
            )
                        VALUES(?, ?, ?, ?, ?, ?, ?, 'assigned', datetime('now'), datetime('now'))
            "#,
            params![
                "VNMON709",
                category_id(&conn, "monitor"),
                "Monitor",
                "Dell 24 Monitor",
                "Mon709",
                "office",
                "ADP-709",
            ],
        )
        .expect("insert asset with dashboard metadata");

        let stored = conn
            .query_row(
                "SELECT display_name_short, usage_location, adapter_number FROM assets WHERE asset_code = ?",
                params!["VNMON709"],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .expect("load stored dashboard metadata");

        assert_eq!(stored.0.as_deref(), Some("Mon709"));
        assert_eq!(stored.1.as_deref(), Some("office"));
        assert_eq!(stored.2.as_deref(), Some("ADP-709"));
    }

    #[test]
    fn asset_dashboard_summary_counts_serialized_and_quantity_inventory() {
        let conn = open_test_connection();
        let laptop_category_id = category_id(&conn, "laptop");
        let mouse_category_id = category_id(&conn, "mouse");

        conn.execute(
            r#"
            INSERT INTO assets(asset_code, category_id, asset_type, display_name, status, created_at, updated_at)
            VALUES
              ('VNLAP001', ?, 'Laptop', 'Dell 1', 'in_stock', datetime('now'), datetime('now')),
              ('VNLAP002', ?, 'Laptop', 'Dell 2', 'assigned', datetime('now'), datetime('now')),
              ('VNMON001', ?, 'Monitor', 'Mon001', 'assigned', datetime('now'), datetime('now'))
            "#,
            params![laptop_category_id, laptop_category_id, category_id(&conn, "monitor")],
        )
        .expect("insert serialized assets");

        conn.execute(
            r#"
            INSERT INTO stock_items(
              category_id, item_name, brand, model, warehouse, quantity_on_hand, assigned_quantity, note, created_at, updated_at
            )
            VALUES(?, 'Mouse Logi', 'Logitech', 'G102', 'HCM', 12, 3, NULL, datetime('now'), datetime('now'))
            "#,
            params![mouse_category_id],
        )
        .expect("insert quantity stock item");

        let summary = get_asset_dashboard_summary_conn(&conn).expect("load dashboard summary");

        assert_eq!(summary.total_serialized_assets, 3);
        assert_eq!(summary.serialized_in_stock, 1);
        assert_eq!(summary.serialized_assigned, 2);
        assert_eq!(summary.total_quantity_on_hand, 12);
        assert_eq!(summary.total_quantity_assigned, 3);
    }

    #[test]
    fn asset_dashboard_serialized_rows_include_holder_and_usage_location() {
        let mut conn = open_test_connection();
        let team_id = seed_team(&conn, "Examworks");
        let employee_row_id = seed_employee(&conn, "ASWVN1302", "Le The Hung", team_id);
        let request_id = seed_borrow_request(&conn, employee_row_id, "ASWVN1302", "Le The Hung");
        let monitor_category_id = category_id(&conn, "monitor");
        let laptop_category_id = category_id(&conn, "laptop");

        let inserted_assets = upsert_assets_conn(
            &mut conn,
            vec![
                AssetUpsertInput {
                    asset_code: "VNMON709".to_string(),
                    category_id: Some(monitor_category_id),
                    asset_type: "Monitor".to_string(),
                    display_name: "Monitor LG".to_string(),
                    display_name_short: Some("Mon709".to_string()),
                    brand: Some("LG".to_string()),
                    model: Some("LG 27".to_string()),
                    serial_number: Some("MON-SN-709".to_string()),
                    usage_location: Some("office".to_string()),
                    adapter_number: Some("ADP-709".to_string()),
                    warehouse: None,
                    notes: None,
                },
                AssetUpsertInput {
                    asset_code: "VNLAP050".to_string(),
                    category_id: Some(laptop_category_id),
                    asset_type: "Laptop".to_string(),
                    display_name: "Dell Latitude 5440".to_string(),
                    display_name_short: None,
                    brand: Some("Dell".to_string()),
                    model: Some("5440".to_string()),
                    serial_number: Some("LAP-SN-050".to_string()),
                    usage_location: None,
                    adapter_number: Some("LAP-ADP-050".to_string()),
                    warehouse: Some("HCM".to_string()),
                    notes: None,
                },
            ],
        )
        .expect("seed dashboard assets");

        conn.execute(
            "UPDATE assets SET status = 'assigned', updated_at = datetime('now') WHERE id = ?",
            params![inserted_assets[0].id],
        )
        .expect("mark monitor assigned");

        conn.execute(
            "INSERT INTO borrow_request_items(borrow_request_id, asset_id, asset_code_snapshot) VALUES(?, ?, ?)",
            params![request_id, inserted_assets[0].id, "VNMON709"],
        )
        .expect("insert borrow request item");
        conn.execute(
            "INSERT INTO asset_loans(asset_id, employee_id_fk, borrow_request_id, borrowed_at) VALUES(?, ?, ?, datetime('now'))",
            params![inserted_assets[0].id, employee_row_id, request_id],
        )
        .expect("insert active monitor loan");

        let rows = list_asset_dashboard_serialized_conn(&conn).expect("load serialized dashboard rows");
        let monitor = rows
            .iter()
            .find(|row| row.asset_code == "VNMON709")
            .expect("find monitor row");
        let laptop = rows
            .iter()
            .find(|row| row.asset_code == "VNLAP050")
            .expect("find laptop row");

        assert_eq!(monitor.category_code.as_deref(), Some("monitor"));
    assert!(monitor.computer_name.is_none());
        assert_eq!(monitor.display_name_short.as_deref(), Some("Mon709"));
        assert_eq!(monitor.adapter_number.as_deref(), Some("ADP-709"));
        assert_eq!(monitor.usage_location.as_deref(), Some("office"));
        assert_eq!(monitor.holder_employee_id.as_deref(), Some("ASWVN1302"));
        assert_eq!(monitor.holder_full_name.as_deref(), Some("Le The Hung"));
        assert_eq!(monitor.status, "assigned");

        assert_eq!(laptop.category_code.as_deref(), Some("laptop"));
        assert_eq!(laptop.computer_name.as_deref(), Some("ASWVNLAP050"));
        assert_eq!(laptop.adapter_number.as_deref(), Some("LAP-ADP-050"));
        assert!(laptop.holder_employee_id.is_none());
        assert!(laptop.usage_location.is_none());
    }

    #[test]
    fn asset_dashboard_quantity_rows_include_category_and_counts() {
        let conn = open_test_connection();
        let keyboard_category_id = category_id(&conn, "keyboard");
        let mouse_category_id = category_id(&conn, "mouse");

        conn.execute(
            r#"
            INSERT INTO stock_items(
              category_id, item_name, brand, model, warehouse, quantity_on_hand, assigned_quantity, note, created_at, updated_at
            )
            VALUES
              (?, 'Keyboard Logi', 'Logitech', 'K120', 'HCM', 10, 2, 'Floor stock', datetime('now'), datetime('now')),
              (?, 'Mouse Logi', 'Logitech', 'G102', 'HN', 50, 5, NULL, datetime('now'), datetime('now'))
            "#,
            params![keyboard_category_id, mouse_category_id],
        )
        .expect("insert quantity stock rows");

        let rows = list_asset_dashboard_quantity_conn(&conn).expect("load quantity dashboard rows");

        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| {
            row.category_code == "keyboard"
                && row.item_name == "Keyboard Logi"
                && row.model.as_deref() == Some("K120")
                && row.warehouse.as_deref() == Some("HCM")
                && row.quantity_on_hand == 10
                && row.assigned_quantity == 2
        }));
        assert!(rows.iter().any(|row| {
            row.category_code == "mouse"
                && row.item_name == "Mouse Logi"
                && row.quantity_on_hand == 50
                && row.assigned_quantity == 5
        }));
    }

    #[test]
    fn update_stock_item_quantity_only_mutates_stock_items() {
        let mut conn = open_test_connection();
        let mouse_category_id = category_id(&conn, "mouse");

        conn.execute(
            r#"
            INSERT INTO stock_items(
              category_id, item_name, brand, model, warehouse, quantity_on_hand, assigned_quantity, note, created_at, updated_at
            )
            VALUES(?, 'Mouse Logi', 'Logitech', 'G102', 'HCM', 50, 5, NULL, datetime('now'), datetime('now'))
            "#,
            params![mouse_category_id],
        )
        .expect("insert stock item");
        let stock_item_id = conn.last_insert_rowid();

        conn.execute(
            r#"
            INSERT INTO assets(asset_code, category_id, asset_type, display_name, status, created_at, updated_at)
            VALUES('VNLAP900', ?, 'Laptop', 'Dell Latitude 5440', 'assigned', datetime('now'), datetime('now'))
            "#,
            params![category_id(&conn, "laptop")],
        )
        .expect("insert unrelated asset");

        let updated = update_stock_item_quantity_conn(
            &mut conn,
            StockItemQuantityUpdateInput {
                stock_item_id,
                quantity_on_hand: 40,
                assigned_quantity: 7,
            },
        )
        .expect("update stock item quantity");

        assert_eq!(updated.quantity_on_hand, 40);
        assert_eq!(updated.assigned_quantity, 7);

        let asset_status = conn
            .query_row(
                "SELECT status FROM assets WHERE asset_code = 'VNLAP900'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("load unrelated asset status");
        assert_eq!(asset_status, "assigned");
    }

    #[test]
    fn seeded_asset_category_prefixes_cover_laptop_family_and_monitor_codes() {
        let conn = open_test_connection();

        let laptop_prefixes = load_category_prefix_values(&conn, "laptop");
        let monitor_prefixes = load_category_prefix_values(&conn, "monitor");

        assert_eq!(
            laptop_prefixes,
            vec![
                "VNLAP".to_string(),
                "VNIMACPRO".to_string(),
                "VNMACAIR".to_string(),
                "VNMACPRO".to_string(),
            ]
        );
        assert_eq!(monitor_prefixes, vec!["VNMON".to_string()]);
    }

    #[test]
    fn asset_category_prefix_lookup_resolves_asset_code_family() {
        let conn = open_test_connection();
        let expected_laptop_id = conn
            .query_row(
                "SELECT id FROM asset_categories WHERE category_code = 'laptop'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("load laptop category id");
        let expected_monitor_id = conn
            .query_row(
                "SELECT id FROM asset_categories WHERE category_code = 'monitor'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("load monitor category id");

        let laptop_category =
            load_asset_category_by_prefix_conn(&conn, "VNMACPRO003").expect("resolve laptop prefix");
        let monitor_category =
            load_asset_category_by_prefix_conn(&conn, "VNMON709").expect("resolve monitor prefix");

        assert_eq!(
            laptop_category.as_ref().map(|record| record.id),
            Some(expected_laptop_id)
        );
        assert_eq!(
            monitor_category.as_ref().map(|record| record.id),
            Some(expected_monitor_id)
        );
        assert_eq!(
            laptop_category.as_ref().map(|record| record.tracking_mode.as_str()),
            Some("serialized")
        );
        assert_eq!(
            monitor_category
                .as_ref()
                .map(|record| record.tracking_mode.as_str()),
            Some("serialized")
        );
    }

    #[test]
    fn asset_category_prefix_lookup_prefers_longest_match_over_primary_flag() {
        let conn = open_test_connection();

        conn.execute(
            r#"
            INSERT INTO asset_categories(
              category_code,
              category_name,
              tracking_mode,
              prefix_code,
              qr_required,
              is_active,
              created_at,
              updated_at
            )
            VALUES('device_family', 'Device Family', 'serialized', 'VN', 0, 1, datetime('now'), datetime('now'))
            "#,
            [],
        )
        .expect("insert broader device category");

        let device_category_id = conn
            .query_row(
                "SELECT id FROM asset_categories WHERE category_code = 'device_family'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("load broader device category id");
        let laptop_category_id = conn
            .query_row(
                "SELECT id FROM asset_categories WHERE category_code = 'laptop'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("load laptop category id");

        upsert_asset_category_prefix_conn(&conn, device_category_id, "VN", true, true)
            .expect("seed shorter broad prefix");

        let resolved =
            load_asset_category_by_prefix_conn(&conn, "VNMACPRO003").expect("resolve longest prefix");

        assert_eq!(resolved.as_ref().map(|record| record.id), Some(laptop_category_id));
    }

    #[test]
    fn asset_category_prefixes_reject_duplicate_active_values() {
        let conn = open_test_connection();

        let mouse_category_id = conn
            .query_row(
                "SELECT id FROM asset_categories WHERE category_code = 'mouse'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("load mouse category id");

        let duplicate_error = conn
            .execute(
                r#"
                INSERT INTO asset_category_prefixes(
                  category_id,
                  prefix_value,
                  is_primary,
                  is_active,
                  created_at,
                  updated_at
                )
                VALUES(?, ?, 0, 1, datetime('now'), datetime('now'))
                "#,
                params![mouse_category_id, "VNLAP"],
            )
            .expect_err("duplicate active prefix should be rejected");

        let message = duplicate_error.to_string().to_lowercase();
        assert!(
            message.contains("unique") || message.contains("constraint"),
            "expected uniqueness error, got: {duplicate_error}"
        );
    }

    #[test]
    fn asset_category_prefixes_reject_sql_wildcard_characters() {
        let conn = open_test_connection();
        let mouse_category_id = conn
            .query_row(
                "SELECT id FROM asset_categories WHERE category_code = 'mouse'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("load mouse category id");

        let wildcard_error = upsert_asset_category_prefix_conn(&conn, mouse_category_id, "VN%", false, true)
            .expect_err("wildcard characters should be rejected");

        assert!(
            wildcard_error.contains("wildcard"),
            "expected wildcard validation error, got: {wildcard_error}"
        );
    }

    #[test]
    fn asset_category_prefixes_reject_multiple_active_primary_values_per_category() {
        let conn = open_test_connection();
        let laptop_category_id = conn
            .query_row(
                "SELECT id FROM asset_categories WHERE category_code = 'laptop'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("load laptop category id");

        let duplicate_primary_error = conn
            .execute(
                r#"
                INSERT INTO asset_category_prefixes(
                  category_id,
                  prefix_value,
                  is_primary,
                  is_active,
                  created_at,
                  updated_at
                )
                VALUES(?, ?, 1, 1, datetime('now'), datetime('now'))
                "#,
                params![laptop_category_id, "VNLAPPRIMARY2"],
            )
            .expect_err("second active primary should be rejected");

        let message = duplicate_primary_error.to_string().to_lowercase();
        assert!(
            message.contains("unique") || message.contains("constraint"),
            "expected active primary uniqueness error, got: {duplicate_primary_error}"
        );
    }

    #[test]
    fn seeded_asset_categories_include_tracking_mode_and_prefix_rules() {
        let conn = open_test_connection();

        let categories = list_asset_categories_conn(&conn).expect("list seeded asset categories");

        assert!(
            categories.iter().any(|category| {
                category.category_code == "laptop"
                    && category.tracking_mode == "serialized"
                    && category.prefix_code.as_deref() == Some("VNLAP")
                    && category.qr_required
                    && category.is_active
            }),
            "expected seeded laptop serialized category with prefix"
        );
        assert!(
            categories.iter().any(|category| {
                category.category_code == "mouse"
                    && category.tracking_mode == "quantity"
                    && category.prefix_code.is_none()
                    && !category.qr_required
                    && category.is_active
            }),
            "expected seeded mouse quantity category"
        );
    }

    #[test]
    fn create_asset_category_supports_multiple_prefixes() {
        let conn = open_test_connection();

        let created = upsert_asset_category_conn(
            &conn,
            AssetCategoryUpsertInput {
                id: None,
                category_code: "dock".to_string(),
                category_name: "Dock Station".to_string(),
                tracking_mode: "serialized".to_string(),
                qr_required: false,
                prefixes: vec![
                    AssetCategoryPrefixInput {
                        prefix_value: "VNDOCK".to_string(),
                        is_primary: true,
                    },
                    AssetCategoryPrefixInput {
                        prefix_value: "DOCK".to_string(),
                        is_primary: false,
                    },
                ],
            },
        )
        .expect("create dock category");

        assert_eq!(created.category_code, "dock");
        assert_eq!(created.category_name, "Dock Station");
        assert_eq!(created.tracking_mode, "serialized");
        assert_eq!(
            created
                .prefixes
                .iter()
                .map(|prefix| (prefix.prefix_value.clone(), prefix.is_primary))
                .collect::<Vec<_>>(),
            vec![
                ("VNDOCK".to_string(), true),
                ("DOCK".to_string(), false),
            ]
        );
    }

    #[test]
    fn list_asset_category_details_returns_prefixes_for_all_categories() {
        let conn = open_test_connection();

        upsert_asset_category_conn(
            &conn,
            AssetCategoryUpsertInput {
                id: None,
                category_code: "dock".to_string(),
                category_name: "Dock Station".to_string(),
                tracking_mode: "serialized".to_string(),
                qr_required: false,
                prefixes: vec![
                    AssetCategoryPrefixInput {
                        prefix_value: "VNDOCK".to_string(),
                        is_primary: true,
                    },
                    AssetCategoryPrefixInput {
                        prefix_value: "DOCK".to_string(),
                        is_primary: false,
                    },
                ],
            },
        )
        .expect("create dock category");

        let details = list_asset_category_details_conn(&conn).expect("list category details");
        let dock = details
            .iter()
            .find(|detail| detail.category_code == "dock")
            .expect("dock detail should be present");
        let laptop = details
            .iter()
            .find(|detail| detail.category_code == "laptop")
            .expect("laptop detail should be present");

        assert_eq!(
            dock.prefixes
                .iter()
                .map(|prefix| prefix.prefix_value.as_str())
                .collect::<Vec<_>>(),
            vec!["VNDOCK", "DOCK"]
        );
        assert!(
            !laptop.prefixes.is_empty(),
            "expected seeded laptop category to keep its prefixes"
        );
    }

    #[test]
    fn update_asset_category_changes_label_tracking_mode_and_prefixes() {
        let conn = open_test_connection();

        let created = upsert_asset_category_conn(
            &conn,
            AssetCategoryUpsertInput {
                id: None,
                category_code: "dock".to_string(),
                category_name: "Dock Station".to_string(),
                tracking_mode: "serialized".to_string(),
                qr_required: false,
                prefixes: vec![AssetCategoryPrefixInput {
                    prefix_value: "VNDOCK".to_string(),
                    is_primary: true,
                }],
            },
        )
        .expect("create dock category");

        let updated = upsert_asset_category_conn(
            &conn,
            AssetCategoryUpsertInput {
                id: Some(created.id),
                category_code: "dock".to_string(),
                category_name: "Docking Peripheral".to_string(),
                tracking_mode: "quantity".to_string(),
                qr_required: false,
                prefixes: vec![
                    AssetCategoryPrefixInput {
                        prefix_value: "VNDOCK".to_string(),
                        is_primary: false,
                    },
                    AssetCategoryPrefixInput {
                        prefix_value: "VNHUB".to_string(),
                        is_primary: true,
                    },
                ],
            },
        )
        .expect("update dock category");

        assert_eq!(updated.category_name, "Docking Peripheral");
        assert_eq!(updated.tracking_mode, "quantity");
        assert_eq!(updated.prefix_code.as_deref(), Some("VNHUB"));
        assert_eq!(
            updated
                .prefixes
                .iter()
                .map(|prefix| (prefix.prefix_value.clone(), prefix.is_primary))
                .collect::<Vec<_>>(),
            vec![
                ("VNHUB".to_string(), true),
                ("VNDOCK".to_string(), false),
            ]
        );
    }

    #[test]
    fn deactivate_asset_category_preserves_categories_referenced_by_assets_or_stock_items() {
        let conn = open_test_connection();
        let monitor_category_id = category_id(&conn, "monitor");
        let mouse_category_id = category_id(&conn, "mouse");

        conn.execute(
            r#"
            INSERT INTO assets(asset_code, category_id, asset_type, display_name, status, created_at, updated_at)
            VALUES('VNMON888', ?, 'Monitor', 'Dell 24 Monitor', 'in_stock', datetime('now'), datetime('now'))
            "#,
            params![monitor_category_id],
        )
        .expect("seed monitor asset");

        conn.execute(
            r#"
            INSERT INTO stock_items(
              category_id, item_name, brand, model, warehouse, quantity_on_hand, assigned_quantity, note, created_at, updated_at
            )
            VALUES(?, 'Mouse Logi', 'Logitech', 'G102', 'HCM', 8, 2, NULL, datetime('now'), datetime('now'))
            "#,
            params![mouse_category_id],
        )
        .expect("seed mouse stock");

        let deactivated_monitor =
            deactivate_asset_category_conn(&conn, monitor_category_id).expect("deactivate monitor category");
        let deactivated_mouse =
            deactivate_asset_category_conn(&conn, mouse_category_id).expect("deactivate mouse category");

        assert!(!deactivated_monitor.is_active);
        assert!(!deactivated_mouse.is_active);
        assert_eq!(
            conn.query_row(
                "SELECT is_active FROM asset_categories WHERE id = ?",
                params![monitor_category_id],
                |row| row.get::<_, i64>(0),
            )
            .expect("load monitor active flag"),
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT is_active FROM asset_categories WHERE id = ?",
                params![mouse_category_id],
                |row| row.get::<_, i64>(0),
            )
            .expect("load mouse active flag"),
            0
        );
    }

    #[test]
    fn asset_category_upsert_rejects_duplicate_active_prefixes() {
        let conn = open_test_connection();

        let duplicate_error = upsert_asset_category_conn(
            &conn,
            AssetCategoryUpsertInput {
                id: None,
                category_code: "dock".to_string(),
                category_name: "Dock Station".to_string(),
                tracking_mode: "serialized".to_string(),
                qr_required: false,
                prefixes: vec![AssetCategoryPrefixInput {
                    prefix_value: "VNLAP".to_string(),
                    is_primary: true,
                }],
            },
        )
        .expect_err("duplicate laptop prefix should be rejected");

        assert!(
            duplicate_error.to_lowercase().contains("prefix"),
            "expected duplicate prefix error, got: {duplicate_error}"
        );
    }
}

fn normalize_asset_category_code(value: String) -> Result<String, String> {
    let raw = require_text(value, "categoryCode")?;
    let normalized = raw.trim().to_ascii_lowercase().replace(' ', "_");
    if normalized
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    {
        Ok(normalized)
    } else {
        Err("categoryCode can only contain letters, numbers, hyphen, underscore, and spaces".to_string())
    }
}

fn normalize_asset_tracking_mode(value: String) -> Result<String, String> {
    let normalized = require_text(value, "trackingMode")?.to_ascii_lowercase();
    match normalized.as_str() {
        "serialized" | "quantity" => Ok(normalized),
        _ => Err("trackingMode must be either 'serialized' or 'quantity'".to_string()),
    }
}

fn normalize_asset_category_prefix_inputs(
    prefixes: Vec<AssetCategoryPrefixInput>,
) -> Result<Vec<AssetCategoryPrefixInput>, String> {
    let mut seen = HashSet::new();
    let mut normalized_prefixes = Vec::with_capacity(prefixes.len());
    let mut primary_count = 0_i32;

    for prefix in prefixes {
        let Some(normalized_prefix) = normalize_asset_category_prefix(&prefix.prefix_value) else {
            return Err("asset category prefix cannot be blank".to_string());
        };
        if normalized_prefix.contains('%') || normalized_prefix.contains('_') {
            return Err("asset category prefix cannot contain SQL wildcard characters (% or _)".to_string());
        }
        if !seen.insert(normalized_prefix.clone()) {
            return Err(format!("duplicate prefix '{normalized_prefix}' in category payload"));
        }
        if prefix.is_primary {
            primary_count += 1;
        }
        normalized_prefixes.push(AssetCategoryPrefixInput {
            prefix_value: normalized_prefix,
            is_primary: prefix.is_primary,
        });
    }

    if primary_count > 1 {
        return Err("asset category can only have one primary prefix".to_string());
    }

    if !normalized_prefixes.is_empty() && primary_count == 0 {
        normalized_prefixes[0].is_primary = true;
    }

    Ok(normalized_prefixes)
}

fn with_asset_category_savepoint<T, F>(conn: &Connection, f: F) -> Result<T, String>
where
    F: FnOnce(&Connection) -> Result<T, String>,
{
    conn.execute_batch("SAVEPOINT asset_category_change")
        .map_err(|err| format!("failed to start asset category savepoint: {err}"))?;

    match f(conn) {
        Ok(value) => {
            conn.execute_batch("RELEASE SAVEPOINT asset_category_change")
                .map_err(|err| format!("failed to release asset category savepoint: {err}"))?;
            Ok(value)
        }
        Err(err) => {
            let _ = conn.execute_batch(
                "ROLLBACK TO SAVEPOINT asset_category_change; RELEASE SAVEPOINT asset_category_change",
            );
            Err(err)
        }
    }
}
