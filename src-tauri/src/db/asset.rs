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
              warehouse,
              notes,
              status,
              created_at,
              updated_at
            )
            VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'in_stock', datetime('now'), datetime('now'))
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
              warehouse,
              notes,
              status,
              created_at,
              updated_at
            )
            VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'in_stock', datetime('now'), datetime('now'))
            ON CONFLICT(asset_code) DO UPDATE SET
              category_id = excluded.category_id,
              asset_type = excluded.asset_type,
              display_name = excluded.display_name,
              display_name_short = excluded.display_name_short,
              brand = excluded.brand,
              model = excluded.model,
              serial_number = excluded.serial_number,
              usage_location = excluded.usage_location,
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
              asset_type,
              display_name,
              display_name_short,
              usage_location,
              status,
              created_at,
              updated_at
            )
            VALUES(?, ?, ?, ?, ?, 'assigned', datetime('now'), datetime('now'))
            "#,
            params!["VNMON709", "Monitor", "Dell 24 Monitor", "Mon709", "office"],
        )
        .expect("insert asset with dashboard metadata");

        let stored = conn
            .query_row(
                "SELECT display_name_short, usage_location FROM assets WHERE asset_code = ?",
                params!["VNMON709"],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .expect("load stored dashboard metadata");

        assert_eq!(stored.0.as_deref(), Some("Mon709"));
        assert_eq!(stored.1.as_deref(), Some("office"));
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
}
