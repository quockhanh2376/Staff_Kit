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
    pub asset_type: String,
    pub display_name: String,
    pub model: Option<String>,
    pub serial_number: Option<String>,
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
    asset_type: &str,
    display_name: &str,
    model: Option<&str>,
    serial_number: Option<&str>,
    notes: Option<&str>,
) -> Result<i64, String> {
    executor
        .execute(
            r#"
            INSERT INTO assets(
              asset_code,
              asset_type,
              display_name,
              model,
              serial_number,
              notes,
              status,
              created_at,
              updated_at
            )
            VALUES(?, ?, ?, ?, ?, ?, 'in_stock', datetime('now'), datetime('now'))
            "#,
            params![
                asset_code,
                asset_type,
                display_name,
                model,
                serial_number,
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

pub(crate) fn create_asset_tx(
    tx: &Transaction<'_>,
    input: &AssetUpsertInput,
) -> Result<AssetRecord, String> {
    let asset_code = require_text(input.asset_code.clone(), "assetCode")?.to_uppercase();
    let asset_type = require_text(input.asset_type.clone(), "assetType")?;
    let display_name = require_text(input.display_name.clone(), "displayName")?;
    let model = normalize_optional_text(input.model.clone());
    let serial_number = normalize_optional_text(input.serial_number.clone());
    let notes = normalize_optional_text(input.notes.clone());

    let asset_id = insert_asset_stmt(
        tx,
        asset_code.as_str(),
        asset_type.as_str(),
        display_name.as_str(),
        model.as_deref(),
        serial_number.as_deref(),
        notes.as_deref(),
    )?;

    load_asset_record_by_id_conn(tx, asset_id)
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
        let asset_type = require_text(input.asset_type, "assetType")?;
        let display_name = require_text(input.display_name, "displayName")?;
        let model = normalize_optional_text(input.model);
        let serial_number = normalize_optional_text(input.serial_number);
        let notes = normalize_optional_text(input.notes);

        tx.execute(
            r#"
            INSERT INTO assets(
              asset_code,
              asset_type,
              display_name,
              model,
              serial_number,
              notes,
              status,
              created_at,
              updated_at
            )
            VALUES(?, ?, ?, ?, ?, ?, 'in_stock', datetime('now'), datetime('now'))
            ON CONFLICT(asset_code) DO UPDATE SET
              asset_type = excluded.asset_type,
              display_name = excluded.display_name,
              model = excluded.model,
              serial_number = excluded.serial_number,
              notes = excluded.notes,
              updated_at = datetime('now')
            "#,
            params![
                asset_code.as_str(),
                asset_type.as_str(),
                display_name.as_str(),
                model.as_deref(),
                serial_number.as_deref(),
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

    #[test]
    fn upsert_assets_conn_inserts_new_assets_as_in_stock() {
        let mut conn = open_test_connection();

        let records = upsert_assets_conn(
            &mut conn,
            vec![AssetUpsertInput {
                asset_code: "asset-001".to_string(),
                asset_type: "Laptop".to_string(),
                display_name: "Dell Latitude".to_string(),
                model: Some("7440".to_string()),
                serial_number: Some("SN-001".to_string()),
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
                asset_type: "Laptop".to_string(),
                display_name: "Dell Latitude".to_string(),
                model: None,
                serial_number: None,
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
                asset_type: "Laptop".to_string(),
                display_name: "Dell Latitude 7450".to_string(),
                model: Some("7450".to_string()),
                serial_number: Some("SN-001".to_string()),
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
    fn seeded_asset_categories_include_tracking_mode_and_prefix_rules() {
        let conn = open_test_connection();

        let categories = list_asset_categories_conn(&conn).expect("list seeded asset categories");

        assert!(
            categories.iter().any(|category| {
                category.category_code == "laptop"
                    && category.tracking_mode == "serialized"
                    && category.prefix_code.as_deref() == Some("ASWVNLAP")
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
