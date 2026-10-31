//! Skill and resource domain model entry point.

use std::{
    fmt, fs,
    path::{Component, Path, PathBuf},
    str::FromStr,
};

use crate::core::{
    error::{Error, Result},
    paths::normalize_windows_path,
    symlink::LinkKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkableItemType {
    Skill,
    Resource,
}

impl LinkableItemType {
    pub const fn as_str(self) -> &'static str {
        match self {
            LinkableItemType::Skill => "skill",
            LinkableItemType::Resource => "resource",
        }
    }
}

impl fmt::Display for LinkableItemType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkableItem {
    pub id: String,
    pub name: String,
    pub alias: Option<String>,
    pub default_link_name: String,
    pub item_type: LinkableItemType,
    pub source_kind: LinkKind,
    pub source_path: PathBuf,
    pub default_target_dir: Option<PathBuf>,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl LinkableItem {
    pub fn link_name(&self) -> String {
        self.default_link_name.clone()
    }

    pub fn default_project_link_path(&self) -> Result<PathBuf> {
        let link_name = self.link_name();
        match self.item_type {
            LinkableItemType::Skill => Ok(PathBuf::from(".agents").join("skills").join(link_name)),
            LinkableItemType::Resource => {
                let Some(target_dir) = &self.default_target_dir else {
                    return Err(Error::invalid_arguments(format!(
                        "resource `{}` does not have a default target directory",
                        self.name
                    )));
                };
                Ok(target_dir.join(link_name))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedSource {
    pub absolute_path: PathBuf,
    pub source_kind: LinkKind,
}

pub fn validate_skill_source(source_path: &Path) -> Result<ValidatedSource> {
    let source = validate_source_path(source_path)?;
    if source.source_kind != LinkKind::Directory {
        return Err(Error::invalid_arguments(format!(
            "skill source must be a directory: {}",
            source_path.display()
        )));
    }

    let skill_md = source.absolute_path.join("SKILL.md");
    let content = fs::read_to_string(&skill_md).map_err(|error| {
        Error::invalid_arguments(format!(
            "skill source must contain readable SKILL.md at {}: {error}",
            skill_md.display()
        ))
    })?;

    if content.trim().is_empty() {
        return Err(Error::invalid_arguments(format!(
            "skill source SKILL.md must not be empty: {}",
            skill_md.display()
        )));
    }

    Ok(source)
}

pub fn validate_resource_source(source_path: &Path) -> Result<ValidatedSource> {
    validate_source_path(source_path)
}

pub fn validate_item_name(name: &str, label: &str) -> Result<()> {
    validate_path_segment(name, label)
}

pub fn validate_optional_alias(alias: Option<&str>) -> Result<()> {
    if let Some(alias) = alias {
        validate_path_segment(alias, "alias")?;
    }
    Ok(())
}

pub fn validate_project_relative_target_dir(target_dir: &Path) -> Result<()> {
    if target_dir.as_os_str().is_empty() {
        return Err(Error::invalid_arguments(
            "target directory must not be empty",
        ));
    }

    if target_dir.is_absolute() {
        return Err(Error::invalid_arguments(format!(
            "target directory must be relative to the project root: {}",
            target_dir.display()
        )));
    }

    if target_dir.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(Error::invalid_arguments(format!(
            "target directory must stay inside the project: {}",
            target_dir.display()
        )));
    }

    Ok(())
}

fn validate_source_path(source_path: &Path) -> Result<ValidatedSource> {
    let absolute_path = fs::canonicalize(source_path).map_err(|error| {
        Error::invalid_arguments(format!(
            "source path is not readable or does not exist: {}; {error}",
            source_path.display()
        ))
    })?;
    let absolute_path = normalize_windows_path(&absolute_path);
    let metadata = fs::metadata(&absolute_path)?;
    let source_kind = if metadata.is_file() {
        LinkKind::File
    } else if metadata.is_dir() {
        LinkKind::Directory
    } else {
        return Err(Error::invalid_arguments(format!(
            "source path must be a file or directory: {}",
            absolute_path.display()
        )));
    };

    Ok(ValidatedSource {
        absolute_path,
        source_kind,
    })
}

fn validate_path_segment(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(Error::invalid_arguments(format!(
            "{label} must not be empty"
        )));
    }

    let path = Path::new(value);
    if path.components().count() != 1
        || value.contains('/')
        || value.contains('\\')
        || value.chars().any(is_windows_reserved_name_char)
        || matches!(
            path.components().next(),
            Some(
                Component::CurDir
                    | Component::ParentDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        )
    {
        return Err(Error::invalid_arguments(format!(
            "{label} must be a single path segment: {value}"
        )));
    }

    Ok(())
}

fn is_windows_reserved_name_char(character: char) -> bool {
    matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
}

pub fn parse_item_type(value: &str) -> Result<LinkableItemType> {
    match value {
        "skill" => Ok(LinkableItemType::Skill),
        "resource" => Ok(LinkableItemType::Resource),
        _ => Err(Error::database(format!(
            "unknown linkable item type `{value}`"
        ))),
    }
}

pub fn parse_link_kind(value: &str) -> Result<LinkKind> {
    LinkKind::from_str(value).map_err(|_| Error::database(format!("unknown source kind `{value}`")))
}

/// Canonical project-relative spelling used by the Resource uniqueness key.
pub fn normalize_target_dir(path: &Path) -> Result<String> {
    validate_project_relative_target_dir(path)?;
    let text = path
        .to_str()
        .ok_or_else(|| Error::invalid_arguments("target directory must be UTF-8"))?;
    let parts: Vec<_> = text
        .split(['/', '\\'])
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();
    Ok(if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    })
}
