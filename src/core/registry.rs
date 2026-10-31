//! Linkable item registry entry point.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension};

use crate::core::{
    error::{Error, Result},
    linkable::{
        normalize_target_dir, parse_item_type, parse_link_kind, validate_item_name,
        validate_optional_alias, validate_resource_source, validate_skill_source, LinkableItem,
        LinkableItemType,
    },
    paths::normalize_windows_path,
    store::Store,
    util::timestamp,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddLinkableItem {
    pub item_type: LinkableItemType,
    pub name: String,
    pub alias: Option<String>,
    pub source_path: PathBuf,
    pub default_target_dir: Option<PathBuf>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub items: Vec<LinkableItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupSummary {
    pub id: String,
    pub name: String,
    pub item_count: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupUpdateReport {
    pub group: Group,
    pub skipped: Vec<String>,
}
impl Store {
    pub fn add_item(&self, request: AddLinkableItem) -> Result<LinkableItem> {
        self.write(|c| add_item_with_connection(c, request))
    }
    pub fn list_items(&self, kind: LinkableItemType) -> Result<Vec<LinkableItem>> {
        list_items_with_connection(self.connection(), kind)
    }
    pub fn find_item(&self, kind: LinkableItemType, identifier: &str) -> Result<LinkableItem> {
        find_item_with_connection(self.connection(), kind, identifier)
    }
    pub fn find_any_item(&self, identifier: &str) -> Result<LinkableItem> {
        find_any_item_with_connection(self.connection(), identifier)
    }
    pub fn rename_item(
        &self,
        kind: LinkableItemType,
        identifier: &str,
        new_name: &str,
    ) -> Result<LinkableItem> {
        self.write(|c| {
            validate_item_name(new_name, "name")?;
            let item = find_item_with_connection(c, kind, identifier)?;
            let link_name = if kind == LinkableItemType::Skill && item.alias.is_none() {
                new_name.to_string()
            } else {
                item.link_name()
            };
            c.execute(
                "UPDATE linkable_items SET name=?2,default_link_name=?3,updated_at=?4 WHERE id=?1",
                params![item.id, new_name, link_name, timestamp()],
            )?;
            find_item_by_id(c, &item.id)
        })
    }
    pub fn remove_item(&self, kind: LinkableItemType, identifier: &str) -> Result<LinkableItem> {
        self.write(|c| {
            let item = find_item_with_connection(c, kind, identifier)?;
            c.execute("DELETE FROM linkable_items WHERE id=?1", [&item.id])?;
            Ok(item)
        })
    }
    pub fn refresh_item(&self, kind: LinkableItemType, identifier: &str) -> Result<LinkableItem> {
        self.write(|c| {
            let item = find_item_with_connection(c, kind, identifier)?;
            let source = validate_source_for_type(kind, &item.source_path)?;
            if source.source_kind != item.source_kind {
                return Err(Error::invalid_arguments(format!(
                    "{} '{}' source kind changed from {} to {}",
                    kind, item.name, item.source_kind, source.source_kind
                )));
            }
            let link_name = default_link_name(
                kind, &item.name, item.alias.as_deref(), &source.absolute_path,
            );
            c.execute(
                "UPDATE linkable_items SET source_path=?2,default_link_name=?3,updated_at=?4 WHERE id=?1",
                params![
                    item.id, path_to_registry_text(&source.absolute_path, "source path")?,
                    link_name, timestamp()
                ],
            )?;
            find_item_by_id(c, &item.id)
        })
    }
    pub fn create_group(&self, name: &str, description: Option<&str>) -> Result<Group> {
        self.write(|c| create_group_with_connection(c, name, description))
    }
    pub fn list_group_summaries(&self) -> Result<Vec<GroupSummary>> {
        let mut statement = self.connection().prepare(
            "SELECT g.id, g.name, COUNT(gi.item_id)
             FROM groups g
             LEFT JOIN group_items gi ON gi.group_id = g.id
             GROUP BY g.id
             ORDER BY g.name",
        )?;
        let rows = statement
            .query_map([], |r| {
                Ok(GroupSummary {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    item_count: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
    pub fn show_group(&self, identifier: &str) -> Result<Group> {
        find_group_with_connection(self.connection(), identifier)
    }
    pub fn rename_group(&self, identifier: &str, new_name: &str) -> Result<Group> {
        self.write(|c| rename_group_with_connection(c, identifier, new_name))
    }
    pub fn delete_group(&self, identifier: &str) -> Result<Group> {
        self.write(|c| delete_group_with_connection(c, identifier))
    }
    pub fn add_group_items(&self, group: &str, items: &[String]) -> Result<GroupUpdateReport> {
        self.write(|c| add_group_items_with_connection(c, group, items))
    }
    pub fn remove_group_items(&self, group: &str, items: &[String]) -> Result<Group> {
        self.write(|c| remove_group_items_with_connection(c, group, items))
    }
}

fn add_item_with_connection(
    connection: &Connection,
    request: AddLinkableItem,
) -> Result<LinkableItem> {
    validate_item_name(&request.name, "name")?;
    validate_optional_alias(request.alias.as_deref())?;
    let source = validate_source_for_type(request.item_type, &request.source_path)?;

    if request.default_target_dir.is_none() && request.item_type == LinkableItemType::Resource {
        return Err(Error::invalid_arguments(
            "resource add requires --target-dir for the default project link path",
        ));
    }

    let candidate = default_link_name(
        request.item_type,
        &request.name,
        request.alias.as_deref(),
        &source.absolute_path,
    );

    let now = timestamp();
    let id = random_id(connection, "itm")?;
    connection.execute(
        r#"
        INSERT INTO linkable_items
            (id, kind, name, alias, source_path, source_kind, target_dir, description,
             created_at, updated_at, default_link_name)
        VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?10)
        "#,
        params![
            id,
            request.item_type.as_str(),
            request.name,
            request.alias,
            path_to_registry_text(&source.absolute_path, "source path")?,
            source.source_kind.to_string(),
            request
                .default_target_dir
                .as_ref()
                .map(|path| normalize_target_dir(path))
                .transpose()?,
            request.description,
            now,
            candidate,
        ],
    )?;

    find_item_by_id(connection, &id)
}

fn list_items_with_connection(
    connection: &Connection,
    item_type: LinkableItemType,
) -> Result<Vec<LinkableItem>> {
    let mut statement = connection.prepare(
        r#"
        SELECT id, kind, name, alias, source_path, source_kind, target_dir, description,
               created_at, updated_at, default_link_name
        FROM linkable_items
        WHERE kind = ?1
        ORDER BY name
        "#,
    )?;
    let rows = statement.query_map(params![item_type.as_str()], item_from_row)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

fn find_item_with_connection(
    connection: &Connection,
    item_type: LinkableItemType,
    identifier: &str,
) -> Result<LinkableItem> {
    lookup_item(connection, identifier)?
        .filter(|item| item.item_type == item_type)
        .ok_or_else(|| Error::database(format!("unknown {} '{identifier}'", item_type.as_str())))
}

fn find_any_item_with_connection(
    connection: &Connection,
    identifier: &str,
) -> Result<LinkableItem> {
    lookup_item(connection, identifier)?
        .ok_or_else(|| Error::database(format!("unknown linkable item '{identifier}'")))
}

fn lookup_item(connection: &Connection, identifier: &str) -> Result<Option<LinkableItem>> {
    Ok(connection
        .query_row(
            "SELECT id, kind, name, alias, source_path, source_kind, target_dir, description,
                created_at, updated_at, default_link_name
         FROM linkable_items
         WHERE id = ?1 OR name = ?1
         ORDER BY (id = ?1) DESC",
            [identifier],
            item_from_row,
        )
        .optional()?)
}

fn find_item_by_id(connection: &Connection, id: &str) -> Result<LinkableItem> {
    connection
        .query_row(
            r#"
            SELECT id, kind, name, alias, source_path, source_kind, target_dir, description,
                   created_at, updated_at, default_link_name
            FROM linkable_items
            WHERE id = ?1
            "#,
            params![id],
            item_from_row,
        )
        .map_err(Error::from)
}

fn create_group_with_connection(
    connection: &Connection,
    name: &str,
    description: Option<&str>,
) -> Result<Group> {
    validate_item_name(name, "group name")?;

    let now = timestamp();
    let id = random_id(connection, "grp")?;
    connection.execute(
        r#"
        INSERT INTO groups (id, name, description, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?4)
        "#,
        params![id, name, description, now],
    )?;

    find_group_with_connection(connection, &id)
}

fn find_group_with_connection(connection: &Connection, identifier: &str) -> Result<Group> {
    let mut group = connection
        .query_row(
            r#"
            SELECT id, name, description, created_at, updated_at
            FROM groups
            WHERE id = ?1 OR name = ?1
            ORDER BY (id = ?1) DESC
            "#,
            params![identifier],
            group_from_row,
        )
        .optional()?
        .ok_or_else(|| Error::database(format!("unknown group `{identifier}`")))?;

    group.items = group_items_with_connection(connection, &group.id)?;
    Ok(group)
}

fn rename_group_with_connection(
    connection: &Connection,
    identifier: &str,
    new_name: &str,
) -> Result<Group> {
    validate_item_name(new_name, "group name")?;
    let group = find_group_with_connection(connection, identifier)?;

    connection.execute(
        r#"
        UPDATE groups
        SET name = ?2, updated_at = ?3
        WHERE id = ?1
        "#,
        params![group.id, new_name, timestamp()],
    )?;

    find_group_with_connection(connection, &group.id)
}

fn delete_group_with_connection(connection: &Connection, identifier: &str) -> Result<Group> {
    let group = find_group_with_connection(connection, identifier)?;
    connection.execute("DELETE FROM groups WHERE id = ?1", params![group.id])?;
    Ok(group)
}

fn add_group_items_with_connection(
    connection: &Connection,
    group_identifier: &str,
    item_identifiers: &[String],
) -> Result<GroupUpdateReport> {
    if item_identifiers.is_empty() {
        return Err(Error::invalid_arguments(
            "group add requires at least one item",
        ));
    }
    let group = find_group_with_connection(connection, group_identifier)?;
    let mut seen = BTreeSet::new();
    let mut skipped = Vec::new();
    let mut changed = false;
    for identifier in item_identifiers {
        let item = find_any_item_with_connection(connection, identifier)?;
        if !seen.insert(item.id.clone()) {
            continue;
        }
        if group.items.iter().any(|existing| existing.id == item.id) {
            skipped.push(item.name);
            continue;
        }
        connection.execute(
            "INSERT INTO group_items(group_id,item_id,created_at) VALUES(?1,?2,?3)",
            params![group.id, item.id, timestamp()],
        )?;
        changed = true;
    }
    if changed {
        connection.execute(
            "UPDATE groups SET updated_at=?2 WHERE id=?1",
            params![group.id, timestamp()],
        )?;
    }
    Ok(GroupUpdateReport {
        group: find_group_with_connection(connection, &group.id)?,
        skipped,
    })
}
fn remove_group_items_with_connection(
    connection: &Connection,
    group_identifier: &str,
    item_identifiers: &[String],
) -> Result<Group> {
    if item_identifiers.is_empty() {
        return Err(Error::invalid_arguments(
            "group remove requires at least one item",
        ));
    }
    let group = find_group_with_connection(connection, group_identifier)?;
    let mut seen = BTreeSet::new();
    for identifier in item_identifiers {
        let item = find_any_item_with_connection(connection, identifier)?;
        if !seen.insert(item.id.clone()) {
            continue;
        }
        let changed = connection.execute(
            "DELETE FROM group_items WHERE group_id=?1 AND item_id=?2",
            params![group.id, item.id],
        )?;
        if changed == 0 {
            return Err(Error::invalid_arguments(format!(
                "group '{}' does not contain item '{}'",
                group.name, item.name
            )));
        }
    }
    connection.execute(
        "UPDATE groups SET updated_at=?2 WHERE id=?1",
        params![group.id, timestamp()],
    )?;
    find_group_with_connection(connection, &group.id)
}

fn group_items_with_connection(
    connection: &Connection,
    group_id: &str,
) -> Result<Vec<LinkableItem>> {
    let mut statement = connection.prepare(
        r#"
        SELECT i.id, i.kind, i.name, i.alias, i.source_path, i.source_kind, i.target_dir,
               i.description, i.created_at, i.updated_at, i.default_link_name
        FROM group_items gi
        JOIN linkable_items i ON i.id = gi.item_id
        WHERE gi.group_id = ?1
        ORDER BY i.kind, i.name
        "#,
    )?;
    let rows = statement.query_map(params![group_id], item_from_row)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

fn group_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Group> {
    Ok(Group {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        items: Vec::new(),
    })
}

fn item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LinkableItem> {
    let item_type =
        parse_item_type(&row.get::<_, String>(1)?).map_err(|error| conversion_error(1, error))?;
    let source_kind =
        parse_link_kind(&row.get::<_, String>(5)?).map_err(|error| conversion_error(5, error))?;
    Ok(LinkableItem {
        id: row.get(0)?,
        item_type,
        name: row.get(2)?,
        alias: row.get(3)?,
        source_path: normalize_windows_path(Path::new(&row.get::<_, String>(4)?)),
        source_kind,
        default_target_dir: row.get::<_, Option<String>>(6)?.map(PathBuf::from),
        description: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        default_link_name: row.get(10)?,
    })
}

fn conversion_error(index: usize, error: Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(error))
}

fn validate_source_for_type(
    item_type: LinkableItemType,
    source_path: &Path,
) -> Result<crate::core::linkable::ValidatedSource> {
    match item_type {
        LinkableItemType::Skill => validate_skill_source(source_path),
        LinkableItemType::Resource => validate_resource_source(source_path),
    }
}

fn path_to_registry_text(path: &Path, label: &str) -> Result<String> {
    path.to_str().map(str::to_string).ok_or_else(|| {
        Error::invalid_arguments(format!(
            "{label} must be valid UTF-8 to store in the global registry"
        ))
    })
}

fn random_id(connection: &Connection, prefix: &str) -> Result<String> {
    let random: String =
        connection.query_row("SELECT lower(hex(randomblob(8)))", [], |r| r.get(0))?;
    Ok(format!("{prefix}_{random}"))
}
fn default_link_name(
    kind: LinkableItemType,
    name: &str,
    alias: Option<&str>,
    source: &Path,
) -> String {
    alias.map(str::to_string).unwrap_or_else(|| match kind {
        LinkableItemType::Skill => name.to_string(),
        LinkableItemType::Resource => source
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(name)
            .to_string(),
    })
}
#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
