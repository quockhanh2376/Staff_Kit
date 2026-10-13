use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{params, Transaction};
use serde::Serialize;
use tauri::AppHandle;
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use super::column::{upsert_dynamic_field_definitions_for_map, upsert_dynamic_fields_tx};
use super::schema::{AZURE_AD_ACCOUNT_FIELD_KEY, STAFF_GROUP_EMPLOYEE_LIST};
use super::{humanize_sqlite_error, open_runtime_connection};

const MSSQL_PORT_ENV: &str = "STAFFKIT_MSSQL_PORT";
const MSSQL_PASSWORD_ENV: &str = "STAFFKIT_MSSQL_PASSWORD";
const APPROVED_MSSQL_USERNAME: &str = "user_reader";
const MSSQL_ACCOUNT_ALLOWLIST_ERROR: &str = "Only the approved Staff Kit SQL login is allowed.";
const MSSQL_SERVER_ENV: &str = "STAFFKIT_MSSQL_SERVER";
const MSSQL_DATABASE_ENV: &str = "STAFFKIT_MSSQL_DATABASE";
const MSSQL_AUTH_TYPE_ENV: &str = "STAFFKIT_MSSQL_AUTH_TYPE";
const MSSQL_USERNAME_ENV: &str = "STAFFKIT_MSSQL_USERNAME";
const REMOTE_EE_SYNC_SECRET_REF: &str = "staffkit.remote-ee-sync.sql-password.v1";
const REMOTE_EE_SYNC_SETTING_PREFIX: &str = "remote_ee_sync_";
const MSSQL_CONNECT_ATTEMPTS: usize = 3;
const MSSQL_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MSSQL_CONNECT_RETRY_DELAY: Duration = Duration::from_millis(250);

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
    FROM [dbo].[Staffs] AS S
    INNER JOIN [dbo].[AspNetUsers] AS ASPU
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

const MSSQL_READ_ONLY_CHECK: &str = r#"
SELECT CAST(CASE WHEN
    COALESCE(IS_SRVROLEMEMBER('sysadmin'), 1) = 1 OR
    COALESCE(IS_SRVROLEMEMBER('securityadmin'), 1) = 1 OR
    COALESCE(IS_SRVROLEMEMBER('serveradmin'), 1) = 1 OR
    COALESCE(IS_SRVROLEMEMBER('bulkadmin'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(NULL, 'SERVER', 'CONTROL SERVER'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(NULL, 'SERVER', 'ALTER ANY DATABASE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(NULL, 'SERVER', 'ALTER ANY LOGIN'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(NULL, 'SERVER', 'ALTER ANY SERVER ROLE'), 1) = 1 OR
    COALESCE(IS_MEMBER('db_owner'), 1) = 1 OR
    COALESCE(IS_MEMBER('db_datawriter'), 1) = 1 OR
    COALESCE(IS_MEMBER('db_ddladmin'), 1) = 1 OR
    COALESCE(IS_MEMBER('db_securityadmin'), 1) = 1 OR
    COALESCE(IS_MEMBER('db_accessadmin'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'INSERT'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'UPDATE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'DELETE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'EXECUTE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'ALTER'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'CONTROL'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'CREATE TABLE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'CREATE PROCEDURE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'CREATE VIEW'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'ALTER ANY ROLE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'ALTER ANY SCHEMA'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'ALTER ANY USER'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME(DB_NAME(), 'DATABASE', 'TAKE OWNERSHIP'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo', 'SCHEMA', 'INSERT'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo', 'SCHEMA', 'UPDATE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo', 'SCHEMA', 'DELETE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo', 'SCHEMA', 'EXECUTE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo', 'SCHEMA', 'ALTER'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo', 'SCHEMA', 'CONTROL'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo', 'SCHEMA', 'TAKE OWNERSHIP'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.Staffs', 'OBJECT', 'INSERT'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.Staffs', 'OBJECT', 'UPDATE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.Staffs', 'OBJECT', 'DELETE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.Staffs', 'OBJECT', 'ALTER'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.Staffs', 'OBJECT', 'CONTROL'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.Staffs', 'OBJECT', 'TAKE OWNERSHIP'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.AspNetUsers', 'OBJECT', 'INSERT'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.AspNetUsers', 'OBJECT', 'UPDATE'), 1) = 1 OR
    COALESCE(HAS_PERMS_BY_NAME('dbo.AspNetUsers', 'OBJECT', 'DELETE'), 1) = 1
    OR COALESCE(HAS_PERMS_BY_NAME('dbo.AspNetUsers', 'OBJECT', 'ALTER'), 1) = 1
    OR COALESCE(HAS_PERMS_BY_NAME('dbo.AspNetUsers', 'OBJECT', 'CONTROL'), 1) = 1
    OR COALESCE(HAS_PERMS_BY_NAME('dbo.AspNetUsers', 'OBJECT', 'TAKE OWNERSHIP'), 1) = 1
    OR EXISTS (
        SELECT 1
        FROM sys.procedures AS P
        WHERE COALESCE(
            HAS_PERMS_BY_NAME(
                QUOTENAME(OBJECT_SCHEMA_NAME(P.object_id)) + '.' + QUOTENAME(P.name),
                'OBJECT',
                'EXECUTE'
            ),
            1
        ) = 1
    )
    OR EXISTS (
        SELECT 1
        FROM sys.fn_my_permissions(NULL, 'SERVER') AS EffectivePermission
        WHERE EffectivePermission.permission_name NOT IN (
            'CONNECT SQL',
            'VIEW ANY DATABASE',
            'VIEW ANY DEFINITION',
            'VIEW SERVER STATE',
            'VIEW SERVER PERFORMANCE STATE',
            'VIEW SERVER SECURITY STATE',
            'VIEW ANY ERROR LOG'
        )
    )
    OR NOT EXISTS (
        SELECT 1
        FROM sys.fn_my_permissions(NULL, 'SERVER') AS EffectivePermission
        WHERE EffectivePermission.permission_name = 'CONNECT SQL'
    )
    OR EXISTS (
        SELECT 1
        FROM sys.fn_my_permissions(DB_NAME(), 'DATABASE') AS EffectivePermission
        WHERE EffectivePermission.permission_name NOT IN (
            'CONNECT',
            'SELECT',
            'VIEW DEFINITION',
            'VIEW DATABASE STATE',
            'VIEW DATABASE PERFORMANCE STATE',
            'VIEW DATABASE SECURITY STATE',
            'SHOWPLAN'
        )
    )
    OR NOT EXISTS (
        SELECT 1
        FROM sys.fn_my_permissions(DB_NAME(), 'DATABASE') AS EffectivePermission
        WHERE EffectivePermission.permission_name = 'CONNECT'
    )
    OR EXISTS (
        SELECT 1
        FROM sys.schemas AS S
        CROSS APPLY sys.fn_my_permissions(S.name, 'SCHEMA') AS EffectivePermission
        WHERE EffectivePermission.permission_name NOT IN ('SELECT', 'VIEW DEFINITION')
    )
    OR EXISTS (
        SELECT 1
        FROM sys.objects AS O
        INNER JOIN sys.schemas AS S ON S.schema_id = O.schema_id
        CROSS APPLY sys.fn_my_permissions(
            QUOTENAME(S.name) + '.' + QUOTENAME(O.name),
            'OBJECT'
        ) AS EffectivePermission
        WHERE O.type IN ('U', 'V', 'P', 'PC', 'FN', 'IF', 'TF', 'FS', 'FT')
          AND EffectivePermission.permission_name NOT IN ('SELECT', 'VIEW DEFINITION')
    )
    OR EXISTS (
        SELECT 1
        FROM sys.database_principals AS P
        CROSS APPLY sys.fn_my_permissions(
            P.name,
            CASE P.type
                WHEN 'R' THEN 'ROLE'
                WHEN 'A' THEN 'APPLICATION ROLE'
                ELSE 'USER'
            END
        ) AS EffectivePermission
        WHERE EffectivePermission.permission_name NOT IN ('VIEW DEFINITION')
    )
    OR EXISTS (
        SELECT 1
        FROM sys.server_principals AS P
        CROSS APPLY sys.fn_my_permissions(P.name, 'LOGIN') AS EffectivePermission
        WHERE P.type <> 'R'
          AND EffectivePermission.permission_name NOT IN ('VIEW DEFINITION')
    )
    OR EXISTS (
        SELECT 1
        FROM sys.server_principals AS P
        WHERE P.type = 'R'
          AND COALESCE(
              HAS_PERMS_BY_NAME(P.name, 'SERVER ROLE', 'ANY'),
              1
          ) = 1
    )
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MssqlAuthType {
    SqlServer,
    WindowsIntegrated,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteEeSyncSettings {
    pub server: String,
    pub database: String,
    pub port: u16,
    pub auth_type: MssqlAuthType,
    pub username: String,
    pub has_saved_password: bool,
    pub environment_override: bool,
}

#[derive(Clone)]
struct MssqlConnectionConfig {
    server: String,
    database: String,
    port: u16,
    auth_type: MssqlAuthType,
    username: String,
    password: Option<String>,
    environment_override: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MssqlConnectionTestResult {
    pub status: String,
    pub reason: String,
    pub row_count: Option<usize>,
}

impl MssqlConnectionTestResult {
    fn pass(row_count: usize) -> Self {
        Self {
            status: "PASS".into(),
            reason: "Read-only access verified; approved Query B completed.".into(),
            row_count: Some(row_count),
        }
    }

    fn fail(reason: impl Into<String>) -> Self {
        Self {
            status: "FAIL".into(),
            reason: reason.into(),
            row_count: None,
        }
    }
}

impl MssqlConnectionConfig {
    fn safe_settings(&self, has_saved_password: bool) -> RemoteEeSyncSettings {
        RemoteEeSyncSettings {
            server: self.server.clone(),
            database: self.database.clone(),
            port: self.port,
            auth_type: self.auth_type,
            username: self.username.clone(),
            has_saved_password,
            environment_override: self.environment_override,
        }
    }
}

fn resolve_connection_config_from(
    env: &HashMap<String, String>,
    saved: Option<MssqlConnectionConfig>,
) -> Result<MssqlConnectionConfig, String> {
    let names = [
        MSSQL_SERVER_ENV,
        MSSQL_DATABASE_ENV,
        MSSQL_PORT_ENV,
        MSSQL_AUTH_TYPE_ENV,
        MSSQL_USERNAME_ENV,
        MSSQL_PASSWORD_ENV,
    ];
    if names.iter().any(|name| env.contains_key(*name)) {
        let server = env.get(MSSQL_SERVER_ENV).cloned().unwrap_or_default();
        let database = env.get(MSSQL_DATABASE_ENV).cloned().unwrap_or_default();
        let username = env.get(MSSQL_USERNAME_ENV).cloned().unwrap_or_default();
        let auth_type = match env
            .get(MSSQL_AUTH_TYPE_ENV)
            .map(String::as_str)
            .ok_or_else(|| {
                "MSSQL environment configuration is incomplete; authentication type is required."
                    .to_string()
            })? {
            "sql_server" => MssqlAuthType::SqlServer,
            "windows_integrated" => MssqlAuthType::WindowsIntegrated,
            _ => return Err("MSSQL environment configuration is incomplete or invalid.".into()),
        };
        validate_mssql_account(auth_type, &username)?;
        let port = match env.get(MSSQL_PORT_ENV) {
            Some(value) => value
                .parse::<u16>()
                .map_err(|_| "MSSQL environment configuration has an invalid port.".to_string())?,
            None => 1433,
        };
        let password = env.get(MSSQL_PASSWORD_ENV).cloned();
        if server.trim().is_empty()
            || database.trim().is_empty()
            || port == 0
            || (auth_type == MssqlAuthType::SqlServer
                && (username.trim().is_empty()
                    || password.as_deref().unwrap_or_default().is_empty()))
        {
            return Err("MSSQL environment configuration is incomplete; saved settings are not used as fallback.".into());
        }
        validate_mssql_server(&server)?;
        return Ok(MssqlConnectionConfig {
            server,
            database,
            port,
            auth_type,
            username,
            password,
            environment_override: true,
        });
    }
    let config = saved.ok_or_else(|| "Remote EE Sync settings are not configured.".to_string())?;
    validate_mssql_account(config.auth_type, &config.username)?;
    if config.server.trim().is_empty()
        || config.database.trim().is_empty()
        || config.port == 0
        || (config.auth_type == MssqlAuthType::SqlServer
            && (config.username.trim().is_empty()
                || config.password.as_deref().unwrap_or_default().is_empty()))
    {
        return Err("Remote EE Sync settings are incomplete.".into());
    }
    validate_mssql_server(&config.server)?;
    Ok(config)
}

fn validate_mssql_server(server: &str) -> Result<(), String> {
    let invalid = || {
        "Server must be a valid IPv4 address, IPv6 address, or DNS hostname; URL schemes are not supported.".to_string()
    };

    if server.is_empty()
        || server.trim() != server
        || server.chars().any(char::is_whitespace)
        || server.contains("://")
        || server.contains(['/', '\\', '?', '#', '@'])
    {
        return Err(invalid());
    }

    if server.parse::<IpAddr>().is_ok() {
        return Ok(());
    }

    // A four-part all-numeric host is intended as IPv4, so don't let a malformed
    // address silently fall through and be treated as a DNS name.
    let ipv4_parts: Vec<_> = server.split('.').collect();
    if ipv4_parts.len() == 4
        && ipv4_parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(invalid());
    }

    if server.contains(':') || server.starts_with('[') || server.ends_with(']') {
        return Err(invalid());
    }

    let hostname = server.strip_suffix('.').unwrap_or(server);
    if hostname.is_empty() || hostname.len() > 253 || !hostname.is_ascii() {
        return Err(invalid());
    }
    for label in hostname.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(invalid());
        }
    }

    Ok(())
}

fn process_mssql_environment() -> HashMap<String, String> {
    [
        MSSQL_SERVER_ENV,
        MSSQL_DATABASE_ENV,
        MSSQL_PORT_ENV,
        MSSQL_AUTH_TYPE_ENV,
        MSSQL_USERNAME_ENV,
        MSSQL_PASSWORD_ENV,
    ]
    .iter()
    .filter_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| ((*name).to_string(), value))
    })
    .collect()
}

pub fn get_remote_ee_sync_settings(app: &AppHandle) -> Result<RemoteEeSyncSettings, String> {
    let conn = open_runtime_connection(app)?;
    let read = |key: &str| {
        super::get_setting_value(&conn, &format!("{REMOTE_EE_SYNC_SETTING_PREFIX}{key}"))
    };
    let profile = MssqlConnectionConfig {
        server: read("server")?.unwrap_or_default(),
        database: read("database")?.unwrap_or_else(|| "AssetManagement".into()),
        port: read("port")?
            .and_then(|value| value.parse().ok())
            .unwrap_or(1433),
        auth_type: match read("auth_type")?.as_deref() {
            Some("windows_integrated") => MssqlAuthType::WindowsIntegrated,
            _ => MssqlAuthType::SqlServer,
        },
        username: read("username")?.unwrap_or_default(),
        password: None,
        environment_override: !process_mssql_environment().is_empty(),
    };
    Ok(profile.safe_settings(read_mssql_password().unwrap_or(None).is_some()))
}

pub fn save_remote_ee_sync_settings(
    app: &AppHandle,
    settings: RemoteEeSyncSettings,
    password: Option<String>,
) -> Result<(), String> {
    validate_remote_settings(&settings)?;
    let mut conn = open_runtime_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|_| "Could not save Remote EE Sync settings.".to_string())?;
    write_remote_profile_settings(&tx, &settings)?;
    tx.commit()
        .map_err(|_| "Could not save Remote EE Sync settings.".to_string())?;
    if settings.auth_type == MssqlAuthType::WindowsIntegrated {
        delete_mssql_password()?;
    } else if let Some(password) = password.filter(|value| !value.is_empty()) {
        write_mssql_password(&password)?;
    }
    Ok(())
}

fn write_remote_profile_settings(
    conn: &rusqlite::Connection,
    settings: &RemoteEeSyncSettings,
) -> Result<(), String> {
    for (key, value) in [
        ("server", settings.server.clone()),
        ("database", settings.database.trim().to_string()),
        ("port", settings.port.to_string()),
        (
            "auth_type",
            match settings.auth_type {
                MssqlAuthType::SqlServer => "sql_server",
                MssqlAuthType::WindowsIntegrated => "windows_integrated",
            }
            .to_string(),
        ),
        ("username", settings.username.trim().to_string()),
        ("secret_ref", REMOTE_EE_SYNC_SECRET_REF.to_string()),
    ] {
        super::set_setting_value(
            conn,
            &format!("{REMOTE_EE_SYNC_SETTING_PREFIX}{key}"),
            Some(&value),
        )?;
    }
    Ok(())
}

pub fn remove_remote_ee_sync_password() -> Result<(), String> {
    delete_mssql_password()
}

fn validate_remote_settings(settings: &RemoteEeSyncSettings) -> Result<(), String> {
    validate_mssql_account(settings.auth_type, &settings.username)?;
    if settings.server.trim().is_empty()
        || settings.database.trim().is_empty()
        || settings.port == 0
    {
        return Err("Server, database, and a valid port are required.".into());
    }
    validate_mssql_server(&settings.server)?;
    if settings.auth_type == MssqlAuthType::SqlServer && settings.username.trim().is_empty() {
        return Err("Username is required for SQL Server authentication.".into());
    }
    Ok(())
}

fn validate_mssql_account(auth_type: MssqlAuthType, username: &str) -> Result<(), String> {
    if auth_type == MssqlAuthType::SqlServer && username == APPROVED_MSSQL_USERNAME {
        Ok(())
    } else {
        Err(MSSQL_ACCOUNT_ALLOWLIST_ERROR.into())
    }
}

fn load_connection_config(app: &AppHandle) -> Result<MssqlConnectionConfig, String> {
    let env = process_mssql_environment();
    if !env.is_empty() {
        return resolve_connection_config_from(&env, None);
    }
    let conn = open_runtime_connection(app)?;
    let read = |key: &str| {
        super::get_setting_value(&conn, &format!("{REMOTE_EE_SYNC_SETTING_PREFIX}{key}"))
    };
    let auth_type = match read("auth_type")?.as_deref() {
        Some("windows_integrated") => MssqlAuthType::WindowsIntegrated,
        Some("sql_server") => MssqlAuthType::SqlServer,
        _ => return Err("Remote EE Sync settings are incomplete.".into()),
    };
    let saved = MssqlConnectionConfig {
        server: read("server")?.unwrap_or_default(),
        database: read("database")?.unwrap_or_default(),
        port: read("port")?
            .and_then(|value| value.parse().ok())
            .unwrap_or(1433),
        username: read("username")?.unwrap_or_default(),
        auth_type,
        password: if auth_type == MssqlAuthType::SqlServer {
            read_mssql_password()?
        } else {
            None
        },
        environment_override: false,
    };
    resolve_connection_config_from(&HashMap::new(), Some(saved))
}

#[cfg(windows)]
fn read_mssql_password() -> Result<Option<String>, String> {
    use std::ptr;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND};
    use windows_sys::Win32::Security::Credentials::{
        CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
    };
    let target: Vec<u16> = REMOTE_EE_SYNC_SECRET_REF
        .encode_utf16()
        .chain([0])
        .collect();
    let mut credential: *mut CREDENTIALW = ptr::null_mut();
    if unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) } == 0 {
        return if unsafe { GetLastError() } == ERROR_NOT_FOUND {
            Ok(None)
        } else {
            Err("Windows secure credential could not be read.".into())
        };
    }
    let result = unsafe {
        let item = &*credential;
        if item.CredentialBlob.is_null() || item.CredentialBlobSize == 0 {
            Err("Stored SQL credential is invalid.".to_string())
        } else {
            String::from_utf8(
                std::slice::from_raw_parts(item.CredentialBlob, item.CredentialBlobSize as usize)
                    .to_vec(),
            )
            .map_err(|_| "Stored SQL credential is invalid.".to_string())
        }
    };
    unsafe {
        CredFree(credential.cast());
    }
    result.map(Some)
}
#[cfg(not(windows))]
fn read_mssql_password() -> Result<Option<String>, String> {
    Err("Secure SQL password storage is available only on Windows.".into())
}

#[cfg(windows)]
fn write_mssql_password(password: &str) -> Result<(), String> {
    use std::ptr;
    use windows_sys::Win32::Security::Credentials::{
        CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };
    if password.len() > 5120 {
        return Err("SQL password exceeds Windows Credential Manager limits.".into());
    }
    let mut target: Vec<u16> = REMOTE_EE_SYNC_SECRET_REF
        .encode_utf16()
        .chain([0])
        .collect();
    let mut blob = password.as_bytes().to_vec();
    let mut item = CREDENTIALW::default();
    item.Type = CRED_TYPE_GENERIC;
    item.TargetName = target.as_mut_ptr();
    item.CredentialBlobSize = blob.len() as u32;
    item.CredentialBlob = blob.as_mut_ptr();
    item.Persist = CRED_PERSIST_LOCAL_MACHINE;
    item.UserName = ptr::null_mut();
    let success = unsafe { CredWriteW(&item, 0) };
    blob.fill(0);
    if success == 0 {
        Err("Windows secure credential storage failed.".into())
    } else {
        Ok(())
    }
}
#[cfg(not(windows))]
fn write_mssql_password(_password: &str) -> Result<(), String> {
    Err("Secure SQL password storage is available only on Windows.".into())
}

#[cfg(windows)]
fn delete_mssql_password() -> Result<(), String> {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND};
    use windows_sys::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};
    let target: Vec<u16> = REMOTE_EE_SYNC_SECRET_REF
        .encode_utf16()
        .chain([0])
        .collect();
    if unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) } != 0
        || unsafe { GetLastError() } == ERROR_NOT_FOUND
    {
        Ok(())
    } else {
        Err("Windows secure credential could not be removed.".into())
    }
}
#[cfg(not(windows))]
fn delete_mssql_password() -> Result<(), String> {
    Err("Secure SQL password storage is available only on Windows.".into())
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

type MssqlClient = Client<Compat<TcpStream>>;

fn format_mssql_connect_error(error: impl std::fmt::Display) -> String {
    let detail = error.to_string();
    let normalized = detail.to_ascii_lowercase();

    if normalized.contains("cn name does not match")
        || normalized.contains("hostname mismatch")
        || normalized.contains("wrong principal")
        || normalized.contains("0x800b010f")
    {
        return "TLS certificate hostname mismatch. Configure the SQL Server using the DNS hostname in the certificate SAN; use an IP only when the certificate includes that IP.".into();
    }

    if normalized.contains("untrusted root")
        || normalized.contains("trusted root authority")
        || normalized.contains("unknown ca")
        || normalized.contains("0x800b0109")
    {
        return "TLS certificate chain is not trusted. Install the issuing CA and required intermediate certificates in the Windows certificate trust store.".into();
    }

    if normalized.contains("not within its validity period")
        || normalized.contains("certificate has expired")
        || normalized.contains("cert_e_expired")
        || normalized.contains("0x800b0101")
    {
        return "TLS certificate is expired or not yet valid. Renew the SQL Server certificate and verify the client system clock.".into();
    }

    if normalized.contains("10054")
        || normalized.contains("forcibly closed")
        || normalized.contains("connection reset")
    {
        return "MSSQL connection failed during TLS/authentication negotiation (Windows error 10054). Verify server, port, TLS settings, firewall, and credentials.".into();
    }

    if normalized.contains("tls")
        || normalized.contains("handshake")
        || normalized.contains("certificate")
    {
        return "MSSQL connection failed during TLS negotiation. Verify SQL Server encryption and certificate configuration.".into();
    }

    "MSSQL authentication failed. Verify the selected authentication type and credentials.".into()
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
        return format!("MSSQL network connection failed for {addr}: timeout (Windows error 10060). Check company network/VPN, server availability, TCP, and firewall.");
    }

    format!("MSSQL network connection failed for {addr}. Check server, port, network/VPN, and firewall settings.")
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

async fn connect(connection: &MssqlConnectionConfig) -> Result<MssqlClient, String> {
    validate_mssql_account(connection.auth_type, &connection.username)?;
    let mut config = Config::new();
    config.host(&connection.server);
    config.port(connection.port);
    config.database(&connection.database);
    config.encryption(tiberius::EncryptionLevel::Required);
    config.readonly(true);
    match connection.auth_type {
        MssqlAuthType::SqlServer => config.authentication(tiberius::AuthMethod::sql_server(
            &connection.username,
            connection.password.as_deref().unwrap_or_default(),
        )),
        MssqlAuthType::WindowsIntegrated => {
            #[cfg(windows)]
            config.authentication(tiberius::AuthMethod::Integrated);
            #[cfg(not(windows))]
            return Err("Windows Integrated authentication is available only on Windows.".into());
        }
    }
    let addr = format!("{}:{}", connection.server, connection.port);
    let mut last_error = None;

    for attempt in 1..=MSSQL_CONNECT_ATTEMPTS {
        let tcp_result = timeout(
            MSSQL_CONNECT_TIMEOUT,
            TcpStream::connect((&connection.server[..], connection.port)),
        )
        .await;
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

pub async fn test_remote_ee_sync_connection(app: &AppHandle) -> MssqlConnectionTestResult {
    let result = async {
        let config = load_connection_config(app)?;
        let rows = fetch_employee_list_sync_source(&config).await?;
        Ok::<usize, String>(rows.len())
    }
    .await;
    match result {
        Ok(count) => MssqlConnectionTestResult::pass(count),
        Err(error) => MssqlConnectionTestResult::fail(sanitize_connection_reason(&error)),
    }
}

fn sanitize_connection_reason(error: &str) -> String {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("only the approved staff kit sql login is allowed") {
        "Remote EE Sync is restricted to the approved Staff Kit SQL login.".into()
    } else if normalized.contains("read-only")
        || normalized.contains("write-level")
        || normalized.contains("permission")
    {
        "Read-only permissions could not be verified or write access was detected. Configure a read-only SQL login.".into()
    } else if normalized.contains("windows integrated authentication is available only on windows")
    {
        "Windows Integrated authentication is available only on Windows.".into()
    } else if normalized.contains("secure sql password storage")
        || normalized.contains("credential manager")
    {
        "Windows secure credential storage is unavailable; no SQL connection was attempted.".into()
    } else if normalized
        .starts_with("server must be a valid ipv4 address, ipv6 address, or dns hostname")
    {
        "Server must be a valid IPv4 address, IPv6 address, or DNS hostname; URL schemes are not supported.".into()
    } else if normalized.contains("tls certificate hostname mismatch") {
        "TLS certificate hostname mismatch. Configure the SQL Server using the DNS hostname in the certificate SAN; use an IP only when the certificate includes that IP.".into()
    } else if normalized.contains("tls certificate chain is not trusted") {
        "TLS certificate chain is not trusted. Install the issuing CA and required intermediate certificates in the Windows certificate trust store.".into()
    } else if normalized.contains("tls certificate is expired") {
        "TLS certificate is expired or not yet valid. Renew the SQL Server certificate and verify the client system clock.".into()
    } else if normalized.contains("tls")
        || normalized.contains("certificate")
        || normalized.contains("handshake")
    {
        "Encrypted SQL connection or server certificate validation failed. Verify the SQL Server TLS certificate.".into()
    } else if normalized.contains("incomplete")
        || normalized.contains("not configured")
        || normalized.contains("settings")
    {
        error.to_string()
    } else if normalized.contains("10060")
        || normalized.contains("timed out")
        || normalized.contains("network")
    {
        "Could not reach the SQL Server. Check server, port, network/VPN, and firewall.".into()
    } else if normalized.contains("authentication") || normalized.contains("login failed") {
        "SQL Server authentication failed. Check the selected authentication type and credentials."
            .into()
    } else {
        "Connection or approved Query B failed; no database changes were made.".into()
    }
}

async fn fetch_employee_list_sync_source(
    connection: &MssqlConnectionConfig,
) -> Result<Vec<MssqlStaffRecord>, String> {
    let mut client = connect(connection).await?;
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
) -> Result<EmployeeListSyncPreview, String> {
    let connection = load_connection_config(app)?;
    let source = fetch_employee_list_sync_source(&connection).await?;
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
    let connection = load_connection_config(app)?;
    let source = fetch_employee_list_sync_source(&connection).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn sql_server_endpoint_accepts_ip_literals_and_dns_names_verbatim() {
        for server in ["10.184.0.19", "2001:db8::1", "ASWPLUSV2.aswigsvd.int"] {
            assert_eq!(validate_mssql_server(server), Ok(()), "{server}");
            let resolved = resolve_connection_config_from(
                &HashMap::new(),
                Some(MssqlConnectionConfig {
                    server: server.into(),
                    database: "AssetManagement".into(),
                    port: 1433,
                    auth_type: MssqlAuthType::SqlServer,
                    username: APPROVED_MSSQL_USERNAME.into(),
                    password: Some("synthetic-test-secret".into()),
                    environment_override: false,
                }),
            )
            .expect("valid host should reach connection configuration");
            assert_eq!(resolved.server, server);
        }
    }

    #[test]
    fn sql_server_endpoint_rejects_urls_and_malformed_addresses() {
        for server in [
            "https://10.184.0.19",
            "tcp://ASWPLUSV2.aswigsvd.int",
            "10.184.0.999",
            "10.184..19",
            "bad host.int",
            "bad-.host.int",
            "2001:db8:::1",
            "[2001:db8::1]",
        ] {
            assert!(validate_mssql_server(server).is_err(), "{server}");
            assert!(
                validate_remote_settings(&RemoteEeSyncSettings {
                    server: server.into(),
                    database: "AssetManagement".into(),
                    port: 1433,
                    auth_type: MssqlAuthType::SqlServer,
                    username: APPROVED_MSSQL_USERNAME.into(),
                    has_saved_password: false,
                    environment_override: false,
                })
                .is_err(),
                "save should reject {server}"
            );
        }
    }

    #[test]
    fn invalid_server_test_connection_reason_is_actionable_and_safe() {
        let reason = sanitize_connection_reason(
            "Server must be a valid IPv4 address, IPv6 address, or DNS hostname; URL schemes are not supported.",
        );

        assert!(reason.contains("valid IPv4 address"));
        assert!(reason.contains("URL schemes are not supported"));
    }

    #[test]
    fn saved_server_value_is_preserved_for_connection_and_profile_storage() {
        let server = "ASWPLUSV2.aswigsvd.int";
        let config = MssqlConnectionConfig {
            server: server.into(),
            database: "AssetManagement".into(),
            port: 1433,
            auth_type: MssqlAuthType::SqlServer,
            username: APPROVED_MSSQL_USERNAME.into(),
            password: Some("synthetic-test-secret".into()),
            environment_override: false,
        };
        let resolved = resolve_connection_config_from(&HashMap::new(), Some(config))
            .expect("resolve saved endpoint");
        assert_eq!(resolved.server, server);

        let conn = Connection::open_in_memory().expect("in-memory settings DB");
        conn.execute_batch("CREATE TABLE app_settings(setting_key TEXT PRIMARY KEY, setting_value TEXT, updated_at TEXT);")
            .expect("create settings table");
        write_remote_profile_settings(
            &conn,
            &RemoteEeSyncSettings {
                server: server.into(),
                database: "AssetManagement".into(),
                port: 1433,
                auth_type: MssqlAuthType::SqlServer,
                username: APPROVED_MSSQL_USERNAME.into(),
                has_saved_password: false,
                environment_override: false,
            },
        )
        .expect("persist endpoint");
        let stored: String = conn
            .query_row(
                "SELECT setting_value FROM app_settings WHERE setting_key = ?1",
                [format!("{REMOTE_EE_SYNC_SETTING_PREFIX}server")],
                |row| row.get(0),
            )
            .expect("read persisted endpoint");
        assert_eq!(stored, server);

        let mut tiberius_config = Config::new();
        tiberius_config.host(&resolved.server);
        tiberius_config.port(resolved.port);
        assert_eq!(tiberius_config.get_addr(), "ASWPLUSV2.aswigsvd.int:1433");
    }

    #[test]
    fn remote_ee_sync_settings_dto_never_contains_password_material() {
        let settings = RemoteEeSyncSettings {
            server: "sql.example".into(),
            database: "AssetManagement".into(),
            port: 1433,
            auth_type: MssqlAuthType::SqlServer,
            username: APPROVED_MSSQL_USERNAME.into(),
            has_saved_password: true,
            environment_override: false,
        };
        let serialized = serde_json::to_string(&settings).expect("serialize safe DTO");
        assert!(
            !serialized.contains("\"password\":") && !serialized.contains("synthetic-test-secret")
        );
        assert!(!serialized.contains("synthetic-test-secret"));
        assert!(serialized.contains("hasSavedPassword"));
    }

    #[test]
    fn partial_environment_override_is_rejected_without_falling_back_to_saved_profile() {
        let env = HashMap::from([(
            "STAFFKIT_MSSQL_SERVER".to_string(),
            "sql.example".to_string(),
        )]);
        let saved = MssqlConnectionConfig {
            server: "saved.example".into(),
            database: "AssetManagement".into(),
            port: 1433,
            auth_type: MssqlAuthType::SqlServer,
            username: APPROVED_MSSQL_USERNAME.into(),
            password: Some("synthetic-saved-secret".into()),
            environment_override: false,
        };
        let result = resolve_connection_config_from(&env, Some(saved));
        assert!(result.err().unwrap().contains("incomplete"));
    }

    #[test]
    fn only_exact_approved_login_is_accepted_for_saved_and_environment_profiles() {
        let approved = MssqlConnectionConfig {
            server: "sql.example".into(),
            database: "AssetManagement".into(),
            port: 1433,
            auth_type: MssqlAuthType::SqlServer,
            username: "user_reader".into(),
            password: Some("synthetic-test-secret".into()),
            environment_override: false,
        };
        assert!(resolve_connection_config_from(&HashMap::new(), Some(approved.clone())).is_ok());

        for username in [
            "aswhite-plus-webserver",
            "other_user",
            "user_reader-",
            "User_Reader",
            "user_reader ",
        ] {
            let mut saved = approved.clone();
            saved.username = username.into();
            assert!(
                resolve_connection_config_from(&HashMap::new(), Some(saved)).is_err(),
                "saved profile username {username:?} must be rejected"
            );

            let env = HashMap::from([
                (MSSQL_SERVER_ENV.into(), "sql.example".into()),
                (MSSQL_DATABASE_ENV.into(), "AssetManagement".into()),
                (MSSQL_AUTH_TYPE_ENV.into(), "sql_server".into()),
                (MSSQL_USERNAME_ENV.into(), username.into()),
                (MSSQL_PASSWORD_ENV.into(), "synthetic-test-secret".into()),
            ]);
            assert!(
                resolve_connection_config_from(&env, None).is_err(),
                "environment username {username:?} must be rejected"
            );
        }
    }

    #[test]
    fn only_approved_sql_authentication_can_be_saved_or_resolved() {
        let mut settings = RemoteEeSyncSettings {
            server: "sql.example".into(),
            database: "AssetManagement".into(),
            port: 1433,
            auth_type: MssqlAuthType::SqlServer,
            username: "user_reader".into(),
            has_saved_password: false,
            environment_override: false,
        };
        assert!(validate_remote_settings(&settings).is_ok());
        settings.username = "aswhite-plus-webserver".into();
        assert!(validate_remote_settings(&settings).is_err());

        let integrated_env = HashMap::from([
            (MSSQL_SERVER_ENV.into(), "sql.example".into()),
            (MSSQL_DATABASE_ENV.into(), "AssetManagement".into()),
            (MSSQL_AUTH_TYPE_ENV.into(), "windows_integrated".into()),
            (MSSQL_USERNAME_ENV.into(), APPROVED_MSSQL_USERNAME.into()),
        ]);
        assert!(resolve_connection_config_from(&integrated_env, None).is_err());

        let mut integrated = MssqlConnectionConfig {
            server: "sql.example".into(),
            database: "AssetManagement".into(),
            port: 1433,
            auth_type: MssqlAuthType::WindowsIntegrated,
            username: "user_reader".into(),
            password: None,
            environment_override: false,
        };
        assert!(resolve_connection_config_from(&HashMap::new(), Some(integrated.clone())).is_err());
        integrated.auth_type = MssqlAuthType::SqlServer;
        integrated.username = "user_reader".into();
        integrated.password = Some("synthetic-test-secret".into());
        assert!(resolve_connection_config_from(&HashMap::new(), Some(integrated)).is_ok());
    }

    #[tokio::test]
    async fn connect_rejects_unapproved_login_before_network_or_tls() {
        for username in [
            "aswhite-plus-webserver",
            "other_user",
            "user_reader-",
            "User_Reader",
            "user_reader ",
        ] {
            let connection = MssqlConnectionConfig {
                server: "127.0.0.1".into(),
                database: "AssetManagement".into(),
                port: 1,
                auth_type: MssqlAuthType::SqlServer,
                username: username.into(),
                password: Some("synthetic-test-secret".into()),
                environment_override: false,
            };
            let error = connect(&connection)
                .await
                .err()
                .expect("disallowed account must fail before attempting the local socket");
            assert!(error.contains(MSSQL_ACCOUNT_ALLOWLIST_ERROR));
        }
    }

    #[test]
    fn saved_profile_persists_only_non_secret_fields_and_secret_reference() {
        let conn = Connection::open_in_memory().expect("in-memory settings DB");
        conn.execute_batch("CREATE TABLE app_settings(setting_key TEXT PRIMARY KEY, setting_value TEXT, updated_at TEXT);")
            .expect("create settings table");
        let settings = RemoteEeSyncSettings {
            server: "sql.example".into(),
            database: "AssetManagement".into(),
            port: 1433,
            auth_type: MssqlAuthType::SqlServer,
            username: APPROVED_MSSQL_USERNAME.into(),
            has_saved_password: false,
            environment_override: false,
        };
        write_remote_profile_settings(&conn, &settings).expect("save non-secret profile");
        let keys: Vec<String> = conn
            .prepare("SELECT setting_key FROM app_settings ORDER BY setting_key")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let values: Vec<String> = conn
            .prepare("SELECT setting_value FROM app_settings ORDER BY setting_key")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(values.contains(&REMOTE_EE_SYNC_SECRET_REF.to_string()));
        assert!(!keys.iter().any(|key| key.ends_with("password")));
        assert_eq!(keys.len(), 6);
    }

    #[test]
    fn environment_override_is_used_only_when_required_fields_are_complete() {
        let env = HashMap::from([
            (
                "STAFFKIT_MSSQL_SERVER".to_string(),
                "sql.example".to_string(),
            ),
            (
                "STAFFKIT_MSSQL_DATABASE".to_string(),
                "AssetManagement".to_string(),
            ),
            ("STAFFKIT_MSSQL_PORT".to_string(), "1433".to_string()),
            (
                "STAFFKIT_MSSQL_AUTH_TYPE".to_string(),
                "sql_server".to_string(),
            ),
            (
                "STAFFKIT_MSSQL_USERNAME".to_string(),
                APPROVED_MSSQL_USERNAME.to_string(),
            ),
            (
                "STAFFKIT_MSSQL_PASSWORD".to_string(),
                "synthetic-test-secret".to_string(),
            ),
        ]);
        let config = resolve_connection_config_from(&env, None).expect("complete env profile");
        assert_eq!(config.server, "sql.example");
        assert_eq!(config.database, "AssetManagement");
        assert_eq!(config.username, APPROVED_MSSQL_USERNAME);
        assert_eq!(config.password.as_deref(), Some("synthetic-test-secret"));
        assert!(config.environment_override);
    }

    #[test]
    fn test_connection_result_contains_safe_status_reason_and_optional_row_count() {
        let result =
            MssqlConnectionTestResult::fail("Read-only permissions could not be verified.");
        let serialized = serde_json::to_string(&result).expect("serialize result");
        assert!(serialized.contains("FAIL"));
        assert!(serialized.contains("reason"));
        assert!(!serialized.contains("password"));
        assert_eq!(result.row_count, None);
    }

    #[test]
    fn tls_test_connection_reason_distinguishes_hostname_mismatch_without_raw_details() {
        let formatted = format_mssql_connect_error(
            "The certificate's CN name does not match the passed value. password=secret",
        );
        let reason = sanitize_connection_reason(&formatted);

        assert!(reason.contains("DNS hostname"));
        assert!(reason.contains("certificate SAN"));
        assert!(!reason.contains("secret"));
        assert!(!reason.contains("CN name"));
    }

    #[test]
    fn tls_test_connection_reason_distinguishes_untrusted_chain() {
        let formatted = format_mssql_connect_error(
            "A certificate chain could not be built to a trusted root authority.",
        );
        let reason = sanitize_connection_reason(&formatted);

        assert!(reason.contains("trusted"));
        assert!(reason.contains("CA"));
    }

    #[test]
    fn tls_test_connection_reason_distinguishes_expired_certificate() {
        let formatted = format_mssql_connect_error(
            "A required certificate is not within its validity period when verifying against the current system clock.",
        );
        let reason = sanitize_connection_reason(&formatted);

        let normalized = reason.to_ascii_lowercase();
        assert!(normalized.contains("expired"));
        assert!(normalized.contains("renew"));
    }

    #[test]
    fn tls_test_connection_reason_keeps_unknown_tls_failure_actionable_and_safe() {
        let formatted = format_mssql_connect_error("TLS handshake failed with secret=do-not-show");
        let reason = sanitize_connection_reason(&formatted);

        assert!(reason.contains("TLS"));
        assert!(reason.contains("certificate"));
        assert!(!reason.contains("do-not-show"));
    }

    #[test]
    fn ee_list_sync_query_is_the_fixed_approved_read_only_query() {
        let query = EE_LIST_SYNC_QUERY.to_ascii_lowercase();

        assert!(query.contains("inner join [dbo].[aspnetusers]"));
        assert!(query.contains("from [dbo].[staffs]"));
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
    fn read_only_login_check_fails_closed_for_write_roles_and_table_permissions() {
        let check = MSSQL_READ_ONLY_CHECK.to_ascii_lowercase();
        assert!(check.contains("is_srvrolemember('sysadmin')"));
        assert!(check.contains("is_member('db_datawriter')"));
        assert!(check.contains("has_perms_by_name('dbo.staffs', 'object', 'update')"));
        assert!(check.contains("has_perms_by_name('dbo.aspnetusers', 'object', 'insert')"));
        assert!(check.contains("has_perms_by_name(null, 'server', 'control server')"));
        assert!(check.contains("has_perms_by_name(null, 'server', 'alter any server role')"));
        assert!(check.contains("has_perms_by_name(null, 'server', 'alter any login')"));
        assert!(check.contains("has_perms_by_name(db_name(), 'database', 'alter any role')"));
        assert!(check.contains("has_perms_by_name('dbo', 'schema', 'alter')"));
        assert!(check.contains("has_perms_by_name('dbo.staffs', 'object', 'take ownership')"));
        assert!(check.contains("has_perms_by_name(db_name(), 'database', 'execute')"));
        assert!(check.contains("has_perms_by_name('dbo', 'schema', 'execute')"));
        assert!(check.contains("from sys.procedures as p"));
        assert!(check.contains("from sys.fn_my_permissions(null, 'server')"));
        assert!(check.contains("permission_name = 'connect sql'"));
        assert!(check.contains("permission_name = 'connect'"));
        assert!(check.contains("cross apply sys.fn_my_permissions(s.name, 'schema')"));
        assert!(check.contains("from sys.database_principals as p"));
        assert!(check.contains("when 'r' then 'role'"));
        assert!(check.contains("from sys.server_principals as p"));
        assert!(check.contains("sys.fn_my_permissions(p.name, 'login')"));
        assert!(check.contains("has_perms_by_name(p.name, 'server role', 'any')"));
        assert!(check.contains("where p.type = 'r'"));
        assert!(
            check.contains("where o.type in ('u', 'v', 'p', 'pc', 'fn', 'if', 'tf', 'fs', 'ft')")
        );
        assert!(check.contains("permission_name not in ('select', 'view definition')"));
        assert!(!check.contains("references"));
        assert!(check.contains("coalesce(") && check.contains("then 1 else 0 end as bit"));
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
}
