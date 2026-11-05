//! Plan, validate and execute managed project link changes.
use crate::core::{
    error::{Error, Result},
    harness::HarnessMapping,
    linkable::{
        validate_optional_alias, validate_project_relative_target_dir, validate_resource_source,
        validate_skill_source, LinkableItem, LinkableItemType,
    },
    manifest::{
        load_manifest, manifest_path, record_from_mapping, save_manifest, InitLinkOutcome,
        LinkOrigin, LinkRecord, Manifest,
    },
    paths::{managed_relative_path_key, normalize_windows_path, resolve_managed_project_path},
    store::Store,
    symlink::{
        evaluate_create, evaluate_remove, CreateSymlinkOptions, CreateSymlinkOutcome, LinkKind,
        LinkStatus, RemoveSymlinkOutcome, SymlinkBackend, SymlinkProvider,
    },
    util::timestamp,
};
use std::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkItemRequest {
    pub identifier: String,
    pub link_name_override: Option<String>,
    pub target_dir_override: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkItemReport {
    pub id: String,
    pub project_root: PathBuf,
    pub manifest_path: PathBuf,
    pub item_name: String,
    pub item_type: LinkableItemType,
    pub source_path: PathBuf,
    pub link_path: PathBuf,
    pub outcome: CreateSymlinkOutcome,
    pub provider_backend: SymlinkBackend,
    pub dry_run: bool,
    pub created_dirs: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusReport {
    pub project_root: PathBuf,
    pub manifest_path: PathBuf,
    pub entries: Vec<StatusEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    pub record: LinkRecord,
    pub absolute_source_path: PathBuf,
    pub absolute_link_path: PathBuf,
    pub status: LinkStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlinkReport {
    pub project_root: PathBuf,
    pub manifest_path: PathBuf,
    pub outcomes: Vec<UnlinkEntry>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlinkEntry {
    pub record: LinkRecord,
    pub absolute_link_path: PathBuf,
    pub outcome: RemoveSymlinkOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkGroupReport {
    pub project_root: PathBuf,
    pub manifest_path: PathBuf,
    pub group_name: String,
    pub reports: Vec<LinkItemReport>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanMode {
    Default,
    Broken,
    MissingSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LinkOptions {
    pub dry_run: bool,
    pub force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnlinkOptions {
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanOptions {
    pub mode: CleanMode,
    pub dry_run: bool,
}

impl CleanOptions {
    pub const fn new(mode: CleanMode) -> Self {
        Self {
            mode,
            dry_run: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanReport {
    pub project_root: PathBuf,
    pub manifest_path: PathBuf,
    pub removed: Vec<UnlinkEntry>,
    pub dropped_missing: Vec<LinkRecord>,
    pub dry_run: bool,
}

pub struct ProjectCtx<'a> {
    pub root: &'a Path,
    pub provider: &'a mut dyn SymlinkProvider,
}

struct LinkPlan {
    manifest: Manifest,
    dirs: BTreeMap<PathBuf, PathBuf>,
    changes: Vec<PlannedChange>,
}

enum PlannedChange {
    Create {
        source: PathBuf,
        link: PathBuf,
        kind: LinkKind,
        outcome: CreateSymlinkOutcome,
    },
    Remove {
        link: PathBuf,
        outcome: RemoveSymlinkOutcome,
    },
}

impl LinkPlan {
    fn new(root: &Path) -> Result<Self> {
        Ok(Self {
            manifest: load_manifest(root)?,
            dirs: BTreeMap::new(),
            changes: Vec::new(),
        })
    }

    fn execute(&mut self, ctx: &mut ProjectCtx, dry_run: bool) -> Result<()> {
        self.manifest.validate()?;
        if dry_run {
            return Ok(());
        }
        for dir in self.dirs.values() {
            let path = resolve_managed_project_path(ctx.root, dir)?;
            fs::create_dir(&path).map_err(|error| Error::io(&path, error))?;
        }
        for change in &mut self.changes {
            match change {
                PlannedChange::Create {
                    source,
                    link,
                    kind,
                    outcome,
                } => match outcome {
                    CreateSymlinkOutcome::Created => {
                        ctx.provider.create_symlink(source, link, *kind)?
                    }
                    CreateSymlinkOutcome::ReplacedWrongSymlink => {
                        ctx.provider.replace_symlink(source, link, *kind)?
                    }
                    CreateSymlinkOutcome::AlreadyCorrect => {}
                },
                PlannedChange::Remove { link, outcome } => {
                    *outcome = ctx.provider.remove_symlink(link)?;
                }
            }
        }
        save_manifest(ctx.root, &self.manifest)
    }

    fn add_item(
        &mut self,
        ctx: &ProjectCtx,
        item: LinkableItem,
        link_name: Option<&str>,
        target_dir: Option<&Path>,
        opts: LinkOptions,
        command: &str,
    ) -> Result<LinkItemReport> {
        validate_registered_source(&item)?;
        let link_path: PathBuf = project_link_path_for_item(&item, link_name, target_dir)?
            .components()
            .filter(|part| *part != std::path::Component::CurDir)
            .collect();
        let mut record = link_record_from_item(
            &item,
            &link_path,
            ctx.provider.backend(),
            &manifest_record_id(&item, &link_path),
        );
        record.created_by_command = command.to_string();
        let (id, outcome, created_dirs) = self.add_link(ctx, record, opts)?;
        Ok(LinkItemReport {
            id,
            project_root: ctx.root.to_path_buf(),
            manifest_path: manifest_path(ctx.root),
            item_name: item.name,
            item_type: item.item_type,
            source_path: item.source_path,
            link_path,
            outcome,
            provider_backend: ctx.provider.backend(),
            dry_run: opts.dry_run,
            created_dirs,
        })
    }

    fn add_link(
        &mut self,
        ctx: &ProjectCtx,
        mut record: LinkRecord,
        opts: LinkOptions,
    ) -> Result<(String, CreateSymlinkOutcome, Vec<PathBuf>)> {
        let link_path = &record.link_path;
        let key = managed_relative_path_key(link_path);
        for existing in &self.manifest.links {
            let existing_key = managed_relative_path_key(&existing.link_path);
            if existing.origin == record.origin
                && existing.item_id == record.item_id
                && existing_key == key
            {
                record.id = existing.id.clone();
                continue;
            }
            if key == existing_key
                || key.starts_with(&existing_key)
                || existing_key.starts_with(&key)
            {
                return Err(Error::conflict(format!(
                    "project link path '{}' conflicts with managed '{}'",
                    link_path.display(),
                    existing.link_path.display()
                )));
            }
        }
        let source = project_path(ctx.root, &record.source_path)?;
        ctx.provider.validate_source(&source, record.link_kind)?;
        let absolute_link = resolve_managed_project_path(ctx.root, link_path)?;
        let dirs = missing_parent_dirs(ctx.root, link_path)?;
        let created_dirs = dirs
            .into_iter()
            .filter(
                |dir| match self.dirs.entry(managed_relative_path_key(dir)) {
                    Entry::Vacant(entry) => {
                        entry.insert(dir.clone());
                        true
                    }
                    Entry::Occupied(_) => false,
                },
            )
            .collect();
        let state = ctx
            .provider
            .link_status(&source, &absolute_link, record.link_kind)?;
        let outcome = evaluate_create(
            &state,
            true,
            &source,
            &absolute_link,
            ctx.provider.backend(),
            CreateSymlinkOptions { force: opts.force },
        )?;
        self.changes.push(PlannedChange::Create {
            source,
            link: absolute_link,
            kind: record.link_kind,
            outcome,
        });
        if outcome == CreateSymlinkOutcome::AlreadyCorrect {
            if let Some(existing) = self
                .manifest
                .links
                .iter()
                .find(|entry| entry.id == record.id)
            {
                record.created_at = existing.created_at;
                record.updated_at = existing.updated_at;
                if record != *existing {
                    record.updated_at = timestamp();
                }
            }
        }
        let id = record.id.clone();
        self.manifest.upsert(record);
        Ok((id, outcome, created_dirs))
    }

    fn remove_selected(
        &mut self,
        ctx: &ProjectCtx,
        selected: &BTreeSet<usize>,
    ) -> Result<Vec<UnlinkEntry>> {
        let mut outcomes = Vec::new();
        let mut remaining = Vec::new();
        for (index, record) in self.manifest.links.iter().cloned().enumerate() {
            if !selected.contains(&index) {
                remaining.push(record);
                continue;
            }
            let source = project_path(ctx.root, &record.source_path)?;
            let link = resolve_managed_project_path(ctx.root, &record.link_path)?;
            let state = ctx.provider.link_status(&source, &link, record.link_kind)?;
            let outcome = evaluate_remove(&state, &link, ctx.provider.backend())?;
            self.changes.push(PlannedChange::Remove {
                link: link.clone(),
                outcome,
            });
            outcomes.push(UnlinkEntry {
                record,
                absolute_link_path: link,
                outcome,
            });
        }
        self.manifest.links = remaining;
        Ok(outcomes)
    }
}

pub fn link_item(
    ctx: &mut ProjectCtx,
    store: &Store,
    req: &LinkItemRequest,
    opts: LinkOptions,
) -> Result<LinkItemReport> {
    let item = store.find_any_item(&req.identifier)?;
    let mut plan = LinkPlan::new(ctx.root)?;
    let report = plan.add_item(
        ctx,
        item,
        req.link_name_override.as_deref(),
        req.target_dir_override.as_deref(),
        opts,
        "link",
    )?;
    plan.execute(ctx, opts.dry_run)?;
    Ok(report)
}

pub fn link_group(
    ctx: &mut ProjectCtx,
    store: &Store,
    group: &str,
    opts: LinkOptions,
) -> Result<LinkGroupReport> {
    let group = store.show_group(group)?;
    let reports = link_items(ctx, group.items, opts, "link")?;
    Ok(LinkGroupReport {
        project_root: ctx.root.to_path_buf(),
        manifest_path: manifest_path(ctx.root),
        group_name: group.name,
        reports,
        dry_run: opts.dry_run,
    })
}

pub(super) fn link_items(
    ctx: &mut ProjectCtx,
    items: Vec<LinkableItem>,
    opts: LinkOptions,
    command: &str,
) -> Result<Vec<LinkItemReport>> {
    let mut plan = LinkPlan::new(ctx.root)?;
    let mut reports = Vec::new();
    for item in items {
        reports.push(plan.add_item(ctx, item, None, None, opts, command)?);
    }
    plan.execute(ctx, opts.dry_run)?;
    Ok(reports)
}

pub(super) fn link_harnesses(
    ctx: &mut ProjectCtx,
    mappings: &[HarnessMapping],
) -> Result<(Vec<InitLinkOutcome>, Vec<PathBuf>)> {
    let mut plan = LinkPlan::new(ctx.root)?;
    let mut reports = Vec::new();
    let mut created_dirs = Vec::new();
    for mapping in mappings {
        mapping.source_in(ctx.root)?;
        let record = record_from_mapping(mapping, ctx.provider.backend());
        let (id, outcome, dirs) = plan.add_link(ctx, record, LinkOptions::default())?;
        created_dirs.extend(dirs);
        reports.push(InitLinkOutcome {
            id,
            source_path: mapping.source_path.clone(),
            link_path: mapping.link_path.clone(),
            outcome,
        });
    }
    plan.execute(ctx, false)?;
    Ok((reports, created_dirs))
}

pub fn status(ctx: &mut ProjectCtx) -> Result<StatusReport> {
    let manifest = load_manifest(ctx.root)?;
    let mut entries = Vec::new();
    for record in manifest.links {
        let absolute_source_path = project_path(ctx.root, &record.source_path)?;
        let absolute_link_path = resolve_managed_project_path(ctx.root, &record.link_path)?;
        let status = ctx.provider.link_status(
            &absolute_source_path,
            &absolute_link_path,
            record.link_kind,
        )?;
        entries.push(StatusEntry {
            record,
            absolute_source_path,
            absolute_link_path,
            status,
        });
    }
    Ok(StatusReport {
        project_root: ctx.root.to_path_buf(),
        manifest_path: manifest_path(ctx.root),
        entries,
    })
}

pub fn unlink(
    ctx: &mut ProjectCtx,
    identifier: Option<&str>,
    opts: UnlinkOptions,
) -> Result<UnlinkReport> {
    let plan = LinkPlan::new(ctx.root)?;
    let selected = select_unlink_indexes(&plan.manifest.links, identifier)?;
    execute_unlink(ctx, plan, &selected, opts)
}

pub fn unlink_group(
    ctx: &mut ProjectCtx,
    store: &Store,
    group: &str,
    opts: UnlinkOptions,
) -> Result<UnlinkReport> {
    let group = store.show_group(group)?;
    let plan = LinkPlan::new(ctx.root)?;
    let ids: BTreeSet<_> = group.items.iter().map(|item| &item.id).collect();
    let selected = plan
        .manifest
        .links
        .iter()
        .enumerate()
        .filter_map(|(i, record)| {
            (record.origin == LinkOrigin::Item && ids.contains(&record.item_id)).then_some(i)
        })
        .collect();
    execute_unlink(ctx, plan, &selected, opts)
}

fn execute_unlink(
    ctx: &mut ProjectCtx,
    mut plan: LinkPlan,
    selected: &BTreeSet<usize>,
    opts: UnlinkOptions,
) -> Result<UnlinkReport> {
    let mut outcomes = plan.remove_selected(ctx, selected)?;
    plan.execute(ctx, opts.dry_run)?;
    for (entry, change) in outcomes.iter_mut().zip(&plan.changes) {
        if let PlannedChange::Remove { outcome, .. } = change {
            entry.outcome = *outcome;
        }
    }
    Ok(UnlinkReport {
        project_root: ctx.root.to_path_buf(),
        manifest_path: manifest_path(ctx.root),
        outcomes,
        dry_run: opts.dry_run,
    })
}

pub fn clean(ctx: &mut ProjectCtx, opts: CleanOptions) -> Result<CleanReport> {
    let mut plan = LinkPlan::new(ctx.root)?;
    let mut dropped_missing = Vec::new();
    let mut removed = Vec::new();
    let mut remaining = Vec::new();
    for record in plan.manifest.links.iter().cloned() {
        let source = project_path(ctx.root, &record.source_path)?;
        let link = resolve_managed_project_path(ctx.root, &record.link_path)?;
        let state = ctx.provider.link_status(&source, &link, record.link_kind)?;
        let matches = match opts.mode {
            CleanMode::Default => matches!(
                state,
                LinkStatus::Missing | LinkStatus::BrokenSymlink { .. }
            ),
            CleanMode::Broken => matches!(state, LinkStatus::BrokenSymlink { .. }),
            CleanMode::MissingSource => source_is_missing(&source)?,
        };
        if !matches {
            remaining.push(record);
            continue;
        }
        let outcome = evaluate_remove(&state, &link, ctx.provider.backend())?;
        if outcome == RemoveSymlinkOutcome::Missing {
            dropped_missing.push(record);
        } else {
            plan.changes.push(PlannedChange::Remove {
                link: link.clone(),
                outcome,
            });
            removed.push(UnlinkEntry {
                record,
                absolute_link_path: link,
                outcome,
            });
        }
    }
    plan.manifest.links = remaining;
    plan.execute(ctx, opts.dry_run)?;
    for (entry, change) in removed.iter_mut().zip(&plan.changes) {
        if let PlannedChange::Remove { outcome, .. } = change {
            entry.outcome = *outcome;
        }
    }
    Ok(CleanReport {
        project_root: ctx.root.to_path_buf(),
        manifest_path: manifest_path(ctx.root),
        removed,
        dropped_missing,
        dry_run: opts.dry_run,
    })
}

fn missing_parent_dirs(root: &Path, link: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    let mut relative = PathBuf::new();
    if let Some(parent) = link.parent() {
        for part in parent
            .components()
            .filter(|part| *part != std::path::Component::CurDir)
        {
            relative.push(part.as_os_str());
            let absolute = resolve_managed_project_path(root, &relative)?;
            match fs::symlink_metadata(&absolute) {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    dirs.push(relative.clone())
                }
                Err(error) => return Err(Error::io(&absolute, error)),
            }
        }
    }
    Ok(dirs)
}

fn validate_registered_source(item: &LinkableItem) -> Result<()> {
    let source = match item.item_type {
        LinkableItemType::Skill => validate_skill_source(&item.source_path)?,
        LinkableItemType::Resource => validate_resource_source(&item.source_path)?,
    };

    if source.source_kind != item.source_kind {
        return Err(Error::invalid_arguments(format!(
            "{} `{}` source kind changed from {} to {}",
            item.item_type, item.name, item.source_kind, source.source_kind
        )));
    }

    Ok(())
}

fn project_link_path_for_item(
    item: &LinkableItem,
    link_name_override: Option<&str>,
    target_dir_override: Option<&Path>,
) -> Result<PathBuf> {
    validate_optional_alias(link_name_override)?;

    let link_name = link_name_override
        .map(str::to_string)
        .unwrap_or_else(|| item.link_name());

    match item.item_type {
        LinkableItemType::Skill => {
            if target_dir_override.is_some() {
                return Err(Error::invalid_arguments(
                    "aglink link --target-dir is only valid for resources",
                ));
            }
            Ok(PathBuf::from(".agents").join("skills").join(link_name))
        }
        LinkableItemType::Resource => {
            let target_dir = match target_dir_override {
                Some(target_dir) => {
                    validate_project_relative_target_dir(target_dir)?;
                    target_dir.to_path_buf()
                }
                None => item.default_target_dir.clone().ok_or_else(|| {
                    Error::invalid_arguments(format!(
                        "resource `{}` does not have a default target directory; pass --target-dir <dir>",
                        item.name
                    ))
                })?,
            };
            Ok(target_dir.join(link_name))
        }
    }
}

fn source_is_missing(path: &Path) -> Result<bool> {
    match fs::metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(Error::io(path, error)),
    }
}

fn link_record_from_item(
    item: &LinkableItem,
    link_path: &Path,
    backend: SymlinkBackend,
    record_id: &str,
) -> LinkRecord {
    let now = timestamp();
    LinkRecord {
        id: record_id.to_string(),
        origin: LinkOrigin::Item,
        harness_name: None,
        item_id: item.id.clone(),
        item_name: item.name.clone(),
        source_path: item.source_path.clone(),
        link_path: link_path.to_path_buf(),
        link_kind: item.source_kind,
        provider_backend: backend.to_string(),
        created_by_command: "link".to_string(),
        created_at: now.clone(),
        updated_at: now,
    }
}

fn manifest_record_id(item: &LinkableItem, link_path: &Path) -> String {
    format!(
        "link:{}:{}",
        item.id,
        link_path.to_string_lossy().replace('\\', "/")
    )
}

fn select_unlink_indexes(
    records: &[LinkRecord],
    identifier: Option<&str>,
) -> Result<BTreeSet<usize>> {
    let Some(identifier) = identifier else {
        return Ok(records
            .iter()
            .enumerate()
            .filter_map(|(index, record)| (record.origin == LinkOrigin::Item).then_some(index))
            .collect());
    };

    let matches: Vec<usize> = records
        .iter()
        .enumerate()
        .filter_map(|(index, record)| {
            if record.id == identifier
                || record.item_id == identifier
                || record.item_name == identifier
                || record.link_path == Path::new(identifier)
                || record.link_path.file_name().and_then(|name| name.to_str()) == Some(identifier)
            {
                Some(index)
            } else {
                None
            }
        })
        .collect();

    match matches.len() {
        0 => Err(Error::manifest(format!(
            "no managed project link matches `{identifier}`"
        ))),
        1 => Ok(matches.into_iter().collect()),
        _ => Err(Error::invalid_arguments(format!(
            "managed project link `{identifier}` is ambiguous; use a manifest id or link path"
        ))),
    }
}

fn project_path(project_root: &Path, path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(normalize_windows_path(path))
    } else {
        resolve_managed_project_path(project_root, path)
    }
}

#[cfg(test)]
#[path = "project_links_tests.rs"]
mod tests;
