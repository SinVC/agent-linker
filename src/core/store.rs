//! SQLite connections, explicit migrations, backups, and diagnostics.
use crate::core::{
    error::{Error, Result},
    harness::builtin_harnesses,
    linkable::normalize_target_dir,
    paths::{resolve_database_path, DbPathReason, DbPathResolution},
    util::{timestamp, timestamp_nanos},
};
use rusqlite::{params, Connection, OpenFlags, Transaction};
use std::{
    fs,
    path::{Path, PathBuf},
};
const LATEST_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub path: PathBuf,
    pub reason: DbPathReason,
    pub previous_version: u32,
    pub current_version: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbCheckStatus {
    Missing,
    SchemaMismatch,
    ReadOnly,
    LockedBusy,
    HealthyWritable,
    Invalid,
}
impl DbCheckStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::SchemaMismatch => "schema_mismatch",
            Self::ReadOnly => "read_only",
            Self::LockedBusy => "locked_busy",
            Self::HealthyWritable => "healthy_writable",
            Self::Invalid => "invalid",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbCheckReport {
    pub path: PathBuf,
    pub reason: DbPathReason,
    pub exists: bool,
    pub status: DbCheckStatus,
    pub schema_version: Option<u32>,
    pub latest_schema_version: u32,
    pub details: Vec<String>,
}
impl DbCheckReport {
    pub fn is_ok(&self) -> bool {
        self.status == DbCheckStatus::HealthyWritable
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbBackupReport {
    pub source_path: PathBuf,
    pub backup_path: PathBuf,
    pub bytes: u64,
}

pub struct Store {
    connection: Connection,
}
impl Store {
    pub fn open_ro(resolution: &DbPathResolution) -> Result<Self> {
        let connection =
            open_existing_connection(&resolution.path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        connection.pragma_update(None, "query_only", true)?;
        require_latest(&connection, &resolution.path)?;
        Ok(Self { connection })
    }
    pub fn open_rw(resolution: &DbPathResolution) -> Result<Self> {
        let connection =
            open_existing_connection(&resolution.path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        connection.pragma_update(None, "foreign_keys", true)?;
        require_latest(&connection, &resolution.path)?;
        Ok(Self { connection })
    }
    pub(super) fn connection(&self) -> &Connection {
        &self.connection
    }
    pub(super) fn write<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let transaction = self.connection.unchecked_transaction()?;
        let result = operation(&transaction)?;
        transaction.commit()?;
        Ok(result)
    }
    pub fn migrate(resolution: &DbPathResolution) -> Result<MigrationReport> {
        if let Some(parent) = resolution
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|e| path_error(parent, e))?;
        }
        let mut connection =
            Connection::open(&resolution.path).map_err(|e| path_error(&resolution.path, e))?;
        let previous_version = schema_version(&connection)?;
        // Foreign keys stay off during the transactional table rebuild; IDs and child rows are copied intact.
        connection.pragma_update(None, "foreign_keys", false)?;
        let transaction = connection.transaction()?;
        migrate_connection(&transaction, previous_version)
            .map_err(|e| path_error(&resolution.path, e))?;
        seed_builtin_harnesses(&transaction)?;
        transaction.commit()?;
        Ok(MigrationReport {
            path: resolution.path.clone(),
            reason: resolution.reason,
            previous_version,
            current_version: LATEST_SCHEMA_VERSION,
        })
    }
}
fn require_latest(connection: &Connection, path: &Path) -> Result<()> {
    match schema_version(connection)? {
        v if v < LATEST_SCHEMA_VERSION => {
            Err(Error::database("database schema is old; run aglink db migrate").at_path(path))
        }
        v if v > LATEST_SCHEMA_VERSION => {
            Err(Error::database("database schema is newer; upgrade aglink").at_path(path))
        }
        _ => Ok(()),
    }
}
pub fn latest_schema_version() -> u32 {
    LATEST_SCHEMA_VERSION
}
pub fn migrate_default_database() -> Result<MigrationReport> {
    Store::migrate(&resolve_database_path()?)
}
pub fn check_default_database() -> Result<DbCheckReport> {
    check_database(&resolve_database_path()?)
}
pub fn check_database(resolution: &DbPathResolution) -> Result<DbCheckReport> {
    let mut report = DbCheckReport {
        path: resolution.path.clone(),
        reason: resolution.reason,
        exists: false,
        status: DbCheckStatus::Missing,
        schema_version: None,
        latest_schema_version: LATEST_SCHEMA_VERSION,
        details: Vec::new(),
    };
    if path_metadata(&resolution.path)?.is_none() {
        return Ok(report);
    }
    report.exists = true;
    let inspect = || -> rusqlite::Result<(u32, Vec<String>)> {
        let connection =
            Connection::open_with_flags(&resolution.path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let version = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        let mut issues = Vec::new();
        if version == LATEST_SCHEMA_VERSION {
            for table in [
                "harnesses",
                "harness_mappings",
                "linkable_items",
                "groups",
                "group_items",
            ] {
                let exists: bool = connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                    [table],
                    |r| r.get(0),
                )?;
                if !exists {
                    issues.push(format!("missing table: {table}"));
                }
            }
            let mut statement = connection.prepare("PRAGMA quick_check")?;
            for result in statement.query_map([], |r| r.get::<_, String>(0))? {
                let result = result?;
                if result != "ok" {
                    issues.push(result);
                }
            }
        }
        Ok((version, issues))
    };
    match inspect() {
        Ok((version, issues)) => {
            report.schema_version = Some(version);
            report.details = issues;
            if version != LATEST_SCHEMA_VERSION {
                report.status = DbCheckStatus::SchemaMismatch;
                return Ok(report);
            }
            if !report.details.is_empty() {
                report.status = DbCheckStatus::Invalid;
                return Ok(report);
            }
        }
        Err(error) => {
            report.status = diagnostic_status(&error);
            report.details.push(error.to_string());
            return Ok(report);
        }
    }
    let probe = || -> rusqlite::Result<()> {
        let connection =
            Connection::open_with_flags(&resolution.path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        connection.busy_timeout(std::time::Duration::ZERO)?;
        let version = report.schema_version.unwrap();
        connection.execute_batch(&format!(
            "BEGIN IMMEDIATE; PRAGMA user_version={version}; ROLLBACK;"
        ))?;
        Ok(())
    };
    match probe() {
        Ok(()) => report.status = DbCheckStatus::HealthyWritable,
        Err(error) => {
            report.status = diagnostic_status(&error);
            report.details.push(error.to_string());
        }
    }
    Ok(report)
}
fn diagnostic_status(error: &rusqlite::Error) -> DbCheckStatus {
    match error.sqlite_error_code() {
        Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked) => {
            DbCheckStatus::LockedBusy
        }
        Some(
            rusqlite::ErrorCode::ReadOnly
            | rusqlite::ErrorCode::PermissionDenied
            | rusqlite::ErrorCode::CannotOpen,
        ) => DbCheckStatus::ReadOnly,
        _ => DbCheckStatus::Invalid,
    }
}
pub fn backup_default_database(destination: Option<&Path>) -> Result<DbBackupReport> {
    let resolution = resolve_database_path()?;
    backup_database(&resolution, destination)
}

pub fn backup_database(
    resolution: &DbPathResolution,
    destination: Option<&Path>,
) -> Result<DbBackupReport> {
    // Snapshot the existing source without migrating, seeding, or creating it.
    let connection = open_existing_connection(&resolution.path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let backup_path = destination.map_or_else(
        || default_backup_path(resolution),
        |path| path.to_path_buf(),
    );
    let destination_text = backup_path.to_str().ok_or_else(|| {
        Error::invalid_arguments("backup path is not UTF-8").at_path(&backup_path)
    })?;

    if path_metadata(&backup_path)?.is_some() {
        return Err(Error::project(format!(
            "backup destination already exists: {}",
            backup_path.display()
        )));
    }

    if let Some(parent) = backup_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| path_error(parent, error))?;
    }

    // Reserve the destination so a concurrent backup cannot overwrite it.
    let reservation = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup_path)
        .map_err(|error| path_error(&backup_path, error))?;
    drop(reservation);
    if let Err(error) = connection.execute("VACUUM INTO ?1", params![destination_text]) {
        let _ = fs::remove_file(&backup_path);
        return Err(path_error(&backup_path, error));
    }
    let bytes = fs::metadata(&backup_path)
        .map_err(|error| path_error(&backup_path, error))?
        .len();
    Ok(DbBackupReport {
        source_path: resolution.path.clone(),
        backup_path,
        bytes,
    })
}

fn path_error(path: &Path, error: impl Into<Error>) -> Error {
    error.into().at_path(path)
}

fn path_metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(path_error(path, error)),
    }
}

fn open_existing_connection(path: &Path, flags: OpenFlags) -> Result<Connection> {
    if path_metadata(path)?.is_none() {
        return Err(
            Error::database("database missing; run aglink db migrate to initialize").at_path(path),
        );
    }
    Connection::open_with_flags(path, flags).map_err(|error| path_error(path, error))
}

fn migrate_connection(connection: &Connection, current_version: u32) -> Result<()> {
    if current_version > LATEST_SCHEMA_VERSION {
        return Err(Error::database(format!(
            "database schema version {current_version} is newer than supported version {LATEST_SCHEMA_VERSION}; upgrade aglink"
        )));
    }

    if current_version < 1 {
        connection.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS frameworks (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                built_in INTEGER NOT NULL CHECK (built_in IN (0, 1)),
                enabled_by_default INTEGER NOT NULL CHECK (enabled_by_default IN (0, 1)),
                enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS framework_mappings (
                id TEXT PRIMARY KEY NOT NULL,
                framework_id TEXT NOT NULL,
                source_path TEXT NOT NULL,
                link_path TEXT NOT NULL,
                link_kind TEXT NOT NULL CHECK (link_kind IN ('file', 'directory')),
                required INTEGER NOT NULL CHECK (required IN (0, 1)),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (framework_id) REFERENCES frameworks(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS linkable_items (
                id TEXT PRIMARY KEY NOT NULL,
                kind TEXT NOT NULL CHECK (kind IN ('skill', 'resource')),
                name TEXT NOT NULL,
                alias TEXT,
                source_path TEXT NOT NULL,
                source_kind TEXT NOT NULL CHECK (source_kind IN ('file', 'directory')),
                target_dir TEXT,
                description TEXT,
                version TEXT,
                commit_hash TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_linkable_items_kind_name
                ON linkable_items(kind, name);
            CREATE TABLE IF NOT EXISTS groups (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS group_items (
                group_id TEXT NOT NULL,
                item_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (group_id, item_id),
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE,
                FOREIGN KEY (item_id) REFERENCES linkable_items(id) ON DELETE CASCADE
            );
            PRAGMA user_version = 1;
            "#,
        )?;
    }

    if current_version < 2 {
        connection.execute_batch(
            r#"
            ALTER TABLE linkable_items
                ADD COLUMN source_type TEXT NOT NULL DEFAULT 'local-path';
            ALTER TABLE linkable_items
                ADD COLUMN source_ownership TEXT NOT NULL DEFAULT 'external';
            ALTER TABLE linkable_items
                ADD COLUMN repo_url TEXT;
            ALTER TABLE linkable_items
                ADD COLUMN repo_commit TEXT;
            PRAGMA user_version = 2;
            "#,
        )?;
    }

    if current_version < 3 {
        migrate_v3(connection)?;
    }
    Ok(())
}

fn migrate_v3(connection: &Connection) -> Result<()> {
    connection.execute_batch(r#"
 CREATE TABLE harnesses (
 id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL UNIQUE COLLATE NOCASE, display_name TEXT NOT NULL,
 built_in INTEGER NOT NULL CHECK(built_in IN(0,1)), enabled INTEGER NOT NULL CHECK(enabled IN(0,1)), created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
 CREATE TABLE harness_mappings (
 id TEXT PRIMARY KEY NOT NULL, harness_id TEXT NOT NULL REFERENCES harnesses(id) ON DELETE CASCADE,
 source_path TEXT NOT NULL, link_path TEXT NOT NULL COLLATE NOCASE, link_kind TEXT NOT NULL CHECK(link_kind IN('file','directory')),
 required INTEGER NOT NULL CHECK(required IN(0,1)), created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, UNIQUE(harness_id,link_path));
 CREATE TABLE linkable_items_v3 (
 id TEXT PRIMARY KEY NOT NULL, kind TEXT NOT NULL CHECK(kind IN('skill','resource')),name TEXT NOT NULL UNIQUE COLLATE NOCASE,
 alias TEXT,default_link_name TEXT NOT NULL,source_path TEXT NOT NULL,source_kind TEXT NOT NULL CHECK(source_kind IN('file','directory')),
 target_dir TEXT,description TEXT,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
 CREATE TABLE groups_v3 (
 id TEXT PRIMARY KEY NOT NULL,name TEXT NOT NULL UNIQUE COLLATE NOCASE,description TEXT,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
 CREATE TABLE group_items_v3 (
 group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,item_id TEXT NOT NULL REFERENCES linkable_items(id) ON DELETE CASCADE,
 created_at INTEGER NOT NULL,PRIMARY KEY(group_id,item_id));
 INSERT INTO harnesses SELECT id,name,display_name,built_in,enabled,CAST(REPLACE(created_at,'unix:','') AS INTEGER),CAST(REPLACE(updated_at,'unix:','') AS INTEGER) FROM frameworks;
 INSERT INTO harness_mappings SELECT id,framework_id,source_path,link_path,link_kind,required,CAST(REPLACE(created_at,'unix:','') AS INTEGER),CAST(REPLACE(updated_at,'unix:','') AS INTEGER) FROM framework_mappings;
 INSERT INTO linkable_items_v3 SELECT id,kind,name,alias,COALESCE(NULLIF(alias,''),name),source_path,source_kind,target_dir,description,CAST(REPLACE(created_at,'unix:','') AS INTEGER),CAST(REPLACE(updated_at,'unix:','') AS INTEGER) FROM linkable_items;
 INSERT INTO groups_v3 SELECT id,name,description,CAST(REPLACE(created_at,'unix:','') AS INTEGER),CAST(REPLACE(updated_at,'unix:','') AS INTEGER) FROM groups;
 INSERT INTO group_items_v3 SELECT group_id,item_id,CAST(REPLACE(created_at,'unix:','') AS INTEGER) FROM group_items;
 "#)?;
    let rows = {
        let mut statement=connection.prepare("SELECT id,name,alias,source_path,target_dir FROM linkable_items_v3 WHERE kind='resource'")?;
        let rows = statement.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, name, alias, source, target) in rows {
        let link_name = alias.filter(|s| !s.is_empty()).unwrap_or_else(|| {
            Path::new(&source)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&name)
                .to_string()
        });
        let target = target
            .map(|s| normalize_target_dir(Path::new(&s)))
            .transpose()?;
        connection.execute(
            "UPDATE linkable_items_v3 SET default_link_name=?2,target_dir=?3 WHERE id=?1",
            params![id, link_name, target],
        )?;
    }
    connection.execute_batch(r#"
 DROP TABLE group_items; DROP TABLE groups; DROP TABLE linkable_items;
 DROP TABLE framework_mappings; DROP TABLE frameworks; DROP TABLE config;
 ALTER TABLE linkable_items_v3 RENAME TO linkable_items;
 ALTER TABLE groups_v3 RENAME TO groups;
 ALTER TABLE group_items_v3 RENAME TO group_items;
 CREATE UNIQUE INDEX idx_skill_default_link_name ON linkable_items(default_link_name COLLATE NOCASE) WHERE kind='skill';
 CREATE UNIQUE INDEX idx_resource_target_link_name ON linkable_items(target_dir COLLATE NOCASE,default_link_name COLLATE NOCASE) WHERE kind='resource';
 PRAGMA user_version=3;
 "#)?;
    Ok(())
}
fn seed_builtin_harnesses(connection: &Connection) -> Result<()> {
    let now = timestamp();
    for harness in builtin_harnesses() {
        connection.execute(
            "INSERT OR IGNORE INTO harnesses
             (id,name,display_name,built_in,enabled,created_at,updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?6)",
            params![
                harness.id,
                harness.name,
                harness.display_name,
                harness.built_in,
                harness.enabled,
                now
            ],
        )?;
        for m in harness.mappings {
            connection.execute(
                "INSERT OR IGNORE INTO harness_mappings
                 (id,harness_id,source_path,link_path,link_kind,required,created_at,updated_at)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?7)",
                params![
                    m.id,
                    m.harness_id,
                    m.source_path.to_string_lossy(),
                    m.link_path.to_string_lossy(),
                    m.link_kind.to_string(),
                    m.required,
                    now
                ],
            )?;
        }
    }
    Ok(())
}
fn schema_version(connection: &Connection) -> Result<u32> {
    Ok(connection.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}
fn default_backup_path(resolution: &DbPathResolution) -> PathBuf {
    let source_path = &resolution.path;
    let parent = source_path.parent().unwrap_or_else(|| Path::new("."));
    let parent = if resolution.reason == DbPathReason::WindowsInstall {
        parent.parent().unwrap_or(parent).join("backups")
    } else {
        parent.to_path_buf()
    };
    parent.join(format!("agent-linker-backup-{}.db", timestamp_nanos()))
}

impl From<rusqlite::Error> for Error {
    fn from(source: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(code, _) = &source {
            match code.code {
                rusqlite::ErrorCode::ConstraintViolation
                    if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
                        || code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY =>
                {
                    return Self::Conflict {
                        message: format!(
                            "identifier, name, or default link destination already exists: {source}"
                        ),
                        path: None,
                        source: Some(source),
                    }
                }
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked => {
                    return Self::DatabaseBusy { path: None, source }
                }
                rusqlite::ErrorCode::PermissionDenied | rusqlite::ErrorCode::ReadOnly => {
                    return Self::PermissionDenied {
                        path: None,
                        source: Box::new(source),
                    }
                }
                _ => {}
            }
        }
        Self::Database { path: None, source }
    }
}
#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
