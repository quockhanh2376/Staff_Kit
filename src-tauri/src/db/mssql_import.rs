use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{params, OptionalExtension, Transaction};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use super::column::{upsert_dynamic_field_definitions_for_map, upsert_dynamic_fields_tx};
use super::employee::UpsertAction;
use super::schema::{AZURE_AD_ACCOUNT_FIELD_KEY, STAFF_GROUP_EMPLOYEE_LIST};
use super::{
    humanize_sqlite_error, normalize_optional_text, open_runtime_connection, require_text,
};

const MSSQL_HOST_ENV: &str = "STAFFKIT_MSSQL_HOST";
const MSSQL_PORT_ENV: &str = "STAFFKIT_MSSQL_PORT";
const MSSQL_USER_ENV: &str = "STAFFKIT_MSSQL_USER";
const MSSQL_PASSWORD_ENV: &str = "STAFFKIT_MSSQL_PASSWORD";
const MSSQL_CONNECT_ATTEMPTS: usize = 3;
const MSSQL_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MSSQL_CONNECT_RETRY_DELAY: Duration = Duration::from_millis(250);

const DEFAULT_MSSQL_QUERY: &str = r#"
WITH StaffList AS
(
    SELECT
        S.[Code],
        S.[Name],
        S.[NickName],
        COALESCE(S.[WorkEmail], S.[WorkEmail2]) AS [WorkEmail],
        ASPU.UserName AS [AzureAD],
        ROW_NUMBER() OVER (
            PARTITION BY S.[Code]
            ORDER BY S.[Code]
        ) AS RowNum
    FROM [AssetManagement].[dbo].[Staffs] AS S
    INNER JOIN [AssetManagement].[dbo].[AspNetUsers] AS ASPU
        ON S.Id = ASPU.StaffId
    WHERE S.[Resigned] = 0
)
SELECT TOP (1000)
    [Code],
    [Name],
    [NickName],
    [WorkEmail],
    [AzureAD]
FROM StaffList
WHERE RowNum = 1
ORDER BY [Code]
"#;

const EE_LIST_SYNC_QUERY: &str = r#"
WITH StaffList AS
(
    SELECT
        S.[Code],
        S.[Name],
        S.[NickName],
        COALESCE(S.[WorkEmail], S.[WorkEmail2]) AS [WorkEmail],
        ASPU.[UserName] AS [AzureAD],
        ROW_NUMBER() OVER (
            PARTITION BY UPPER(LTRIM(RTRIM(S.[Code])))
            ORDER BY S.[Id], ASPU.[UserName]
        ) AS RowNum
    FROM [AssetManagement].[dbo].[Staffs] AS S
    INNER JOIN [AssetManagement].[dbo].[AspNetUsers] AS ASPU
        ON S.[Id] = ASPU.[StaffId]
    WHERE S.[Resigned] = 0
)
SELECT [Code], [Name], [NickName], [WorkEmail], [AzureAD]
FROM StaffList
WHERE RowNum = 1
ORDER BY [Code]
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalEmployeeListRecord {
    employee_id: String,
    full_name: String,
    nick_name: Option<String>,
    email: Option<String>,
    azure_ad_account: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeSyncCounts {
    pub new: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub missing: usize,
    pub conflicts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeSyncRow {
    pub code: String,
    pub status: String,
    pub source_name: Option<String>,
    pub local_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeSyncDiff {
    pub counts: EmployeeSyncCounts,
    pub rows: Vec<EmployeeSyncRow>,
    pub conflicts: Vec<EmployeeSyncConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeSyncConflict {
    pub code: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeListSyncPreview {
    pub preview_id: String,
    pub created_at: String,
    pub counts: EmployeeSyncCounts,
    pub rows: Vec<EmployeeSyncRow>,
    pub conflicts: Vec<EmployeeSyncConflict>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeListSyncResult {
    pub counts: EmployeeSyncCounts,
    pub completed_at: String,
}

#[derive(Debug, Clone)]
struct PendingEmployeeListSync {
    source: Vec<MssqlStaffRecord>,
    local: Vec<LocalEmployeeListRecord>,
    non_employee_list_codes: Vec<String>,
}

static PENDING_EMPLOYEE_LIST_SYNCS: OnceLock<Mutex<HashMap<String, PendingEmployeeListSync>>> =
    OnceLock::new();

pub fn build_connection_string(host: &str, port: u16, user: &str, password: &str) -> String {
    format!(
        "server=tcp:{host},{port};database=AssetManagement;uid={user};pwd={password};encrypt=true;TrustServerCertificate=yes;"
    )
}

const MSSQL_READ_ONLY_CHECK: &str = r#"
SELECT CAST(CASE WHEN
    COALESCE(IS_SRVROLEMEMBER('sysadmin'), 1) = 1 OR
    COALESCE(IS_SRVROLEMEMBER('securityadmin'), 1) = 1 OR
    COALESCE(IS_SRVROLEMEMBER('serveradmin'), 1) = 1 OR
    COALESCE(IS_MEMBER('db_owner'), 1) = 1 OR
    COALESCE(IS_MEMBER('db_datawriter'), 1) = 1 OR
    COALESCE(IS_MEMBER('db_ddladmin'), 1) = 1 OR
    COALESCE(IS_MEMBER('db_securityadmin'), 1) = 1 OR
    COALESCE(IS_MEMBER('db_accessadmin'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'INSERT'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'UPDATE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'DELETE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'ALTER'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'CONTROL'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'CREATE TABLE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'ALTER ANY SCHEMA'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'ALTER ANY USER'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.Staffs', 'OBJECT', 'INSERT'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.Staffs', 'OBJECT', 'UPDATE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.Staffs', 'OBJECT', 'DELETE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.Staffs', 'OBJECT', 'ALTER'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.Staffs', 'OBJECT', 'CONTROL'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.AspNetUsers', 'OBJECT', 'INSERT'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.AspNetUsers', 'OBJECT', 'UPDATE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.AspNetUsers', 'OBJECT', 'DELETE'), 1) = 1
    OR COALESCE(HAS_PERMS_BY_NAME('dbo.AspNetUsers', 'OBJECT', 'ALTER'), 1) = 1
    OR COALESCE(HAS_PERMS_BY_NAME('dbo.AspNetUsers', 'OBJECT', 'CONTROL'), 1) = 1
    THEN 1 ELSE 0 END AS bit) AS HasWriteAccess
"#;

async fn ensure_mssql_login_is_read_only(client: &mut MssqlClient) -> Result<(), String> {
    let rows = client
        .query(MSSQL_READ_ONLY_CHECK, &[])
        .await
        .map_err(|_| "Could not verify read-only access for the MSSQL source login.".to_string())?
        .into_first_result()
        .await
        .map_err(|_| "Could not verify read-only access for the MSSQL source login.".to_string())?;
    let has_write_access = rows
        .first()
        .and_then(|row| row.get::<bool, _>(0))
        .ok_or_else(|| {
            "Could not verify read-only access for the MSSQL source login.".to_string()
        })?;
    if has_write_access {
        return Err("MSSQL source login has write-level permissions. Configure a read-only login before syncing.".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlConnectionDefaults {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
}

impl Default for MssqlConnectionDefaults {
    fn default() -> Self {
        let port = std::env::var(MSSQL_PORT_ENV)
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(1433);

        Self {
            host: std::env::var(MSSQL_HOST_ENV).unwrap_or_default(),
            port,
            user: std::env::var(MSSQL_USER_ENV).unwrap_or_default(),
            password: std::env::var(MSSQL_PASSWORD_ENV).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlStaffRecord {
    pub code: String,
    pub name: String,
    pub nick_name: Option<String>,
    pub work_email: Option<String>,
    pub azure_ad_account: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlImportPreview {
    pub total_rows: usize,
    pub records: Vec<MssqlStaffRecord>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlImportReport {
    pub total_rows: usize,
    pub imported: u32,
    pub updated: u32,
    pub failed: u32,
    pub errors: Vec<MssqlImportError>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlImportError {
    pub code: String,
    pub action: String,
    pub error: Option<String>,
}

type MssqlClient = Client<Compat<TcpStream>>;

fn format_mssql_connect_error(error: impl std::fmt::Display) -> String {
    let detail = error.to_string();
    let normalized = detail.to_ascii_lowercase();

    if normalized.contains("10054")
        || normalized.contains("forcibly closed")
        || normalized.contains("connection reset")
    {
        return format!(
            "MSSQL connection failed: the server closed the connection during TLS/authentication negotiation (Windows error 10054). Verify the SQL Server port, TLS/encryption settings, firewall, and SQL credentials. Details: {detail}"
        );
    }

    if normalized.contains("tls")
        || normalized.contains("handshake")
        || normalized.contains("certificate")
    {
        return format!(
            "MSSQL connection failed during TLS negotiation. Verify the SQL Server encryption settings and certificate configuration. Details: {detail}"
        );
    }

    format!("MSSQL authentication failed. Verify the SQL username and password. Details: {detail}")
}

fn is_retryable_mssql_connect_error(detail: &str) -> bool {
    let normalized = detail.to_ascii_lowercase();
    normalized.contains("10054")
        || normalized.contains("10060")
        || normalized.contains("forcibly closed")
        || normalized.contains("connection reset")
        || normalized.contains("timed out")
        || normalized.contains("timeout")
        || normalized.contains("connection error")
        || normalized.contains("i/o")
}

fn format_mssql_tcp_connect_error(addr: &str, detail: &str) -> String {
    let normalized = detail.to_ascii_lowercase();
    if normalized.contains("10060") || normalized.contains("timed out") {
        return format!(
            "MSSQL network connection failed for {addr}: the server did not respond within the timeout (Windows error 10060). Check that the computer is connected to the company network/VPN, SQL Server is running, TCP is enabled, and firewall port 1433 is open. Details: {detail}"
        );
    }

    format!(
        "MSSQL network connection failed for {addr}. Check the host, port, network/VPN, and firewall settings. Details: {detail}"
    )
}

fn append_import_log(app: &AppHandle, message: impl AsRef<str>) {
    let Ok(mut log_dir) = app.path().app_local_data_dir() else {
        eprintln!("[mssql_import] {}", message.as_ref());
        return;
    };

    log_dir.push("logs");
    if let Err(err) = fs::create_dir_all(&log_dir) {
        eprintln!(
            "[mssql_import] failed to create log directory {}: {err}",
            log_dir.to_string_lossy()
        );
        eprintln!("[mssql_import] {}", message.as_ref());
        return;
    }

    let log_path = log_dir.join("mssql-import.log");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    match OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(mut file) => {
            let _ = writeln!(file, "[{timestamp}] {}", message.as_ref());
        }
        Err(err) => {
            eprintln!(
                "[mssql_import] failed to open log file {}: {err}",
                log_path.to_string_lossy()
            );
            eprintln!("[mssql_import] {}", message.as_ref());
        }
    }
}

fn get_required_mssql_text(
    row: &tiberius::Row,
    index: usize,
    label: &str,
) -> Result<String, String> {
    if let Some(value) = row.get::<&str, _>(index) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    if let Some(value) = row.get::<i32, _>(index) {
        return Ok(value.to_string());
    }

    if let Some(value) = row.get::<i64, _>(index) {
        return Ok(value.to_string());
    }

    Err(format!("MSSQL row missing {label} column"))
}

fn get_optional_mssql_text(row: &tiberius::Row, index: Option<usize>) -> Option<String> {
    let index = index?;
    if let Some(value) = row.get::<&str, _>(index) {
        return Some(value.trim().to_string()).filter(|value| !value.is_empty());
    }

    None
}

fn find_mssql_column_index(row: &tiberius::Row, aliases: &[&str]) -> Option<usize> {
    row.columns().iter().position(|column| {
        aliases
            .iter()
            .any(|alias| column.name().eq_ignore_ascii_case(alias))
    })
}

fn normalized_staff_code(value: &str) -> String {
    value.trim().to_uppercase()
}

fn normalize_sync_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn compare_employee_list(
    source: &[MssqlStaffRecord],
    local: &[LocalEmployeeListRecord],
    non_employee_list_codes: &[String],
) -> EmployeeSyncDiff {
    let local_by_code: HashMap<String, &LocalEmployeeListRecord> = local
        .iter()
        .map(|record| (normalized_staff_code(&record.employee_id), record))
        .collect();
    let non_list_codes: std::collections::HashSet<String> = non_employee_list_codes
        .iter()
        .map(|code| normalized_staff_code(code))
        .collect();
    let source_codes: std::collections::HashSet<String> = source
        .iter()
        .map(|record| normalized_staff_code(&record.code))
        .collect();

    let mut diff = EmployeeSyncDiff {
        counts: EmployeeSyncCounts::default(),
        rows: Vec::new(),
        conflicts: Vec::new(),
    };

    for record in source {
        let code_key = normalized_staff_code(&record.code);
        if non_list_codes.contains(&code_key) {
            diff.counts.conflicts += 1;
            diff.conflicts.push(EmployeeSyncConflict {
                code: record.code.clone(),
                reason: "This Staff Code belongs to another employee group.".to_string(),
            });
            diff.rows.push(EmployeeSyncRow {
                code: record.code.clone(),
                status: "conflict".to_string(),
                source_name: Some(record.name.clone()),
                local_name: None,
            });
            continue;
        }

        let local_record = local_by_code.get(&code_key).copied();
        let status = match local_record {
            None => {
                diff.counts.new += 1;
                "new"
            }
            Some(existing) if sync_record_matches(record, existing) => {
                diff.counts.unchanged += 1;
                "unchanged"
            }
            Some(_) => {
                diff.counts.changed += 1;
                "changed"
            }
        };
        diff.rows.push(EmployeeSyncRow {
            code: record.code.clone(),
            status: status.to_string(),
            source_name: Some(record.name.clone()),
            local_name: local_record.map(|existing| existing.full_name.clone()),
        });
    }

    for record in local {
        if !source_codes.contains(&normalized_staff_code(&record.employee_id)) {
            diff.counts.missing += 1;
            diff.rows.push(EmployeeSyncRow {
                code: record.employee_id.clone(),
                status: "missing".to_string(),
                source_name: None,
                local_name: Some(record.full_name.clone()),
            });
        }
    }
    diff
}

fn sync_record_matches(source: &MssqlStaffRecord, local: &LocalEmployeeListRecord) -> bool {
    source.name.trim() == local.full_name.trim()
        && normalize_sync_text(source.nick_name.as_deref())
            == normalize_sync_text(local.nick_name.as_deref())
        && normalize_sync_text(source.work_email.as_deref()).map(|value| value.to_lowercase())
            == normalize_sync_text(local.email.as_deref()).map(|value| value.to_lowercase())
        && normalize_sync_text(source.azure_ad_account.as_deref())
            == normalize_sync_text(local.azure_ad_account.as_deref())
}

fn upsert_mssql_staff_record(
    tx: &Transaction<'_>,
    record: &MssqlStaffRecord,
    staff_group: &str,
) -> Result<UpsertAction, String> {
    let employee_id = require_text(record.code.clone(), "employeeId")?
        .trim()
        .to_uppercase();
    let full_name = require_text(record.name.clone(), "fullName")?;
    let email =
        normalize_optional_text(record.work_email.clone()).map(|value| value.to_lowercase());
    let nick_name = normalize_optional_text(record.nick_name.clone());

    let existing_id: Option<i64> = tx
        .query_row(
            "SELECT id FROM employees WHERE employee_id = ?",
            params![employee_id.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("failed to check employee existence: {err}"))?;

    if let Some(id) = existing_id {
        tx.execute(
            r#"
            UPDATE employees
            SET
              full_name = ?,
              nick_name = COALESCE(?, nick_name),
              email = ?,
              updated_at = datetime('now')
            WHERE id = ?
            "#,
            params![
                full_name.as_str(),
                nick_name.as_deref(),
                email.as_deref(),
                id
            ],
        )
        .map_err(humanize_sqlite_error)?;

        upsert_mssql_azure_ad_account(tx, id, record.azure_ad_account.as_deref())?;

        return Ok(UpsertAction::Updated);
    }

    tx.execute(
        r#"
        INSERT INTO employees (
          employee_id,
          full_name,
          nick_name,
          email,
          staff_group,
          updated_at
        ) VALUES (
          ?, ?, ?, ?, ?, datetime('now')
        )
        "#,
        params![
            employee_id.as_str(),
            full_name.as_str(),
            nick_name.as_deref(),
            email.as_deref(),
            staff_group,
        ],
    )
    .map_err(humanize_sqlite_error)?;

    let inserted_id = tx.last_insert_rowid();
    upsert_mssql_azure_ad_account(tx, inserted_id, record.azure_ad_account.as_deref())?;

    Ok(UpsertAction::Inserted)
}

fn upsert_mssql_azure_ad_account(
    tx: &Transaction<'_>,
    employee_id: i64,
    value: Option<&str>,
) -> Result<(), String> {
    let Some(value) = normalize_optional_text(value.map(str::to_string)) else {
        return Ok(());
    };

    let mut fields = HashMap::new();
    fields.insert(AZURE_AD_ACCOUNT_FIELD_KEY.to_string(), value);
    upsert_dynamic_field_definitions_for_map(tx, &fields)?;
    upsert_dynamic_fields_tx(tx, employee_id, &fields)
}

async fn connect(connection_string: &str) -> Result<MssqlClient, String> {
    let config = Config::from_ado_string(connection_string)
        .map_err(|err| format!("invalid connection string: {err}"))?;
    let addr = config.get_addr();
    let mut last_error = None;

    for attempt in 1..=MSSQL_CONNECT_ATTEMPTS {
        let tcp_result = timeout(MSSQL_CONNECT_TIMEOUT, TcpStream::connect(&addr)).await;
        let tcp = match tcp_result {
            Ok(Ok(tcp)) => tcp,
            Ok(Err(err)) => {
                last_error = Some(format_mssql_tcp_connect_error(&addr, &err.to_string()));
                if attempt == MSSQL_CONNECT_ATTEMPTS {
                    break;
                }
                sleep(MSSQL_CONNECT_RETRY_DELAY).await;
                continue;
            }
            Err(_) => {
                last_error = Some(format_mssql_tcp_connect_error(
                    &addr,
                    &format!(
                        "connection timed out after {} seconds",
                        MSSQL_CONNECT_TIMEOUT.as_secs()
                    ),
                ));
                if attempt == MSSQL_CONNECT_ATTEMPTS {
                    break;
                }
                sleep(MSSQL_CONNECT_RETRY_DELAY).await;
                continue;
            }
        };

        tcp.set_nodelay(true).ok();

        match Client::connect(config.clone(), tcp.compat_write()).await {
            Ok(client) => return Ok(client),
            Err(err) => {
                let detail = err.to_string();
                let formatted_error = format_mssql_connect_error(&detail);
                if !is_retryable_mssql_connect_error(&detail) || attempt == MSSQL_CONNECT_ATTEMPTS {
                    return Err(formatted_error);
                }
                last_error = Some(formatted_error);
                sleep(MSSQL_CONNECT_RETRY_DELAY).await;
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        format!("MSSQL connection failed for {addr}. No connection attempt completed.")
    }))
}

pub async fn get_mssql_connection_defaults() -> MssqlConnectionDefaults {
    MssqlConnectionDefaults::default()
}

pub async fn test_mssql_connection(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
) -> Result<bool, String> {
    let conn_str = build_connection_string(host, port, user, password);
    let mut client = connect(&conn_str).await?;
    ensure_mssql_login_is_read_only(&mut client).await?;
    let stream = client
        .query("SELECT 1 AS test", &[])
        .await
        .map_err(|err| format!("MSSQL query failed: {err}"))?;

    let _ = stream.into_results().await;
    Ok(true)
}

async fn fetch_employee_list_sync_source(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
) -> Result<Vec<MssqlStaffRecord>, String> {
    let conn_str = build_connection_string(host, port, user, password);
    let mut client = connect(&conn_str).await?;
    ensure_mssql_login_is_read_only(&mut client).await?;
    let stream = client
        .query(EE_LIST_SYNC_QUERY, &[])
        .await
        .map_err(|_| "Could not read the configured employee source query.".to_string())?;
    let rows = stream
        .into_first_result()
        .await
        .map_err(|_| "Could not read the configured employee source results.".to_string())?;
    let mut records = Vec::with_capacity(rows.len());
    for row in &rows {
        let code = get_required_mssql_text(
            row,
            find_mssql_column_index(row, &["Code"])
                .ok_or_else(|| "Employee source result is missing Code.".to_string())?,
            "Code",
        )?;
        let name = get_required_mssql_text(
            row,
            find_mssql_column_index(row, &["Name"])
                .ok_or_else(|| "Employee source result is missing Name.".to_string())?,
            "Name",
        )?;
        records.push(MssqlStaffRecord {
            code,
            name,
            nick_name: get_optional_mssql_text(row, find_mssql_column_index(row, &["NickName"])),
            work_email: get_optional_mssql_text(row, find_mssql_column_index(row, &["WorkEmail"])),
            azure_ad_account: get_optional_mssql_text(
                row,
                find_mssql_column_index(row, &["AzureAD"]),
            ),
        });
    }
    Ok(records)
}

fn read_employee_list_snapshot(
    app: &AppHandle,
) -> Result<(Vec<LocalEmployeeListRecord>, Vec<String>), String> {
    let path = super::resolve_database_path_read_only(app)?;
    let conn = super::open_encrypted_read_only_connection(&path)?;
    read_employee_list_snapshot_conn(&conn)
}

fn read_employee_list_snapshot_conn(
    conn: &rusqlite::Connection,
) -> Result<(Vec<LocalEmployeeListRecord>, Vec<String>), String> {
    let mut local = Vec::new();
    let mut statement = conn
        .prepare(
            r#"
            SELECT e.employee_id, e.full_name, e.nick_name, e.email, azure.value
            FROM employees e
            LEFT JOIN employee_dynamic_values azure
              ON azure.employee_id = e.id AND azure.field_key = ?1
            WHERE COALESCE(NULLIF(TRIM(e.staff_group), ''), 'employee_list') = ?2
            ORDER BY UPPER(TRIM(e.employee_id)), e.id
            "#,
        )
        .map_err(|err| format!("failed to read EE List preview: {err}"))?;
    let rows = statement
        .query_map(
            params![AZURE_AD_ACCOUNT_FIELD_KEY, STAFF_GROUP_EMPLOYEE_LIST],
            |row| {
                Ok(LocalEmployeeListRecord {
                    employee_id: row.get(0)?,
                    full_name: row.get(1)?,
                    nick_name: row.get(2)?,
                    email: row.get(3)?,
                    azure_ad_account: row.get(4)?,
                })
            },
        )
        .map_err(|err| format!("failed to read EE List preview: {err}"))?;
    for row in rows {
        local.push(row.map_err(|err| format!("failed to read EE List preview row: {err}"))?);
    }

    let mut statement = conn
        .prepare(
            r#"SELECT employee_id FROM employees
               WHERE COALESCE(NULLIF(TRIM(staff_group), ''), 'employee_list') <> ?1
               ORDER BY UPPER(TRIM(employee_id)), id"#,
        )
        .map_err(|err| format!("failed to check employee group conflicts: {err}"))?;
    let codes = statement
        .query_map([STAFF_GROUP_EMPLOYEE_LIST], |row| row.get::<_, String>(0))
        .map_err(|err| format!("failed to check employee group conflicts: {err}"))?;
    let mut non_employee_list_codes = Vec::new();
    for code in codes {
        non_employee_list_codes
            .push(code.map_err(|err| format!("failed to read employee group conflict: {err}"))?);
    }
    Ok((local, non_employee_list_codes))
}

fn next_employee_sync_preview_id() -> String {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!(
        "{epoch}-{}",
        NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

pub async fn preview_employee_list_sync(
    app: &AppHandle,
    host: &str,
    port: u16,
    user: &str,
    password: &str,
) -> Result<EmployeeListSyncPreview, String> {
    let source = fetch_employee_list_sync_source(host, port, user, password).await?;
    let (local, non_employee_list_codes) = read_employee_list_snapshot(app)?;
    let diff = compare_employee_list(&source, &local, &non_employee_list_codes);
    let preview_id = next_employee_sync_preview_id();
    let created_at = chrono::Utc::now().to_rfc3339();
    let pending = PendingEmployeeListSync {
        source,
        local,
        non_employee_list_codes,
    };
    let previews = PENDING_EMPLOYEE_LIST_SYNCS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut previews = previews
        .lock()
        .map_err(|_| "Employee sync preview state is unavailable. Please retry.".to_string())?;
    previews.insert(preview_id.clone(), pending);
    while previews.len() > 4 {
        if let Some(oldest) = previews.keys().min().cloned() {
            previews.remove(&oldest);
        }
    }
    Ok(EmployeeListSyncPreview {
        preview_id,
        created_at,
        counts: diff.counts,
        rows: diff.rows,
        conflicts: diff.conflicts,
    })
}

pub async fn apply_employee_list_sync(
    app: &AppHandle,
    preview_id: &str,
) -> Result<EmployeeListSyncResult, String> {
    let credentials = MssqlConnectionDefaults::default();
    if credentials.host.trim().is_empty()
        || credentials.user.trim().is_empty()
        || credentials.password.is_empty()
    {
        return Err(
            "Missing MSSQL configuration. Configure the read-only source login and retry."
                .to_string(),
        );
    }
    let source = fetch_employee_list_sync_source(
        credentials.host.trim(),
        credentials.port,
        credentials.user.trim(),
        &credentials.password,
    )
    .await?;
    let previews = PENDING_EMPLOYEE_LIST_SYNCS.get_or_init(|| Mutex::new(HashMap::new()));
    let pending = previews
        .lock()
        .map_err(|_| {
            "Employee sync preview state is unavailable. Please create a new preview.".to_string()
        })?
        .get(preview_id)
        .cloned()
        .ok_or_else(|| {
            "This preview has expired. Create a new preview before applying.".to_string()
        })?;
    if source != pending.source {
        return Err(
            "Source or local EE List changed after preview. Create a new preview before applying."
                .to_string(),
        );
    }
    let db_path = super::resolve_database_path_read_only(app)?;
    let mut conn = super::open_encrypted_connection(&db_path)?;
    let tx = conn
        .transaction()
        .map_err(|err| format!("failed to start EE List update transaction: {err}"))?;
    let (local, non_employee_list_codes) = read_employee_list_snapshot_conn(&tx)?;
    if local != pending.local || non_employee_list_codes != pending.non_employee_list_codes {
        return Err(
            "Source or local EE List changed after preview. Create a new preview before applying."
                .to_string(),
        );
    }
    let diff = compare_employee_list(&source, &local, &non_employee_list_codes);
    if diff.counts.conflicts > 0 {
        return Err(
            "Resolve the Staff Code group conflicts before applying this preview.".to_string(),
        );
    }
    let local_by_code: HashMap<String, &LocalEmployeeListRecord> = local
        .iter()
        .map(|record| (normalized_staff_code(&record.employee_id), record))
        .collect();
    for record in &source {
        match local_by_code.get(&normalized_staff_code(&record.code)) {
            None => insert_employee_list_record(&tx, record)?,
            Some(existing) if !sync_record_matches(record, existing) => {
                update_employee_list_record(&tx, record, &existing.employee_id)?;
            }
            Some(_) => {}
        }
    }
    tx.commit()
        .map_err(|err| format!("failed to commit EE List update: {err}"))?;

    if let Ok(mut previews) = previews.lock() {
        previews.remove(preview_id);
    }
    Ok(EmployeeListSyncResult {
        counts: diff.counts,
        completed_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn insert_employee_list_record(
    tx: &Transaction<'_>,
    record: &MssqlStaffRecord,
) -> Result<(), String> {
    let code = normalized_staff_code(&record.code);
    tx.execute(
        r#"INSERT INTO employees (employee_id, full_name, nick_name, email, staff_group, updated_at)
           VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))"#,
        params![
            code,
            record.name.trim(),
            normalize_sync_text(record.nick_name.as_deref()),
            normalize_sync_text(record.work_email.as_deref()).map(|value| value.to_lowercase()),
            STAFF_GROUP_EMPLOYEE_LIST
        ],
    )
    .map_err(|err| format!("failed to insert EE List employee: {err}"))?;
    upsert_sync_azure_ad(
        tx,
        tx.last_insert_rowid(),
        record.azure_ad_account.as_deref(),
    )
}

fn update_employee_list_record(
    tx: &Transaction<'_>,
    record: &MssqlStaffRecord,
    local_code: &str,
) -> Result<(), String> {
    let employee_id: i64 = tx
        .query_row(
            "SELECT id FROM employees WHERE UPPER(TRIM(employee_id)) = ?1 AND COALESCE(NULLIF(TRIM(staff_group), ''), 'employee_list') = ?2",
            params![normalized_staff_code(local_code), STAFF_GROUP_EMPLOYEE_LIST],
            |row| row.get(0),
        )
        .map_err(|err| format!("EE List row changed before update: {err}"))?;
    tx.execute(
        "UPDATE employees SET full_name = ?1, nick_name = ?2, email = ?3, updated_at = datetime('now') WHERE id = ?4",
        params![record.name.trim(), normalize_sync_text(record.nick_name.as_deref()), normalize_sync_text(record.work_email.as_deref()).map(|value| value.to_lowercase()), employee_id],
    )
    .map_err(|err| format!("failed to update EE List employee: {err}"))?;
    upsert_sync_azure_ad(tx, employee_id, record.azure_ad_account.as_deref())
}

fn upsert_sync_azure_ad(
    tx: &Transaction<'_>,
    employee_id: i64,
    value: Option<&str>,
) -> Result<(), String> {
    let value = normalize_sync_text(value);
    if let Some(value) = value {
        let mut fields = HashMap::new();
        fields.insert(AZURE_AD_ACCOUNT_FIELD_KEY.to_string(), value);
        upsert_dynamic_field_definitions_for_map(tx, &fields)?;
        upsert_dynamic_fields_tx(tx, employee_id, &fields)
    } else {
        tx.execute(
            "DELETE FROM employee_dynamic_values WHERE employee_id = ?1 AND field_key = ?2",
            params![employee_id, AZURE_AD_ACCOUNT_FIELD_KEY],
        )
        .map_err(humanize_sqlite_error)?;
        Ok(())
    }
}

pub async fn preview_mssql_staff(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    query: Option<&str>,
) -> Result<MssqlImportPreview, String> {
    let conn_str = build_connection_string(host, port, user, password);
    let sql = query.unwrap_or(DEFAULT_MSSQL_QUERY);
    let mut client = connect(&conn_str).await?;
    ensure_mssql_login_is_read_only(&mut client).await?;

    let stream = client
        .query(sql, &[])
        .await
        .map_err(|err| format!("MSSQL query failed: {err}"))?;

    let rows = stream
        .into_first_result()
        .await
        .map_err(|err| format!("failed to read MSSQL results: {err}"))?;

    let mut records = Vec::with_capacity(rows.len());

    for row in &rows {
        let code_index = find_mssql_column_index(row, &["Code"])
            .ok_or_else(|| "MSSQL row missing Code column".to_string())?;
        let name_index = find_mssql_column_index(row, &["Name"])
            .ok_or_else(|| "MSSQL row missing Name column".to_string())?;
        let code = get_required_mssql_text(row, code_index, "Code")?;
        let name = get_required_mssql_text(row, name_index, "Name")?;
        let nick_name =
            get_optional_mssql_text(row, find_mssql_column_index(row, &["NickName", "Nickname"]));
        let email =
            get_optional_mssql_text(row, find_mssql_column_index(row, &["WorkEmail", "Email"]));
        let azure_ad_account = get_optional_mssql_text(
            row,
            find_mssql_column_index(row, &["AzureAD", "AzureAdAccount", "AzureADAccount"]),
        );

        records.push(MssqlStaffRecord {
            code,
            name,
            nick_name,
            work_email: email,
            azure_ad_account,
        });
    }

    Ok(MssqlImportPreview {
        total_rows: records.len(),
        records,
    })
}

pub async fn import_mssql_staff(
    app: &AppHandle,
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    query: Option<&str>,
    staff_group: Option<&str>,
) -> Result<MssqlImportReport, String> {
    let target_group = staff_group.unwrap_or(STAFF_GROUP_EMPLOYEE_LIST);
    append_import_log(
        app,
        format!(
            "start host={host} port={port} user={user} staff_group={target_group} query={}",
            if query.is_some() { "custom" } else { "default" }
        ),
    );

    let preview = match preview_mssql_staff(host, port, user, password, query).await {
        Ok(preview) => {
            append_import_log(
                app,
                format!("preview_ok total_rows={}", preview.records.len()),
            );
            preview
        }
        Err(err) => {
            append_import_log(app, format!("preview_failed error={err}"));
            return Err(format!("MSSQL import failed during preview: {err}"));
        }
    };

    let mut conn = match open_runtime_connection(app) {
        Ok(conn) => conn,
        Err(err) => {
            append_import_log(app, format!("sqlite_open_failed error={err}"));
            return Err(format!("MSSQL import failed before database write: {err}"));
        }
    };
    let tx = conn.transaction().map_err(|err| {
        append_import_log(app, format!("transaction_start_failed error={err}"));
        format!("MSSQL import failed to start database transaction: {err}")
    })?;

    let mut report = MssqlImportReport {
        total_rows: preview.records.len(),
        imported: 0,
        updated: 0,
        failed: 0,
        errors: Vec::new(),
    };

    for record in &preview.records {
        match upsert_mssql_staff_record(&tx, record, target_group) {
            Ok(UpsertAction::Inserted) => report.imported += 1,
            Ok(UpsertAction::Updated) => report.updated += 1,
            Err(err) => {
                report.failed += 1;
                append_import_log(app, format!("row_failed code={} error={err}", record.code));
                report.errors.push(MssqlImportError {
                    code: record.code.clone(),
                    action: "failed".to_string(),
                    error: Some(err),
                });
            }
        }
    }

    tx.commit().map_err(|err| {
        append_import_log(app, format!("commit_failed error={err}"));
        format!("MSSQL import failed to commit database transaction: {err}")
    })?;

    append_import_log(
        app,
        format!(
            "finish total_rows={} imported={} updated={} failed={}",
            report.total_rows, report.imported, report.updated, report.failed
        ),
    );

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn ee_list_sync_query_is_the_fixed_approved_read_only_query() {
        let query = EE_LIST_SYNC_QUERY.to_ascii_lowercase();

        assert!(query.contains("inner join [assetmanagement].[dbo].[aspnetusers]"));
        assert!(query.contains("where s.[resigned] = 0"));
        assert!(query.contains("coalesce(s.[workemail], s.[workemail2]) as [workemail]"));
        assert!(query.contains("as [azuread]"));
        assert!(query.contains("partition by upper(ltrim(rtrim(s.[code])))"));
        assert!(!query.contains("top ("));
        assert!(!query.contains("insert "));
        assert!(!query.contains("update "));
        assert!(!query.contains("delete "));
    }

    #[test]
    fn employee_sync_diff_matches_trimmed_case_insensitive_staff_code() {
        let source = vec![MssqlStaffRecord {
            code: " aswvn1325 ".to_string(),
            name: "Name A".to_string(),
            nick_name: None,
            work_email: None,
            azure_ad_account: None,
        }];
        let local = vec![LocalEmployeeListRecord {
            employee_id: "ASWVN1325".to_string(),
            full_name: "Name A".to_string(),
            nick_name: None,
            email: None,
            azure_ad_account: None,
        }];

        let diff = compare_employee_list(&source, &local, &[]);

        assert_eq!(diff.counts.unchanged, 1);
        assert_eq!(diff.counts.new, 0);
        assert_eq!(diff.counts.changed, 0);
        assert_eq!(diff.counts.missing, 0);
    }

    #[test]
    fn employee_sync_diff_classifies_new_changed_unchanged_missing_and_group_conflict() {
        let source = vec![
            MssqlStaffRecord {
                code: "ASWVN1325".into(),
                name: "Name A".into(),
                nick_name: None,
                work_email: None,
                azure_ad_account: None,
            },
            MssqlStaffRecord {
                code: "ASWVN1326".into(),
                name: "Name B Updated".into(),
                nick_name: None,
                work_email: None,
                azure_ad_account: None,
            },
            MssqlStaffRecord {
                code: "ASWVN1327".into(),
                name: "Name C".into(),
                nick_name: None,
                work_email: None,
                azure_ad_account: None,
            },
        ];
        let local = vec![
            LocalEmployeeListRecord {
                employee_id: "ASWVN1325".into(),
                full_name: "Name A".into(),
                nick_name: None,
                email: None,
                azure_ad_account: None,
            },
            LocalEmployeeListRecord {
                employee_id: "ASWVN1326".into(),
                full_name: "Name B".into(),
                nick_name: None,
                email: None,
                azure_ad_account: None,
            },
            LocalEmployeeListRecord {
                employee_id: "ASWVN1328".into(),
                full_name: "Name D".into(),
                nick_name: None,
                email: None,
                azure_ad_account: None,
            },
        ];

        let diff = compare_employee_list(&source, &local, &["ASWVN1327".into()]);

        assert_eq!(diff.counts.unchanged, 1);
        assert_eq!(diff.counts.changed, 1);
        assert_eq!(diff.counts.new, 0);
        assert_eq!(diff.counts.missing, 1);
        assert_eq!(diff.conflicts.len(), 1);
        assert_eq!(diff.conflicts[0].code, "ASWVN1327");
    }

    #[test]
    fn approved_sync_helpers_update_only_mapped_employee_list_values() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        super::super::configure_connection(&conn).expect("configure sqlite pragmas");
        super::super::apply_migrations(&conn).expect("apply database migrations");
        conn.execute(
            "INSERT INTO employees (employee_id, full_name, staff_group) VALUES ('ASWVN1325', 'Old Name', 'employee_list')",
            [],
        ).expect("seed EE List employee");
        conn.execute(
            "INSERT INTO employees (employee_id, full_name, staff_group) VALUES ('ASWVN1400', 'Other Group', 'onboarding')",
            [],
        ).expect("seed other group employee");

        let mut conn = conn;
        let tx = conn.transaction().expect("start approved transaction");
        update_employee_list_record(
            &tx,
            &MssqlStaffRecord {
                code: "ASWVN1325".into(),
                name: "Canonical Name".into(),
                nick_name: Some("Nick".into()),
                work_email: Some("person@example.com".into()),
                azure_ad_account: Some("person@corp.example".into()),
            },
            "ASWVN1325",
        )
        .expect("update approved EE List employee");
        insert_employee_list_record(
            &tx,
            &MssqlStaffRecord {
                code: "ASWVN1326".into(),
                name: "New Employee".into(),
                nick_name: None,
                work_email: None,
                azure_ad_account: None,
            },
        )
        .expect("insert approved EE List employee");
        tx.commit().expect("commit approved transaction");

        let (name, nick, email, group): (String, Option<String>, Option<String>, String) = conn
            .query_row(
                "SELECT full_name, nick_name, email, staff_group FROM employees WHERE employee_id = 'ASWVN1325'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("read updated employee");
        assert_eq!(name, "Canonical Name");
        assert_eq!(nick.as_deref(), Some("Nick"));
        assert_eq!(email.as_deref(), Some("person@example.com"));
        assert_eq!(group, STAFF_GROUP_EMPLOYEE_LIST);
        let other_name: String = conn
            .query_row(
                "SELECT full_name FROM employees WHERE employee_id = 'ASWVN1400'",
                [],
                |row| row.get(0),
            )
            .expect("read untouched other group employee");
        assert_eq!(other_name, "Other Group");
        let new_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM employees WHERE employee_id = 'ASWVN1326' AND staff_group = 'employee_list'", [], |row| row.get(0))
            .expect("count inserted employee");
        assert_eq!(new_count, 1);
    }

    #[test]
    fn build_connection_string_requires_encryption_and_trusted_certificate() {
        let connection_string = build_connection_string("sql.example", 1433, "staff", "secret");

        assert!(connection_string.contains("server=tcp:sql.example,1433;"));
        assert!(connection_string.contains("uid=staff;"));
        assert!(connection_string.contains("pwd=secret;"));
        assert!(connection_string.contains("encrypt=true;"));
        assert!(connection_string.contains("TrustServerCertificate=yes;"));
        assert!(connection_string.contains("database=AssetManagement;"));
    }

    #[test]
    fn read_only_login_check_fails_closed_for_write_roles_and_table_permissions() {
        let check = MSSQL_READ_ONLY_CHECK.to_ascii_lowercase();
        assert!(check.contains("is_srvrolemember('sysadmin')"));
        assert!(check.contains("is_member('db_datawriter')"));
        assert!(check.contains("has_perms_by_name('dbo.staffs', 'object', 'update')"));
        assert!(check.contains("has_perms_by_name('dbo.aspnetusers', 'object', 'insert')"));
        assert!(check.contains("coalesce(") && check.contains("then 1 else 0 end as bit"));
    }

    #[test]
    fn upsert_existing_staff_record_updates_azure_ad_account() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        super::super::configure_connection(&conn).expect("configure sqlite pragmas");
        super::super::apply_migrations(&conn).expect("apply database migrations");
        conn.execute(
            "INSERT INTO employees (employee_id, full_name, staff_group) VALUES (?, ?, ?)",
            params!["ASWVN405", "Le Thanh Dat", STAFF_GROUP_EMPLOYEE_LIST],
        )
        .expect("seed existing employee");

        let mut conn = conn;
        let tx = conn.transaction().expect("start transaction");
        let record = MssqlStaffRecord {
            code: "ASWVN405".to_string(),
            name: "Lê Thanh Đạt".to_string(),
            nick_name: None,
            work_email: Some("d.lethanh@eml.com.au".to_string()),
            azure_ad_account: Some("d.le2@aswhiteglobal.com".to_string()),
        };

        assert!(matches!(
            upsert_mssql_staff_record(&tx, &record, STAFF_GROUP_EMPLOYEE_LIST),
            Ok(UpsertAction::Updated)
        ));
        tx.commit().expect("commit transaction");

        let azure_ad_account: Option<String> = conn
            .query_row(
                r#"
                SELECT v.value
                FROM employees e
                LEFT JOIN employee_dynamic_values v
                  ON v.employee_id = e.id
                 AND v.field_key = ?
                WHERE e.employee_id = ?
                "#,
                params![AZURE_AD_ACCOUNT_FIELD_KEY, "ASWVN405"],
                |row| row.get(0),
            )
            .expect("read updated AzureAD account");

        assert_eq!(azure_ad_account.as_deref(), Some("d.le2@aswhiteglobal.com"));
    }

    #[test]
    fn tcp_timeout_error_identifies_endpoint_and_network_action() {
        let message = format_mssql_tcp_connect_error(
            "10.184.0.19:1433",
            "connection timed out (os error 10060)",
        );

        assert!(message.contains("10.184.0.19:1433"));
        assert!(message.contains("Windows error 10060"));
        assert!(message.contains("network/VPN"));
    }

    #[test]
    fn upsert_mssql_staff_record_persists_nickname_and_azure_ad_account() {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        super::super::configure_connection(&conn).expect("configure sqlite pragmas");
        super::super::apply_migrations(&conn).expect("apply database migrations");

        let mut conn = conn;
        let tx = conn.transaction().expect("start transaction");
        let record = MssqlStaffRecord {
            code: "ASWVN001".to_string(),
            name: "Test User".to_string(),
            nick_name: Some("Tester".to_string()),
            work_email: Some("test@example.com".to_string()),
            azure_ad_account: Some("tester@example.com".to_string()),
        };

        assert!(matches!(
            upsert_mssql_staff_record(&tx, &record, STAFF_GROUP_EMPLOYEE_LIST),
            Ok(UpsertAction::Inserted)
        ));
        tx.commit().expect("commit transaction");

        let (nick_name, azure_ad_account): (Option<String>, Option<String>) = conn
            .query_row(
                r#"
                SELECT e.nick_name, v.value
                FROM employees e
                LEFT JOIN employee_dynamic_values v
                  ON v.employee_id = e.id
                 AND v.field_key = ?
                WHERE e.employee_id = ?
                "#,
                params![AZURE_AD_ACCOUNT_FIELD_KEY, "ASWVN001"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read imported employee");

        assert_eq!(nick_name.as_deref(), Some("Tester"));
        assert_eq!(azure_ad_account.as_deref(), Some("tester@example.com"));
    }
}
