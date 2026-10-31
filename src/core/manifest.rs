//! Project `.agents/links.toml` manifest entry point.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::core::{
    error::{Error, Result},
    harness::{default_init_mappings, HarnessMapping},
    paths::{
        managed_relative_path_key, resolve_database_path, resolve_managed_project_path,
        validate_managed_relative_path, DbPathResolution,
    },
    store::Store,
    symlink::{
        default_provider, ensure_symlink, CreateSymlinkOptions, CreateSymlinkOutcome, LinkKind,
        SymlinkBackend, SymlinkProvider,
    },
    util::timestamp,
};

const MANIFEST_VERSION: u32 = 1;
const MANIFEST_PATH: &str = ".agents/links.toml";
const MANAGED_GITIGNORE_BEGIN: &str = "# BEGIN aglink managed";
const MANAGED_GITIGNORE_END: &str = "# END aglink managed";
const MANAGED_GITIGNORE_PATTERNS: &[&str] = &[".claude/", ".agents/"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOrigin {
    Harness,
    Item,
}

impl LinkOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Harness => "harness",
            Self::Item => "item",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRecord {
    pub id: String,
    pub origin: LinkOrigin,
    /// Present only for Harness links; serialized as `framework_name` in schema 1.
    pub harness_name: Option<String>,
    pub item_id: String,
    pub item_name: String,
    pub source_path: PathBuf,
    pub link_path: PathBuf,
    pub link_kind: LinkKind,
    pub provider_backend: SymlinkBackend,
    pub created_by_command: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub schema_version: u32,
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

pub fn init_current_project() -> Result<InitProjectReport> {
    let project_root = std::env::current_dir()?;
    let mut provider = default_provider();
    init_project_with_provider(&project_root, provider.as_mut())
}

pub fn init_project_with_provider(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
) -> Result<InitProjectReport> {
    init_project_with_store(project_root, provider, &resolve_database_path()?)
}

pub fn init_project_with_store(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    resolution: &DbPathResolution,
) -> Result<InitProjectReport> {
    let mappings = match fs::metadata(&resolution.path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => default_init_mappings(),
        Err(error) => return Err(error.into()),
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
            CreateSymlinkOptions::new(),
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
        Err(error) => return Err(error.into()),
    };

    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(Error::manifest(format!(
            "manifest path must be a real file: {}",
            path.display()
        )));
    }

    let manifest = parse_manifest(&fs::read_to_string(&path)?)?;
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
    manifest.validate()?;
    validate_project_paths(project_root, manifest)?;

    let path = resolve_managed_project_path(project_root, Path::new(MANIFEST_PATH))?;
    let Some(parent) = path.parent() else {
        return Err(Error::manifest("manifest path has no parent"));
    };

    // The manifest leaf must never redirect a write through a symlink.
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(Error::manifest(format!(
                "manifest path must be a real file: {}",
                path.display()
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let temporary = resolve_managed_project_path(
        project_root,
        &PathBuf::from(".agents").join(format!(".links.toml.tmp.{}", std::process::id())),
    )?;
    fs::create_dir_all(parent)?;
    // Never follow or overwrite an existing temporary leaf, including symlinks.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let written = write(&mut file, serialize_manifest(manifest).as_bytes());
    drop(file);
    let result = written.and_then(|_| rename(&temporary, &path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
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
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, "")?;
            report.created_paths.push(relative_path.to_path_buf());
            Ok(())
        }
        Err(error) => Err(error.into()),
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
            fs::create_dir_all(&path)?;
            report.created_paths.push(relative_path.to_path_buf());
            Ok(())
        }
        Err(error) => Err(error.into()),
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
            fs::write(&path, block)?;
            report.created_paths.push(PathBuf::from(".gitignore"));
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    }

    let original = fs::read_to_string(&path)?;
    let updated = replace_or_append_managed_block(&original, &block)?;
    if updated != original {
        fs::write(&path, updated)?;
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
        provider_backend: backend,
        created_by_command: "init".to_string(),
        created_at: now.clone(),
        updated_at: now,
    }
}

fn serialize_manifest(manifest: &Manifest) -> String {
    let mut output = format!("schema_version = {}\n", manifest.schema_version);

    for link in &manifest.links {
        output.push('\n');
        output.push_str("[[links]]\n");
        push_kv(&mut output, "id", &link.id);
        push_kv(&mut output, "origin", link.origin.as_str());
        if let Some(name) = &link.harness_name {
            push_kv(&mut output, "framework_name", name);
        }
        push_kv(&mut output, "item_id", &link.item_id);
        push_kv(&mut output, "item_name", &link.item_name);
        push_kv(
            &mut output,
            "source_path",
            &path_to_manifest(&link.source_path),
        );
        push_kv(&mut output, "link_path", &path_to_manifest(&link.link_path));
        push_kv(&mut output, "link_kind", &link.link_kind.to_string());
        push_kv(
            &mut output,
            "provider_backend",
            &link.provider_backend.to_string(),
        );
        push_kv(&mut output, "created_by_command", &link.created_by_command);
        output.push_str(&format!("created_at = {}\n", link.created_at));
        output.push_str(&format!("updated_at = {}\n", link.updated_at));
    }

    output
}

fn push_kv(output: &mut String, key: &str, value: &str) {
    output.push_str(key);
    output.push_str(" = \"");
    output.push_str(&escape_manifest_string(value));
    output.push_str("\"\n");
}

fn parse_manifest(input: &str) -> Result<Manifest> {
    let mut schema_version = None;
    let mut links = Vec::new();
    let mut current = None;

    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line == "[[links]]" {
            if let Some(table) = current.take() {
                links.push(link_from_table(table)?);
            }
            current = Some(BTreeMap::new());
            continue;
        }

        let (key, value) = parse_key_value(line, line_number)?;
        if key == "schema_version" && current.is_none() {
            if schema_version.is_some() {
                return Err(Error::manifest("manifest schema_version is duplicated"));
            }
            schema_version = Some(value.parse::<u32>().map_err(|_| {
                Error::manifest(format!("invalid schema_version on line {line_number}"))
            })?);
            continue;
        }

        let Some(table) = current.as_mut() else {
            return Err(Error::manifest(format!(
                "manifest key `{key}` appears outside [[links]] on line {line_number}"
            )));
        };

        let value =
            if matches!(key.as_str(), "created_at" | "updated_at") && !value.starts_with('"') {
                value
            } else {
                parse_string_value(&value, line_number)?
            };
        if table.insert(key, value).is_some() {
            return Err(Error::manifest(format!(
                "manifest key is duplicated on line {line_number}"
            )));
        }
    }

    if let Some(table) = current {
        links.push(link_from_table(table)?);
    }

    let Some(schema_version) = schema_version else {
        return Err(Error::manifest("manifest is missing schema_version"));
    };

    let manifest = Manifest {
        schema_version,
        links,
    };
    manifest.validate()?;
    Ok(manifest)
}

fn parse_key_value(line: &str, line_number: usize) -> Result<(String, String)> {
    let Some((key, value)) = line.split_once('=') else {
        return Err(Error::manifest(format!(
            "invalid manifest line {line_number}: expected key = value"
        )));
    };

    let key = key.trim();
    if key.is_empty() {
        return Err(Error::manifest(format!(
            "invalid manifest line {line_number}: empty key"
        )));
    }

    Ok((key.to_string(), value.trim().to_string()))
}

fn parse_string_value(value: &str, line_number: usize) -> Result<String> {
    if !value.starts_with('"') || !value.ends_with('"') || value.len() < 2 {
        return Err(Error::manifest(format!(
            "invalid manifest string on line {line_number}"
        )));
    }

    unescape_manifest_string(&value[1..value.len() - 1], line_number)
}

fn link_from_table(mut table: BTreeMap<String, String>) -> Result<LinkRecord> {
    let id = required_field(&mut table, "id")?;
    // Legacy scope was always project; origin replaces the registry sentinel.
    table.remove("scope");
    let mut harness_name = table.remove("framework_name");
    let origin = match table.remove("origin").as_deref() {
        Some("harness") => LinkOrigin::Harness,
        Some("item") => LinkOrigin::Item,
        Some(value) => return Err(Error::manifest(format!("unknown origin `{value}`"))),
        None => match harness_name.as_deref() {
            Some("registry") => {
                harness_name = None;
                LinkOrigin::Item
            }
            Some(_) => LinkOrigin::Harness,
            None => return Err(Error::manifest("manifest link is missing `origin`")),
        },
    };
    let link = LinkRecord {
        id,
        origin,
        harness_name,
        item_id: required_field(&mut table, "item_id")?,
        item_name: required_field(&mut table, "item_name")?,
        source_path: PathBuf::from(required_field(&mut table, "source_path")?),
        link_path: PathBuf::from(required_field(&mut table, "link_path")?),
        link_kind: parse_link_kind(&required_field(&mut table, "link_kind")?)?,
        provider_backend: parse_backend(&required_field(&mut table, "provider_backend")?)?,
        created_by_command: required_field(&mut table, "created_by_command")?,
        created_at: parse_timestamp(&required_field(&mut table, "created_at")?)?,
        updated_at: parse_timestamp(&required_field(&mut table, "updated_at")?)?,
    };

    if let Some(extra) = table.keys().next() {
        return Err(Error::manifest(format!(
            "manifest link `{}` contains unknown key `{extra}`",
            link.id
        )));
    }

    Ok(link)
}

fn parse_timestamp(value: &str) -> Result<i64> {
    value
        .strip_prefix("unix:")
        .unwrap_or(value)
        .parse()
        .map_err(|_| Error::manifest(format!("invalid timestamp: {value}")))
}

fn required_field(table: &mut BTreeMap<String, String>, key: &str) -> Result<String> {
    table
        .remove(key)
        .ok_or_else(|| Error::manifest(format!("manifest link is missing `{key}`")))
}

fn parse_link_kind(value: &str) -> Result<LinkKind> {
    LinkKind::from_str(value).map_err(|_| Error::manifest(format!("unknown link_kind `{value}`")))
}

fn parse_backend(value: &str) -> Result<SymlinkBackend> {
    match value {
        "std" => Ok(SymlinkBackend::Std),
        "windows-broker" => Ok(SymlinkBackend::WindowsBroker),
        "windows-std-fallback" => Ok(SymlinkBackend::WindowsStdFallback),
        "external-ln" => Ok(SymlinkBackend::ExternalLn),
        "mock" => Ok(SymlinkBackend::Mock),
        other => Err(Error::manifest(format!(
            "unknown provider_backend `{other}`"
        ))),
    }
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
        path.replace('\\', "/")
    }
}

fn escape_manifest_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn unescape_manifest_string(value: &str, line_number: usize) -> Result<String> {
    let mut output = String::new();
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }

        let Some(escaped) = chars.next() else {
            return Err(Error::manifest(format!(
                "invalid escape at end of line {line_number}"
            )));
        };

        match escaped {
            '\\' => output.push('\\'),
            '"' => output.push('"'),
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            other => {
                return Err(Error::manifest(format!(
                    "unsupported escape `\\{other}` on line {line_number}"
                )));
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
