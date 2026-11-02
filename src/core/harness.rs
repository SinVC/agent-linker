//! Harness definitions, mappings, and Store business operations.
use crate::core::{
    error::{Error, Result},
    paths::{
        managed_relative_path_key, resolve_managed_project_path, validate_managed_relative_path,
    },
    store::Store,
    symlink::LinkKind,
    util::{timestamp, timestamp_nanos},
};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessMapping {
    pub id: String,
    pub harness_id: String,
    pub harness_name: String,
    pub item_id: String,
    pub item_name: String,
    pub source_path: PathBuf,
    pub link_path: PathBuf,
    pub link_kind: LinkKind,
    pub required: bool,
}

impl HarnessMapping {
    pub fn source_in(&self, project_root: &Path) -> Result<PathBuf> {
        resolve_managed_project_path(project_root, &self.source_path)
    }

    pub fn link_in(&self, project_root: &Path) -> Result<PathBuf> {
        resolve_managed_project_path(project_root, &self.link_path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredHarness {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub built_in: bool,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub mappings: Vec<StoredHarnessMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredHarnessMapping {
    pub id: String,
    pub harness_id: String,
    pub source_path: PathBuf,
    pub link_path: PathBuf,
    pub link_kind: LinkKind,
    pub required: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddHarnessMapping {
    pub harness: String,
    pub source_path: PathBuf,
    pub link_path: PathBuf,
    pub link_kind: LinkKind,
}

pub fn builtin_harnesses() -> Vec<StoredHarness> {
    let id = "claude".to_string();
    let mappings = [
        (
            "init:claude:agents-md",
            "AGENTS.md",
            "CLAUDE.md",
            LinkKind::File,
        ),
        (
            "init:claude:skills-dir",
            ".agents/skills",
            ".claude/skills",
            LinkKind::Directory,
        ),
    ]
    .into_iter()
    .map(|(mapping_id, source, link, kind)| StoredHarnessMapping {
        id: mapping_id.to_string(),
        harness_id: id.clone(),
        source_path: source.into(),
        link_path: link.into(),
        link_kind: kind,
        required: true,
        created_at: 0,
        updated_at: 0,
    })
    .collect();
    vec![StoredHarness {
        id,
        name: "claude".to_string(),
        display_name: "Claude Code".to_string(),
        built_in: true,
        enabled: true,
        created_at: 0,
        updated_at: 0,
        mappings,
    }]
}
impl StoredHarnessMapping {
    pub fn for_project(&self, harness_name: &str) -> HarnessMapping {
        let item_id = match self.id.as_str() {
            "init:claude:agents-md" => "project:agents-md".to_string(),
            "init:claude:skills-dir" => "project:skills-dir".to_string(),
            _ => format!("harness:{}", self.id),
        };
        HarnessMapping {
            id: self.id.clone(),
            harness_id: self.harness_id.clone(),
            harness_name: harness_name.to_string(),
            item_id,
            item_name: self.source_path.to_string_lossy().into_owned(),
            source_path: self.source_path.clone(),
            link_path: self.link_path.clone(),
            link_kind: self.link_kind,
            required: self.required,
        }
    }
}
pub fn default_init_mappings() -> Vec<HarnessMapping> {
    builtin_harnesses()
        .into_iter()
        .filter(|h| h.enabled)
        .flat_map(|h| h.mappings.into_iter().map(move |m| m.for_project(&h.name)))
        .collect()
}
impl Store {
    pub fn list_harnesses(&self) -> Result<Vec<StoredHarness>> {
        let connection = self.connection();
        let mut statement=connection.prepare("SELECT id,name,display_name,built_in,enabled,created_at,updated_at FROM harnesses ORDER BY name")?;
        let mut harnesses = statement
            .query_map([], harness_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for harness in &mut harnesses {
            harness.mappings = list_harness_mappings(connection, Some(&harness.id))?;
        }
        Ok(harnesses)
    }
    pub fn show_harness(&self, identifier: &str) -> Result<StoredHarness> {
        show_harness_with_connection(self.connection(), identifier)
    }
    pub fn enable_harness(&self, identifier: &str) -> Result<()> {
        self.set_harness_enabled(identifier, true)
    }
    pub fn disable_harness(&self, identifier: &str) -> Result<()> {
        self.set_harness_enabled(identifier, false)
    }
    fn set_harness_enabled(&self, identifier: &str, enabled: bool) -> Result<()> {
        self.write(|connection| {
            let changed = connection.execute(
                "UPDATE harnesses SET enabled=?2,updated_at=?3 WHERE id=?1 OR name=?1",
                params![identifier, enabled, timestamp()],
            )?;
            if changed == 0 {
                return Err(Error::NotFound(format!("unknown harness '{identifier}'")));
            }
            Ok(())
        })
    }
    pub fn list_all_mappings(&self) -> Result<Vec<StoredHarnessMapping>> {
        list_harness_mappings(self.connection(), None)
    }
    pub fn list_mappings_for_harness(&self, identifier: &str) -> Result<Vec<StoredHarnessMapping>> {
        Ok(self.show_harness(identifier)?.mappings)
    }
    pub fn add_mapping(&self, request: AddHarnessMapping) -> Result<StoredHarnessMapping> {
        self.write(|connection| {
            validate_mapping_path(&request.source_path, "source path")?;
            validate_mapping_path(&request.link_path, "link path")?;
            let harness_id = find_harness_id(connection, &request.harness)?;
            let source_text = path_to_db_text(&request.source_path, "source path")?;
            let link_text = request.link_path.components().filter_map(|part| match part {
                std::path::Component::Normal(value) => Some(path_to_db_text(Path::new(value), "link path")),
                _ => None,
            }).collect::<Result<Vec<_>>>()?.join("/");
            let key = mapping_path_key(&request.link_path);
            // Older records retain their spelling; compare their managed path keys.
            if list_harness_mappings(connection, Some(&harness_id))?.iter()
                .any(|mapping| mapping_path_key(&mapping.link_path) == key)
            {
                return Err(Error::invalid_arguments(format!("harness mapping destination already exists: {link_text}")));
            }
            let id = format!("mapping:{}:{}", harness_id, timestamp_nanos());
            let now = timestamp();
            connection.execute(
                "INSERT INTO harness_mappings(id,harness_id,source_path,link_path,link_kind,required,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,0,?6,?6)",
                params![id,harness_id,source_text,link_text,request.link_kind.to_string(),now],
            )?;
            find_mapping_by_id(connection, &id)
        })
    }
    pub fn remove_mapping(&self, harness: &str, link_path: &Path) -> Result<StoredHarnessMapping> {
        self.write(|connection| {
            validate_mapping_path(link_path, "link path")?;
            let harness_id = find_harness_id(connection, harness)?;
            let mappings = list_harness_mappings(connection, Some(&harness_id))?;
            let key = mapping_path_key(link_path);
            let mapping = mappings
                .iter()
                .find(|mapping| mapping.link_path.as_os_str() == link_path.as_os_str())
                .or_else(|| {
                    mappings
                        .iter()
                        .find(|mapping| mapping_path_key(&mapping.link_path) == key)
                })
                .ok_or_else(|| {
                    Error::NotFound(format!(
                        "harness `{harness_id}` has no mapping for `{}`",
                        link_path.display()
                    ))
                })?;
            if mapping.required {
                return Err(Error::invalid_arguments(format!(
                    "required harness mapping '{}' cannot be removed",
                    mapping.id
                )));
            }
            connection.execute("DELETE FROM harness_mappings WHERE id=?1", [&mapping.id])?;
            Ok(mapping.clone())
        })
    }
}
fn harness_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredHarness> {
    Ok(StoredHarness {
        id: row.get(0)?,
        name: row.get(1)?,
        display_name: row.get(2)?,
        built_in: row.get(3)?,
        enabled: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        mappings: Vec::new(),
    })
}
fn show_harness_with_connection(
    connection: &Connection,
    identifier: &str,
) -> Result<StoredHarness> {
    let mut harness=connection.query_row("SELECT id,name,display_name,built_in,enabled,created_at,updated_at FROM harnesses WHERE id=?1 OR name=?1",[identifier],harness_from_row).optional()?.ok_or_else(||Error::NotFound(format!("unknown harness '{identifier}'")))?;
    harness.mappings = list_harness_mappings(connection, Some(&harness.id))?;
    Ok(harness)
}
fn list_harness_mappings(
    connection: &rusqlite::Connection,
    harness_id: Option<&str>,
) -> Result<Vec<StoredHarnessMapping>> {
    let (sql, bind_harness) = match harness_id {
        Some(harness_id) => (
            r#"
            SELECT id, harness_id, source_path, link_path, link_kind, required, created_at, updated_at
            FROM harness_mappings
            WHERE harness_id = ?1
            ORDER BY id
            "#,
            Some(harness_id),
        ),
        None => (
            r#"
            SELECT id, harness_id, source_path, link_path, link_kind, required, created_at, updated_at
            FROM harness_mappings
            ORDER BY harness_id, id
            "#,
            None,
        ),
    };

    let mut statement = connection.prepare(sql)?;
    let mut mappings = Vec::new();

    match bind_harness {
        Some(harness_id) => {
            let rows = statement.query_map(params![harness_id], stored_mapping_from_row)?;
            for row in rows {
                mappings.push(row?);
            }
        }
        None => {
            let rows = statement.query_map([], stored_mapping_from_row)?;
            for row in rows {
                mappings.push(row?);
            }
        }
    }

    Ok(mappings)
}

fn stored_mapping_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredHarnessMapping> {
    let link_kind: String = row.get(4)?;
    Ok(StoredHarnessMapping {
        id: row.get(0)?,
        harness_id: row.get(1)?,
        source_path: PathBuf::from(row.get::<_, String>(2)?),
        link_path: PathBuf::from(row.get::<_, String>(3)?),
        link_kind: LinkKind::from_str(&link_kind).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        required: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn find_harness_id(connection: &rusqlite::Connection, harness: &str) -> Result<String> {
    connection
        .query_row(
            "SELECT id FROM harnesses WHERE id = ?1 OR name = ?1",
            params![harness],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("unknown harness `{harness}`")))
}

fn find_mapping_by_id(connection: &rusqlite::Connection, id: &str) -> Result<StoredHarnessMapping> {
    connection
        .query_row(
            r#"
            SELECT id, harness_id, source_path, link_path, link_kind, required, created_at, updated_at
            FROM harness_mappings
            WHERE id = ?1
            "#,
            params![id],
            stored_mapping_from_row,
        )
        .map_err(Error::from)
}

fn mapping_path_key(path: &Path) -> String {
    managed_relative_path_key(path)
        .to_string_lossy()
        .to_ascii_lowercase()
}

fn validate_mapping_path(path: &Path, label: &str) -> Result<()> {
    validate_managed_relative_path(path)
        .map_err(|error| Error::invalid_arguments(format!("{label}: {error}")))
}

fn path_to_db_text(path: &Path, label: &str) -> Result<String> {
    path.to_str().map(str::to_string).ok_or_else(|| {
        Error::invalid_arguments(format!(
            "{label} must be valid UTF-8 to store in the global registry"
        ))
    })
}

#[cfg(test)]
#[path = "harness_tests.rs"]
mod tests;
