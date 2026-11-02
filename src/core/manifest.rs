//! Project `.agents/links.toml` manifest entry point.

use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::core::{
    error::{Error, Result},
    harness::{default_init_mappings, HarnessMapping},
    paths::{
        managed_relative_path_key, resolve_managed_project_path, validate_managed_relative_path,
        DbPathResolution,
    },
    store::Store,
    symlink::{
        ensure_symlink, CreateSymlinkOptions, CreateSymlinkOutcome, LinkKind, SymlinkBackend,
        SymlinkProvider,
    },
    util::timestamp,
};

const MANIFEST_VERSION: u32 = 1;
const MANIFEST_PATH: &str = ".agents/links.toml";
const MANAGED_GITIGNORE_BEGIN: &str = "# BEGIN aglink managed";
const MANAGED_GITIGNORE_END: &str = "# END aglink managed";
const MANAGED_GITIGNORE_PATTERNS: &[&str] = &[".claude/", ".agents/"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkOrigin {
    Harness,
    Item,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredLinkRecord")]
pub struct LinkRecord {
    pub id: String,
    pub origin: LinkOrigin,
    /// Present only for Harness links; serialized as `framework_name` in schema 1.
    #[serde(rename = "framework_name", skip_serializing_if = "Option::is_none")]
    pub harness_name: Option<String>,
    pub item_id: String,
    pub item_name: String,
    #[serde(serialize_with = "serialize_path")]
    pub source_path: PathBuf,
    #[serde(serialize_with = "serialize_path")]
    pub link_path: PathBuf,
    pub link_kind: LinkKind,
    pub provider_backend: String,
    pub created_by_command: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    #[serde(default)]
    pub links: Vec<LinkRecord>,
}

impl Manifest {
    pub fn empty() -> Self {
        Self {
            schema_version: MANIFEST_VERSION,
            links: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.schema_version != MANIFEST_VERSION {
            return Err(Error::manifest(format!(
                "unsupported manifest schema_version {}; expected {MANIFEST_VERSION}",
                self.schema_version
            )));
        }

        let mut seen_ids = BTreeSet::new();
        let mut seen_paths = BTreeSet::new();
        for link in &self.links {
            require_non_empty("id", &link.id)?;
            match (link.origin, link.harness_name.as_deref()) {
                (LinkOrigin::Harness, Some(name)) => require_non_empty("framework_name", name)?,
                (LinkOrigin::Item, None) => {}
                _ => {
                    return Err(Error::manifest(
                        "only harness links must have a framework_name",
                    ))
                }
            }
            require_non_empty("item_id", &link.item_id)?;
            require_non_empty("item_name", &link.item_name)?;
            require_non_empty("created_by_command", &link.created_by_command)?;

            if link.source_path.as_os_str().is_empty() {
                return Err(Error::manifest(format!(
                    "manifest link `{}` has empty source_path",
                    link.id
                )));
            }

            validate_managed_relative_path(&link.link_path).map_err(|error| {
                Error::manifest(format!("manifest link `{}`: {error}", link.id))
            })?;
            let key = managed_relative_path_key(&link.link_path);
            if !seen_paths.insert(key) {
                return Err(Error::manifest(format!(
                    "manifest contains duplicate link_path `{}`",
                    link.link_path.display()
                )));
            }

            if !seen_ids.insert(link.id.clone()) {
                return Err(Error::manifest(format!(
                    "manifest contains duplicate link id `{}`",
                    link.id
                )));
            }
        }

        Ok(())
    }

    pub fn upsert(&mut self, incoming: LinkRecord) {
        if let Some(existing) = self.links.iter_mut().find(|link| link.id == incoming.id) {
            let created_at = existing.created_at.clone();
            *existing = incoming;
            existing.created_at = created_at;
            return;
        }

        self.links.push(incoming);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitProjectReport {
    pub project_root: PathBuf,
    pub manifest_path: PathBuf,
    pub gitignore_path: PathBuf,
    pub created_paths: Vec<PathBuf>,
    pub preserved_paths: Vec<PathBuf>,
    pub link_outcomes: Vec<InitLinkOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitLinkOutcome {
    pub id: String,
    pub source_path: PathBuf,
    pub link_path: PathBuf,
    pub outcome: CreateSymlinkOutcome,
}

pub fn init_project_with_store(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    resolution: &DbPathResolution,
) -> Result<InitProjectReport> {
    let mappings = match fs::metadata(&resolution.path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => default_init_mappings(),
        Err(error) => return Err(Error::io(&resolution.path, error)),
        Ok(_) => Store::open_ro(resolution)?
            .list_harnesses()?
            .into_iter()
            .filter(|harness| harness.enabled)
            .flat_map(|harness| {
                harness
                    .mappings
                    .into_iter()
                    .map(move |mapping| mapping.for_project(&harness.name))
            })
            .collect(),
    };
    init_project_with_mappings(project_root, provider, &mappings)
}

pub fn init_project_with_mappings(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    mappings: &[HarnessMapping],
) -> Result<InitProjectReport> {
    let mut report = InitProjectReport {
        project_root: project_root.to_path_buf(),
        manifest_path: manifest_path(project_root),
        gitignore_path: project_root.join(".gitignore"),
        created_paths: Vec::new(),
        preserved_paths: Vec::new(),
        link_outcomes: Vec::new(),
    };

    ensure_real_file(project_root, Path::new("AGENTS.md"), &mut report)?;
    ensure_real_dir(project_root, Path::new(".agents"), &mut report)?;
    ensure_real_dir(
        project_root,
        &PathBuf::from(".agents").join("skills"),
        &mut report,
    )?;

    let mut manifest = load_manifest(project_root)?;

    ensure_harness_parent_dirs(project_root, mappings, &mut report)?;
    update_gitignore(project_root, &mut report)?;

    for mapping in mappings {
        let source = mapping.source_in(project_root)?;
        let link = mapping.link_in(project_root)?;
        let outcome = ensure_symlink(
            provider,
            &source,
            &link,
            mapping.link_kind,
            CreateSymlinkOptions { force: false },
        )?;

        report.link_outcomes.push(InitLinkOutcome {
            id: mapping.id.to_string(),
            source_path: mapping.source_path.clone(),
            link_path: mapping.link_path.clone(),
            outcome,
        });

        manifest.upsert(record_from_mapping(mapping, provider.backend()));
    }

    manifest.validate()?;
    save_manifest(project_root, &manifest)?;

    Ok(report)
}

pub fn manifest_path(project_root: &Path) -> PathBuf {
    project_root.join(MANIFEST_PATH)
}

pub fn load_manifest(project_root: &Path) -> Result<Manifest> {
    let path = resolve_managed_project_path(project_root, Path::new(MANIFEST_PATH))?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Manifest::empty());
        }
        Err(error) => return Err(Error::io(&path, error)),
    };

    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(Error::manifest("manifest path must be a real file").at_path(&path));
    }

    let content = fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
    let manifest = parse_manifest(&content).map_err(|error| error.at_path(&path))?;
    validate_project_paths(project_root, &manifest)?;
    Ok(manifest)
}

pub fn save_manifest(project_root: &Path, manifest: &Manifest) -> Result<()> {
    save_manifest_with_io(
        project_root,
        manifest,
        |file, content| file.write_all(content),
        |source, destination| fs::rename(source, destination),
    )
}

fn save_manifest_with_io(
    project_root: &Path,
    manifest: &Manifest,
    write: impl FnOnce(&mut fs::File, &[u8]) -> std::io::Result<()>,
    rename: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
) -> Result<()> {
    manifest
        .validate()
        .map_err(|error| error.at_path(manifest_path(project_root)))?;
    validate_project_paths(project_root, manifest)?;

    let path = resolve_managed_project_path(project_root, Path::new(MANIFEST_PATH))?;
    let Some(parent) = path.parent() else {
        return Err(Error::manifest("manifest path has no parent"));
    };

    // The manifest leaf must never redirect a write through a symlink.
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(Error::manifest("manifest path must be a real file").at_path(&path));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(Error::io(&path, error)),
    }
    let temporary = resolve_managed_project_path(
        project_root,
        &PathBuf::from(".agents").join(format!(".links.toml.tmp.{}", std::process::id())),
    )?;
    fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
    // Never follow or overwrite an existing temporary leaf, including symlinks.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| Error::io(&temporary, error))?;
    let written = write(&mut file, serialize_manifest(manifest).as_bytes());
    drop(file);
    let result = written.and_then(|_| rename(&temporary, &path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| Error::io(&path, error))?;
    Ok(())
}

fn validate_project_paths(project_root: &Path, manifest: &Manifest) -> Result<()> {
    for record in &manifest.links {
        resolve_managed_project_path(project_root, &record.link_path)?;
        if !record.source_path.is_absolute() {
            resolve_managed_project_path(project_root, &record.source_path)?;
        }
    }
    Ok(())
}

fn ensure_harness_parent_dirs(
    project_root: &Path,
    mappings: &[HarnessMapping],
    report: &mut InitProjectReport,
) -> Result<()> {
    let mut parents = BTreeSet::new();
    for mapping in mappings {
        if let Some(parent) = mapping.link_path.parent() {
            if parent
                .components()
                .any(|part| matches!(part, std::path::Component::Normal(_)))
            {
                parents.insert(parent.to_path_buf());
            }
        }
    }

    for parent in parents {
        ensure_real_dir(project_root, &parent, report)?;
    }

    Ok(())
}

fn ensure_real_file(
    project_root: &Path,
    relative_path: &Path,
    report: &mut InitProjectReport,
) -> Result<()> {
    let path = resolve_managed_project_path(project_root, relative_path)?;
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(Error::project(format!(
            "{} must be a real file, not a symlink",
            relative_path.display()
        ))),
        Ok(metadata) if metadata.is_file() => {
            report.preserved_paths.push(relative_path.to_path_buf());
            Ok(())
        }
        Ok(metadata) if metadata.is_dir() => Err(Error::project(format!(
            "{} must be a real file, found directory",
            relative_path.display()
        ))),
        Ok(_) => Err(Error::project(format!(
            "{} must be a regular file",
            relative_path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
            }
            fs::write(&path, "").map_err(|error| Error::io(&path, error))?;
            report.created_paths.push(relative_path.to_path_buf());
            Ok(())
        }
        Err(error) => Err(Error::io(&path, error)),
    }
}

fn ensure_real_dir(
    project_root: &Path,
    relative_path: &Path,
    report: &mut InitProjectReport,
) -> Result<()> {
    let path = resolve_managed_project_path(project_root, relative_path)?;
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(Error::project(format!(
            "{} must be a real directory, not a symlink",
            relative_path.display()
        ))),
        Ok(metadata) if metadata.is_dir() => {
            report.preserved_paths.push(relative_path.to_path_buf());
            Ok(())
        }
        Ok(metadata) if metadata.is_file() => Err(Error::project(format!(
            "{} must be a directory, found file",
            relative_path.display()
        ))),
        Ok(_) => Err(Error::project(format!(
            "{} must be a regular directory",
            relative_path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(&path).map_err(|error| Error::io(&path, error))?;
            report.created_paths.push(relative_path.to_path_buf());
            Ok(())
        }
        Err(error) => Err(Error::io(&path, error)),
    }
}

fn update_gitignore(project_root: &Path, report: &mut InitProjectReport) -> Result<()> {
    let path = resolve_managed_project_path(project_root, Path::new(".gitignore"))?;
    let block = managed_gitignore_block();

    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(Error::project(
                ".gitignore must be a real file when it exists",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::write(&path, block).map_err(|error| Error::io(&path, error))?;
            report.created_paths.push(PathBuf::from(".gitignore"));
            return Ok(());
        }
        Err(error) => return Err(Error::io(&path, error)),
    }

    let original = fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
    let updated = replace_or_append_managed_block(&original, &block)?;
    if updated != original {
        fs::write(&path, updated).map_err(|error| Error::io(&path, error))?;
    }
    report.preserved_paths.push(PathBuf::from(".gitignore"));
    Ok(())
}

fn managed_gitignore_block() -> String {
    let mut block = String::new();
    block.push_str(MANAGED_GITIGNORE_BEGIN);
    block.push('\n');
    for pattern in MANAGED_GITIGNORE_PATTERNS {
        block.push_str(pattern);
        block.push('\n');
    }
    block.push_str(MANAGED_GITIGNORE_END);
    block.push('\n');
    block
}

fn replace_or_append_managed_block(original: &str, block: &str) -> Result<String> {
    let begin = original.find(MANAGED_GITIGNORE_BEGIN);
    let end = original.find(MANAGED_GITIGNORE_END);

    match (begin, end) {
        (Some(begin), Some(end)) if begin <= end => {
            let after_end = end + MANAGED_GITIGNORE_END.len();
            let mut updated = String::new();
            updated.push_str(&original[..begin]);
            updated.push_str(block.trim_end_matches('\n'));
            updated.push_str(&original[after_end..]);
            Ok(updated)
        }
        (None, None) => {
            let mut updated = original.to_string();
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            if !updated.is_empty() {
                updated.push('\n');
            }
            updated.push_str(block);
            Ok(updated)
        }
        _ => Err(Error::project(
            ".gitignore contains a partial aglink managed block",
        )),
    }
}

fn record_from_mapping(mapping: &HarnessMapping, backend: SymlinkBackend) -> LinkRecord {
    let now = timestamp();

    LinkRecord {
        id: mapping.id.to_string(),
        origin: LinkOrigin::Harness,
        harness_name: Some(mapping.harness_name.to_string()),
        item_id: mapping.item_id.to_string(),
        item_name: mapping.item_name.to_string(),
        source_path: mapping.source_path.clone(),
        link_path: mapping.link_path.clone(),
        link_kind: mapping.link_kind,
        provider_backend: backend.to_string(),
        created_by_command: "init".to_string(),
        created_at: now.clone(),
        updated_at: now,
    }
}

fn serialize_manifest(manifest: &Manifest) -> String {
    toml::to_string_pretty(manifest).expect("manifest contains only TOML-compatible fields")
}

fn parse_manifest(input: &str) -> Result<Manifest> {
    let manifest: Manifest = toml::from_str(input).map_err(Error::parse)?;
    manifest.validate()?;
    Ok(manifest)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredLinkRecord {
    id: String,
    origin: Option<LinkOrigin>,
    framework_name: Option<String>,
    #[serde(rename = "scope")]
    _scope: Option<String>,
    item_id: String,
    item_name: String,
    source_path: PathBuf,
    link_path: PathBuf,
    link_kind: LinkKind,
    provider_backend: String,
    created_by_command: String,
    #[serde(deserialize_with = "deserialize_timestamp")]
    created_at: i64,
    #[serde(deserialize_with = "deserialize_timestamp")]
    updated_at: i64,
}

impl TryFrom<StoredLinkRecord> for LinkRecord {
    type Error = Error;
    fn try_from(raw: StoredLinkRecord) -> Result<Self> {
        let mut harness_name = raw.framework_name;
        let origin = match raw.origin {
            Some(origin) => origin,
            None => match harness_name.as_deref() {
                Some("registry") => {
                    harness_name = None;
                    LinkOrigin::Item
                }
                Some(_) => LinkOrigin::Harness,
                None => return Err(Error::manifest("manifest link is missing origin")),
            },
        };
        Ok(Self {
            id: raw.id,
            origin,
            harness_name,
            item_id: raw.item_id,
            item_name: raw.item_name,
            source_path: raw.source_path,
            link_path: raw.link_path,
            link_kind: raw.link_kind,
            provider_backend: raw.provider_backend,
            created_by_command: raw.created_by_command,
            created_at: raw.created_at,
            updated_at: raw.updated_at,
        })
    }
}

fn deserialize_timestamp<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<i64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Timestamp {
        Integer(i64),
        Legacy(String),
    }
    match Timestamp::deserialize(deserializer)? {
        Timestamp::Integer(value) => Ok(value),
        Timestamp::Legacy(value) => value
            .strip_prefix("unix:")
            .unwrap_or(&value)
            .parse()
            .map_err(serde::de::Error::custom),
    }
}

fn serialize_path<S: serde::Serializer>(
    path: &Path,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.serialize_str(&path_to_manifest(path))
}

fn require_non_empty(field: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        Err(Error::manifest(format!(
            "manifest field `{field}` is empty"
        )))
    } else {
        Ok(())
    }
}

fn path_to_manifest(path: &Path) -> String {
    let path = path.to_string_lossy();
    // Absolute source paths must round-trip with native separators so Windows
    // canonical paths continue to match the symlink target reported by the OS.
    if Path::new(path.as_ref()).is_absolute() {
        path.to_string()
    } else {
        #[cfg(windows)]
        {
            path.replace('\\', "/")
        }
        #[cfg(not(windows))]
        {
            path.to_string()
        }
    }
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
