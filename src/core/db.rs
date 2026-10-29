//! Global SQLite database entry point.
//! Legacy `framework*` table and column names are retained; harness is the domain term.

use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};

use crate::core::{
    error::{Error, Result},
    paths::{resolve_database_path, DbPathReason, DbPathResolution},
    symlink::LinkKind,
    util::{bool_to_i64, timestamp, timestamp_nanos},
};

const LATEST_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub path: PathBuf,
    pub reason: DbPathReason,
    pub previous_version: u32,
    pub current_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbCheckReport {
    pub path: PathBuf,
    pub reason: DbPathReason,
    pub exists: bool,
    pub writable: bool,
    pub schema_version: Option<u32>,
    pub latest_schema_version: u32,
    pub harness_count: Option<u32>,
    pub mapping_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigUnsetReport {
    pub key: String,
    pub removed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbBackupReport {
    pub source_path: PathBuf,
    pub backup_path: PathBuf,
    pub bytes: u64,
}

impl DbCheckReport {
    pub fn is_ok(&self) -> bool {
        self.exists
            && self.writable
            && self.schema_version == Some(self.latest_schema_version)
            && self.harness_count.unwrap_or(0) > 0
            && self.mapping_count.unwrap_or(0) > 0
    }
}

pub fn migrate_default_database() -> Result<MigrationReport> {
    let resolution = resolve_database_path()?;
    migrate_database(&resolution)
}

pub fn migrate_database(resolution: &DbPathResolution) -> Result<MigrationReport> {
    let connection = open_writable_connection(&resolution.path)?;
    let previous_version = schema_version(&connection)?;
    migrate_connection(&connection, previous_version)?;
    seed_builtin_harnesses(&connection)?;
    let current_version = schema_version(&connection)?;

    Ok(MigrationReport {
        path: resolution.path.clone(),
        reason: resolution.reason,
        previous_version,
        current_version,
    })
}

pub fn check_default_database() -> Result<DbCheckReport> {
    let resolution = resolve_database_path()?;
    check_database(&resolution)
}

pub fn check_database(resolution: &DbPathResolution) -> Result<DbCheckReport> {
    if !resolution.path.exists() {
        return Ok(DbCheckReport {
            path: resolution.path.clone(),
            reason: resolution.reason,
            exists: false,
            writable: false,
            schema_version: None,
            latest_schema_version: LATEST_SCHEMA_VERSION,
            harness_count: None,
            mapping_count: None,
        });
    }

    let writable = open_writable_connection(&resolution.path).is_ok();
    let connection =
        Connection::open_with_flags(&resolution.path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let schema_version = schema_version(&connection)?;
    let (harness_count, mapping_count) = if schema_version >= LATEST_SCHEMA_VERSION {
        (
            Some(count_rows(&connection, CountTable::Harnesses)?),
            Some(count_rows(&connection, CountTable::HarnessMappings)?),
        )
    } else {
        (None, None)
    };

    Ok(DbCheckReport {
        path: resolution.path.clone(),
        reason: resolution.reason,
        exists: true,
        writable,
        schema_version: Some(schema_version),
        latest_schema_version: LATEST_SCHEMA_VERSION,
        harness_count,
        mapping_count,
    })
}

pub fn list_config(resolution: &DbPathResolution) -> Result<Vec<ConfigEntry>> {
    let connection = open_migrated_connection(resolution)?;
    let mut statement = connection.prepare(
        r#"
        SELECT key, value, updated_at
        FROM config
        ORDER BY key
        "#,
    )?;
    let rows = statement.query_map([], |row| {
        Ok(ConfigEntry {
            key: row.get(0)?,
            value: row.get(1)?,
            updated_at: row.get(2)?,
        })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

pub fn get_config(resolution: &DbPathResolution, key: &str) -> Result<Option<ConfigEntry>> {
    let connection = open_migrated_connection(resolution)?;
    connection
        .query_row(
            r#"
            SELECT key, value, updated_at
            FROM config
            WHERE key = ?1
            "#,
            params![key],
            |row| {
                Ok(ConfigEntry {
                    key: row.get(0)?,
                    value: row.get(1)?,
                    updated_at: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(Error::from)
}

pub fn set_config(resolution: &DbPathResolution, key: &str, value: &str) -> Result<ConfigEntry> {
    validate_config_key(key)?;
    let connection = open_migrated_connection(resolution)?;
    let now = timestamp();
    connection.execute(
        r#"
        INSERT INTO config (key, value, updated_at)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at
        "#,
        params![key, value, now],
    )?;
    get_config(resolution, key)?
        .ok_or_else(|| Error::database(format!("config `{key}` was not stored")))
}

pub fn unset_config(resolution: &DbPathResolution, key: &str) -> Result<ConfigUnsetReport> {
    validate_config_key(key)?;
    let connection = open_migrated_connection(resolution)?;
    let removed = connection.execute("DELETE FROM config WHERE key = ?1", params![key])? > 0;
    Ok(ConfigUnsetReport {
        key: key.to_string(),
        removed,
    })
}

pub fn backup_default_database(destination: Option<&Path>) -> Result<DbBackupReport> {
    let resolution = resolve_database_path()?;
    backup_database(&resolution, destination)
}

pub fn backup_database(
    resolution: &DbPathResolution,
    destination: Option<&Path>,
) -> Result<DbBackupReport> {
    migrate_database(resolution)?;
    let backup_path = destination.map_or_else(
        || default_backup_path(&resolution.path),
        |path| path.to_path_buf(),
    );

    if backup_path.exists() {
        return Err(Error::project(format!(
            "backup destination already exists: {}",
            backup_path.display()
        )));
    }

    if let Some(parent) = backup_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let bytes = fs::copy(&resolution.path, &backup_path)?;
    Ok(DbBackupReport {
        source_path: resolution.path.clone(),
        backup_path,
        bytes,
    })
}

pub fn open_migrated_default_connection() -> Result<Connection> {
    let resolution = resolve_database_path()?;
    open_migrated_connection(&resolution)
}

pub fn open_migrated_connection(resolution: &DbPathResolution) -> Result<Connection> {
    let connection = open_writable_connection(&resolution.path)?;
    let previous_version = schema_version(&connection)?;
    migrate_connection(&connection, previous_version)?;
    seed_builtin_harnesses(&connection)?;
    Ok(connection)
}

pub fn latest_schema_version() -> u32 {
    LATEST_SCHEMA_VERSION
}

fn open_writable_connection(path: &Path) -> Result<Connection> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    Connection::open(path).map_err(Error::from)
}

fn migrate_connection(connection: &Connection, current_version: u32) -> Result<()> {
    if current_version > LATEST_SCHEMA_VERSION {
        return Err(Error::database(format!(
            "database schema version {current_version} is newer than supported version {LATEST_SCHEMA_VERSION}"
        )));
    }

    if current_version < 1 {
        connection.execute_batch(
            r#"
            BEGIN;
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
            COMMIT;
            "#,
        )?;
    }

    if current_version < 2 {
        connection.execute_batch(
            r#"
            BEGIN;
            ALTER TABLE linkable_items
                ADD COLUMN source_type TEXT NOT NULL DEFAULT 'local-path';
            ALTER TABLE linkable_items
                ADD COLUMN source_ownership TEXT NOT NULL DEFAULT 'external';
            ALTER TABLE linkable_items
                ADD COLUMN repo_url TEXT;
            ALTER TABLE linkable_items
                ADD COLUMN repo_commit TEXT;
            PRAGMA user_version = 2;
            COMMIT;
            "#,
        )?;
    }

    Ok(())
}

fn seed_builtin_harnesses(connection: &Connection) -> Result<()> {
    let now = timestamp();

    connection.execute(
        r#"
        INSERT INTO frameworks
            (id, name, display_name, built_in, enabled_by_default, enabled, created_at, updated_at)
        VALUES
            ('claude', 'claude', 'Claude Code', 1, 1, 1, ?1, ?1)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            display_name = excluded.display_name,
            built_in = excluded.built_in,
            enabled_by_default = excluded.enabled_by_default,
            updated_at = excluded.updated_at
        "#,
        params![now],
    )?;

    seed_mapping(
        connection,
        "init:claude:agents-md",
        "claude",
        "AGENTS.md",
        "CLAUDE.md",
        LinkKind::File,
        true,
        &now,
    )?;
    seed_mapping(
        connection,
        "init:claude:skills-dir",
        "claude",
        ".agents/skills",
        ".claude/skills",
        LinkKind::Directory,
        true,
        &now,
    )?;

    Ok(())
}

fn seed_mapping(
    connection: &Connection,
    id: &str,
    harness_id: &str,
    source_path: &str,
    link_path: &str,
    link_kind: LinkKind,
    required: bool,
    now: &str,
) -> Result<()> {
    connection.execute(
        r#"
        INSERT INTO framework_mappings
            (id, framework_id, source_path, link_path, link_kind, required, created_at, updated_at)
        VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
        ON CONFLICT(id) DO UPDATE SET
            framework_id = excluded.framework_id,
            source_path = excluded.source_path,
            link_path = excluded.link_path,
            link_kind = excluded.link_kind,
            required = excluded.required,
            updated_at = excluded.updated_at
        "#,
        params![
            id,
            harness_id,
            source_path,
            link_path,
            link_kind.to_string(),
            bool_to_i64(required),
            now,
        ],
    )?;
    Ok(())
}

fn schema_version(connection: &Connection) -> Result<u32> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    u32::try_from(version).map_err(|_| Error::database("database schema version is invalid"))
}

#[derive(Debug, Clone, Copy)]
enum CountTable {
    Harnesses,
    HarnessMappings,
}

impl CountTable {
    const fn count_sql(self) -> &'static str {
        match self {
            CountTable::Harnesses => "SELECT COUNT(*) FROM frameworks",
            CountTable::HarnessMappings => "SELECT COUNT(*) FROM framework_mappings",
        }
    }

    const fn name(self) -> &'static str {
        match self {
            CountTable::Harnesses => "frameworks",
            CountTable::HarnessMappings => "framework_mappings",
        }
    }
}

fn count_rows(connection: &Connection, table: CountTable) -> Result<u32> {
    let count: i64 = connection.query_row(table.count_sql(), [], |row| row.get(0))?;
    u32::try_from(count)
        .map_err(|_| Error::database(format!("{} row count is invalid", table.name())))
}

fn validate_config_key(key: &str) -> Result<()> {
    if key.trim().is_empty() {
        return Err(Error::invalid_arguments("config key must not be empty"));
    }
    Ok(())
}

fn default_backup_path(source_path: &Path) -> PathBuf {
    let parent = source_path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("agent-linker-backup-{}.db", timestamp_nanos()))
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Error::database(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        backup_database, check_database, get_config, latest_schema_version, list_config,
        migrate_database, set_config, unset_config, DbPathReason, DbPathResolution,
    };
    use crate::core::{
        harness::{disable_harness, enable_harness, list_harnesses},
        test_support::TestDir,
    };

    #[test]
    fn migration_seeds_claude_harness_and_mappings() {
        let temp_dir = TestDir::new("db-migrate");
        let resolution = DbPathResolution {
            path: temp_dir.path().join("store").join("agent-linker.db"),
            reason: DbPathReason::ExplicitDataDirEnv,
        };

        let report = migrate_database(&resolution).unwrap();
        assert_eq!(report.previous_version, 0);
        assert_eq!(report.current_version, latest_schema_version());

        let check = check_database(&resolution).unwrap();
        assert!(check.is_ok());
        assert_eq!(check.harness_count, Some(1));
        assert_eq!(check.mapping_count, Some(2));

        let harnesses = list_harnesses(&resolution).unwrap();
        assert_eq!(harnesses.len(), 1);
        assert_eq!(harnesses[0].id, "claude");
        assert!(harnesses[0].enabled);
        assert_eq!(harnesses[0].mappings.len(), 2);
    }

    #[test]
    fn migration_seed_preserves_enabled_state() {
        let temp_dir = TestDir::new("db-seed-preserves-enabled");
        let resolution = DbPathResolution {
            path: temp_dir.path().join("agent-linker.db"),
            reason: DbPathReason::ExplicitDataDirEnv,
        };

        migrate_database(&resolution).unwrap();
        disable_harness(&resolution, "claude").unwrap();
        migrate_database(&resolution).unwrap();
        assert!(!list_harnesses(&resolution).unwrap()[0].enabled);

        enable_harness(&resolution, "claude").unwrap();
        assert!(list_harnesses(&resolution).unwrap()[0].enabled);
    }

    #[test]
    fn config_round_trips_and_database_backup_does_not_overwrite() {
        let temp_dir = TestDir::new("db-config-backup");
        let resolution = DbPathResolution {
            path: temp_dir.path().join("agent-linker.db"),
            reason: DbPathReason::ExplicitDataDirEnv,
        };
        migrate_database(&resolution).unwrap();

        let entry = set_config(&resolution, "output.mode", "quiet").unwrap();
        assert_eq!(entry.key, "output.mode");
        assert_eq!(entry.value, "quiet");
        assert_eq!(
            get_config(&resolution, "output.mode")
                .unwrap()
                .unwrap()
                .value,
            "quiet"
        );
        assert_eq!(list_config(&resolution).unwrap().len(), 1);

        let unset = unset_config(&resolution, "output.mode").unwrap();
        assert!(unset.removed);
        assert!(get_config(&resolution, "output.mode").unwrap().is_none());

        let backup_path = temp_dir.path().join("backup.db");
        let backup = backup_database(&resolution, Some(&backup_path)).unwrap();
        assert_eq!(backup.backup_path, backup_path);
        assert!(backup.bytes > 0);
        assert!(backup.backup_path.is_file());

        let overwrite = backup_database(&resolution, Some(&backup.backup_path)).unwrap_err();
        assert!(overwrite.to_string().contains("already exists"));
    }
}
