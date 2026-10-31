//! Project link/status/unlink orchestration.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[cfg(windows)]
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::{
    error::{Error, Result},
    linkable::{
        validate_optional_alias, validate_project_relative_target_dir, validate_resource_source,
        validate_skill_source, LinkableItem, LinkableItemType,
    },
    manifest::{load_manifest, manifest_path, save_manifest, LinkRecord},
    paths::{
        self, current_project_root, managed_relative_path_key, normalize_windows_path,
        resolve_managed_project_path, DbPathResolution,
    },
    store::{self, Store},
    symlink::{
        default_provider, ensure_symlink, CreateSymlinkOptions, CreateSymlinkOutcome, LinkStatus,
        RemoveSymlinkOutcome, SymlinkBackend, SymlinkError, SymlinkErrorKind, SymlinkProvider,
    },
    util::timestamp,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkItemRequest {
    pub identifier: String,
    pub link_name_override: Option<String>,
    pub target_dir_override: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkItemReport {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub summary: String,
    pub details: Vec<String>,
}

pub fn link_current_project(request: LinkItemRequest) -> Result<LinkItemReport> {
    link_current_project_with_options(request, LinkOptions::default())
}

pub fn link_current_project_with_options(
    request: LinkItemRequest,
    options: LinkOptions,
) -> Result<LinkItemReport> {
    let project_root = current_project_root()?;
    let mut provider = default_provider();
    let resolution = paths::resolve_database_path()?;
    link_project_with_provider_and_db_with_options(
        &project_root,
        provider.as_mut(),
        &resolution,
        request,
        options,
    )
}

pub fn link_project_with_provider(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    request: LinkItemRequest,
) -> Result<LinkItemReport> {
    let resolution = paths::resolve_database_path()?;
    link_project_with_provider_and_db(project_root, provider, &resolution, request)
}

pub fn link_project_with_provider_and_db(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    resolution: &DbPathResolution,
    request: LinkItemRequest,
) -> Result<LinkItemReport> {
    link_project_with_provider_and_db_with_options(
        project_root,
        provider,
        resolution,
        request,
        LinkOptions::default(),
    )
}

pub fn link_project_with_provider_and_db_with_options(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    resolution: &DbPathResolution,
    request: LinkItemRequest,
    options: LinkOptions,
) -> Result<LinkItemReport> {
    let store = Store::open_ro(resolution)?;
    link_project_with_provider_and_store(project_root, provider, &store, request, options)
}

fn link_project_with_provider_and_store(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    store: &Store,
    request: LinkItemRequest,
    options: LinkOptions,
) -> Result<LinkItemReport> {
    let item = store.find_any_item(&request.identifier)?;
    link_project_item_with_provider(
        project_root,
        provider,
        item,
        request.link_name_override.as_deref(),
        request.target_dir_override.as_deref(),
        options,
    )
}

fn link_project_item_with_provider(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    item: LinkableItem,
    link_name_override: Option<&str>,
    target_dir_override: Option<&Path>,
    options: LinkOptions,
) -> Result<LinkItemReport> {
    validate_registered_source(&item)?;

    let link_path = project_link_path_for_item(&item, link_name_override, target_dir_override)?;
    let created_dirs = prepare_link_parent(project_root, &link_path, !options.dry_run)?;

    let source_path = item.source_path.clone();
    let absolute_link_path = resolve_managed_project_path(project_root, &link_path)?;
    let mut manifest = load_manifest(project_root)?;
    let record_id = manifest_record_id(&item, &link_path);

    if let Some(existing) = manifest.links.iter().find(|link| {
        managed_relative_path_key(&link.link_path) == managed_relative_path_key(&link_path)
            && link.id != record_id
    }) {
        return Err(Error::manifest(format!(
            "project link path `{}` is already managed by `{}`",
            link_path.display(),
            existing.item_name
        )));
    }

    let symlink_options = create_symlink_options(options.force);
    let outcome = if options.dry_run {
        plan_symlink(
            provider,
            &source_path,
            &absolute_link_path,
            item.source_kind,
            symlink_options,
        )?
    } else {
        ensure_symlink(
            provider,
            &source_path,
            &absolute_link_path,
            item.source_kind,
            symlink_options,
        )?
    };

    if !options.dry_run {
        let record = link_record_from_item(&item, &link_path, provider.backend(), &record_id);
        manifest.upsert(record);
        save_manifest(project_root, &manifest)?;
    }

    Ok(LinkItemReport {
        project_root: project_root.to_path_buf(),
        manifest_path: manifest_path(project_root),
        item_name: item.name,
        item_type: item.item_type,
        source_path,
        link_path,
        outcome,
        provider_backend: provider.backend(),
        dry_run: options.dry_run,
        created_dirs,
    })
}

pub fn link_group_current_project(group: &str) -> Result<LinkGroupReport> {
    link_group_current_project_with_options(group, LinkOptions::default())
}

pub fn link_group_current_project_with_options(
    group: &str,
    options: LinkOptions,
) -> Result<LinkGroupReport> {
    let project_root = current_project_root()?;
    let mut provider = default_provider();
    let resolution = paths::resolve_database_path()?;
    link_group_project_with_provider_and_db_with_options(
        &project_root,
        provider.as_mut(),
        &resolution,
        group,
        options,
    )
}

pub fn link_group_project_with_provider_and_db(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    resolution: &DbPathResolution,
    group: &str,
) -> Result<LinkGroupReport> {
    link_group_project_with_provider_and_db_with_options(
        project_root,
        provider,
        resolution,
        group,
        LinkOptions::default(),
    )
}

pub fn link_group_project_with_provider_and_db_with_options(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    resolution: &DbPathResolution,
    group: &str,
    options: LinkOptions,
) -> Result<LinkGroupReport> {
    let store = Store::open_ro(resolution)?;
    let group = store.show_group(group)?;
    let group_name = group.name;
    let mut reports = Vec::new();
    for item in group.items {
        reports.push(link_project_item_with_provider(
            project_root,
            provider,
            item,
            None,
            None,
            options,
        )?);
    }

    Ok(LinkGroupReport {
        project_root: project_root.to_path_buf(),
        manifest_path: manifest_path(project_root),
        group_name,
        reports,
        dry_run: options.dry_run,
    })
}

pub fn status_current_project() -> Result<StatusReport> {
    let project_root = current_project_root()?;
    let provider = default_provider();
    status_project_with_provider(&project_root, provider.as_ref())
}

pub fn status_project_with_provider(
    project_root: &Path,
    provider: &dyn SymlinkProvider,
) -> Result<StatusReport> {
    let manifest = load_manifest(project_root)?;
    let mut entries = Vec::new();

    for record in manifest.links {
        let absolute_source_path = project_path(project_root, &record.source_path)?;
        let absolute_link_path = resolve_managed_project_path(project_root, &record.link_path)?;
        let status =
            provider.link_status(&absolute_source_path, &absolute_link_path, record.link_kind)?;
        entries.push(StatusEntry {
            record,
            absolute_source_path,
            absolute_link_path,
            status,
        });
    }

    Ok(StatusReport {
        project_root: project_root.to_path_buf(),
        manifest_path: manifest_path(project_root),
        entries,
    })
}

pub fn unlink_current_project(identifier: Option<String>) -> Result<UnlinkReport> {
    unlink_current_project_with_options(identifier, UnlinkOptions::default())
}

pub fn unlink_current_project_with_options(
    identifier: Option<String>,
    options: UnlinkOptions,
) -> Result<UnlinkReport> {
    let project_root = current_project_root()?;
    let mut provider = default_provider();
    unlink_project_with_provider_with_options(&project_root, provider.as_mut(), identifier, options)
}

pub fn unlink_project_with_provider(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    identifier: Option<String>,
) -> Result<UnlinkReport> {
    unlink_project_with_provider_with_options(
        project_root,
        provider,
        identifier,
        UnlinkOptions::default(),
    )
}

pub fn unlink_project_with_provider_with_options(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    identifier: Option<String>,
    options: UnlinkOptions,
) -> Result<UnlinkReport> {
    let manifest = load_manifest(project_root)?;
    let selected_indexes = select_unlink_indexes(&manifest.links, identifier.as_deref())?;
    unlink_selected_indexes(project_root, provider, manifest, &selected_indexes, options)
}

pub fn unlink_group_current_project(group: &str) -> Result<UnlinkReport> {
    unlink_group_current_project_with_options(group, UnlinkOptions::default())
}

pub fn unlink_group_current_project_with_options(
    group: &str,
    options: UnlinkOptions,
) -> Result<UnlinkReport> {
    let project_root = current_project_root()?;
    let mut provider = default_provider();
    let resolution = paths::resolve_database_path()?;
    unlink_group_project_with_provider_and_db_with_options(
        &project_root,
        provider.as_mut(),
        &resolution,
        group,
        options,
    )
}

pub fn unlink_group_project_with_provider_and_db(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    resolution: &DbPathResolution,
    group: &str,
) -> Result<UnlinkReport> {
    unlink_group_project_with_provider_and_db_with_options(
        project_root,
        provider,
        resolution,
        group,
        UnlinkOptions::default(),
    )
}

pub fn unlink_group_project_with_provider_and_db_with_options(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    resolution: &DbPathResolution,
    group: &str,
    options: UnlinkOptions,
) -> Result<UnlinkReport> {
    let store = Store::open_ro(resolution)?;
    let group = store.show_group(group)?;
    let manifest = load_manifest(project_root)?;
    let item_ids: BTreeSet<&str> = group.items.iter().map(|item| item.id.as_str()).collect();
    let selected_indexes: BTreeSet<usize> = manifest
        .links
        .iter()
        .enumerate()
        .filter_map(|(index, record)| item_ids.contains(&record.item_id.as_str()).then_some(index))
        .collect();

    unlink_selected_indexes(project_root, provider, manifest, &selected_indexes, options)
}

pub fn clean_current_project(mode: CleanMode) -> Result<CleanReport> {
    clean_current_project_with_options(CleanOptions::new(mode))
}

pub fn clean_current_project_with_options(options: CleanOptions) -> Result<CleanReport> {
    let project_root = current_project_root()?;
    let mut provider = default_provider();
    clean_project_with_provider_with_options(&project_root, provider.as_mut(), options)
}

pub fn clean_project_with_provider(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    mode: CleanMode,
) -> Result<CleanReport> {
    clean_project_with_provider_with_options(project_root, provider, CleanOptions::new(mode))
}

pub fn clean_project_with_provider_with_options(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    options: CleanOptions,
) -> Result<CleanReport> {
    let mut manifest = load_manifest(project_root)?;
    let mut selected_indexes = BTreeSet::new();
    let mut drop_only_indexes = BTreeSet::new();

    for (index, record) in manifest.links.iter().enumerate() {
        let absolute_source_path = project_path(project_root, &record.source_path)?;
        let absolute_link_path = resolve_managed_project_path(project_root, &record.link_path)?;
        let status =
            provider.link_status(&absolute_source_path, &absolute_link_path, record.link_kind)?;
        let source_missing = if matches!(options.mode, CleanMode::MissingSource) {
            source_is_missing(&absolute_source_path)?
        } else {
            false
        };

        let selected = match options.mode {
            CleanMode::Default => {
                matches!(
                    status,
                    LinkStatus::Missing | LinkStatus::BrokenSymlink { .. }
                )
            }
            CleanMode::Broken => matches!(status, LinkStatus::BrokenSymlink { .. }),
            CleanMode::MissingSource => source_missing,
        };

        if !selected {
            continue;
        }

        if matches!(status, LinkStatus::Missing) {
            drop_only_indexes.insert(index);
        } else {
            selected_indexes.insert(index);
        }
    }

    preflight_unlink(project_root, provider, &manifest.links, &selected_indexes)?;
    let mut removed = Vec::new();
    let mut dropped_missing = Vec::new();
    let mut remaining = Vec::new();

    let links = std::mem::take(&mut manifest.links);
    for (index, record) in links.into_iter().enumerate() {
        if selected_indexes.contains(&index) {
            let absolute_link_path = resolve_managed_project_path(project_root, &record.link_path)?;
            let outcome = if options.dry_run {
                planned_remove_outcome(project_root, provider, &record)?
            } else {
                provider.remove_symlink(&absolute_link_path)?
            };
            removed.push(UnlinkEntry {
                record,
                absolute_link_path,
                outcome,
            });
        } else if drop_only_indexes.contains(&index) {
            dropped_missing.push(record);
        } else {
            remaining.push(record);
        }
    }

    if !options.dry_run {
        manifest.links = remaining;
        save_manifest(project_root, &manifest)?;
    }

    Ok(CleanReport {
        project_root: project_root.to_path_buf(),
        manifest_path: manifest_path(project_root),
        removed,
        dropped_missing,
        dry_run: options.dry_run,
    })
}

pub fn doctor_current_project(verbose: bool) -> DoctorReport {
    let project_root = match current_project_root() {
        Ok(root) => root,
        Err(error) => {
            return DoctorReport {
                checks: vec![DoctorCheck {
                    name: "project-root".to_string(),
                    ok: false,
                    summary: error.to_string(),
                    details: Vec::new(),
                }],
                ok: false,
            }
        }
    };
    let resolution = match paths::resolve_database_path() {
        Ok(resolution) => resolution,
        Err(error) => {
            return DoctorReport {
                checks: vec![DoctorCheck {
                    name: "database-path".to_string(),
                    ok: false,
                    summary: error.to_string(),
                    details: Vec::new(),
                }],
                ok: false,
            };
        }
    };

    doctor_project(&project_root, &resolution, verbose)
}

pub fn doctor_project(
    project_root: &Path,
    resolution: &DbPathResolution,
    verbose: bool,
) -> DoctorReport {
    let mut checks = Vec::new();

    checks.push(match store::check_database(resolution) {
        Ok(report) => DoctorCheck {
            name: "database".to_string(),
            ok: report.is_ok(),
            summary: report.status.as_str().to_string(),
            details: verbose
                .then(|| {
                    let mut details = vec![
                        format!("path={}", report.path.display()),
                        format!("reason={}", report.reason.as_str()),
                        format!("exists={}", report.exists),
                        format!("status={}", report.status.as_str()),
                        format!(
                            "schema={}/{}",
                            report.schema_version.map_or_else(
                                || "missing".to_string(),
                                |version| version.to_string()
                            ),
                            report.latest_schema_version
                        ),
                    ];
                    details.extend(report.details.clone());
                    details
                })
                .unwrap_or_default(),
        },
        Err(error) => DoctorCheck {
            name: "database".to_string(),
            ok: false,
            summary: error.to_string(),
            details: Vec::new(),
        },
    });

    checks.push(match load_manifest(project_root) {
        Ok(manifest) => DoctorCheck {
            name: "manifest".to_string(),
            ok: true,
            summary: format!("{} managed links", manifest.links.len()),
            details: verbose
                .then(|| vec![format!("path={}", manifest_path(project_root).display())])
                .unwrap_or_default(),
        },
        Err(error) => DoctorCheck {
            name: "manifest".to_string(),
            ok: false,
            summary: error.to_string(),
            details: verbose
                .then(|| vec![format!("path={}", manifest_path(project_root).display())])
                .unwrap_or_default(),
        },
    });

    checks.push(
        match Store::open_ro(resolution).and_then(|store| store.list_harnesses()) {
            Ok(harnesses) => {
                let mapping_count: usize =
                    harnesses.iter().map(|harness| harness.mappings.len()).sum();
                DoctorCheck {
                    name: "harness-adapter".to_string(),
                    ok: true,
                    summary: format!("{} harnesses, {} mappings", harnesses.len(), mapping_count),
                    details: verbose
                        .then(|| {
                            harnesses
                                .iter()
                                .map(|harness| {
                                    format!(
                                        "{} enabled={} mappings={}",
                                        harness.name,
                                        harness.enabled,
                                        harness.mappings.len()
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                }
            }
            Err(error) => DoctorCheck {
                name: "harness-adapter".to_string(),
                ok: false,
                summary: error.to_string(),
                details: Vec::new(),
            },
        },
    );

    let provider = default_provider();
    checks.push(DoctorCheck {
        name: "symlink-backend".to_string(),
        ok: true,
        summary: provider.backend().to_string(),
        details: verbose
            .then(|| vec![format!("backend={}", provider.backend())])
            .unwrap_or_default(),
    });
    drop(provider);

    checks.push(check_broker_probe(verbose));

    let ok = checks.iter().all(|check| check.ok);
    DoctorReport { checks, ok }
}

fn check_broker_probe(verbose: bool) -> DoctorCheck {
    #[cfg(windows)]
    {
        let mut provider = default_provider();
        if provider.backend() != SymlinkBackend::WindowsBroker {
            return DoctorCheck {
                name: "windows-broker".to_string(),
                ok: false,
                summary: format!("unexpected backend {}", provider.backend()),
                details: Vec::new(),
            };
        }

        let probe_root = std::env::temp_dir().join(format!(
            "agent-linker-broker-probe-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        let source = probe_root.join("source.txt");
        let link = probe_root.join("link.txt");

        let probe = (|| -> Result<()> {
            fs::create_dir_all(&probe_root)?;
            fs::write(&source, "probe")?;
            provider.create_symlink(&source, &link, crate::core::symlink::LinkKind::File)?;
            provider.remove_symlink(&link)?;
            Ok(())
        })();
        let _ = fs::remove_dir_all(&probe_root);

        match probe {
            Ok(()) => DoctorCheck {
                name: "windows-broker".to_string(),
                ok: true,
                summary: "available".to_string(),
                details: verbose
                    .then(|| {
                        vec![
                            "backend=windows-broker".to_string(),
                            format!("probe_root={}", probe_root.display()),
                        ]
                    })
                    .unwrap_or_default(),
            },
            Err(Error::Symlink(error)) => DoctorCheck {
                name: "windows-broker".to_string(),
                ok: false,
                summary: error.kind().to_string(),
                details: verbose
                    .then(|| {
                        let mut details = vec![format!("backend={}", error.backend())];
                        if let Some(system_code) = error.system_code() {
                            details.push(format!("system_code={system_code}"));
                        }
                        if let Some(broker_code) = error.broker_code() {
                            details.push(format!("broker_code={broker_code}"));
                        }
                        if let Some(detail) = error.detail() {
                            details.push(format!("detail={detail}"));
                        }
                        details
                    })
                    .unwrap_or_default(),
            },
            Err(error) => DoctorCheck {
                name: "windows-broker".to_string(),
                ok: false,
                summary: error.to_string(),
                details: Vec::new(),
            },
        }
    }

    #[cfg(not(windows))]
    {
        let _ = verbose;
        DoctorCheck {
            name: "windows-broker".to_string(),
            ok: true,
            summary: "not applicable".to_string(),
            details: Vec::new(),
        }
    }
}

fn unlink_selected_indexes(
    project_root: &Path,
    provider: &mut dyn SymlinkProvider,
    mut manifest: crate::core::manifest::Manifest,
    selected_indexes: &BTreeSet<usize>,
    options: UnlinkOptions,
) -> Result<UnlinkReport> {
    preflight_unlink(project_root, provider, &manifest.links, selected_indexes)?;
    let mut outcomes = Vec::new();
    let mut remaining = Vec::new();

    let links = std::mem::take(&mut manifest.links);
    for (index, record) in links.into_iter().enumerate() {
        if selected_indexes.contains(&index) {
            let absolute_link_path = resolve_managed_project_path(project_root, &record.link_path)?;
            let outcome = if options.dry_run {
                planned_remove_outcome(project_root, provider, &record)?
            } else {
                provider.remove_symlink(&absolute_link_path)?
            };
            outcomes.push(UnlinkEntry {
                record,
                absolute_link_path,
                outcome,
            });
        } else {
            remaining.push(record);
        }
    }

    if !options.dry_run {
        manifest.links = remaining;
        save_manifest(project_root, &manifest)?;
    }

    Ok(UnlinkReport {
        project_root: project_root.to_path_buf(),
        manifest_path: manifest_path(project_root),
        outcomes,
        dry_run: options.dry_run,
    })
}

fn preflight_unlink(
    project_root: &Path,
    provider: &dyn SymlinkProvider,
    records: &[LinkRecord],
    selected_indexes: &BTreeSet<usize>,
) -> Result<()> {
    for index in selected_indexes {
        let record = &records[*index];
        let absolute_source_path = project_path(project_root, &record.source_path)?;
        let absolute_link_path = resolve_managed_project_path(project_root, &record.link_path)?;
        match provider.link_status(&absolute_source_path, &absolute_link_path, record.link_kind)? {
            LinkStatus::ExistingRealFile => {
                return Err(SymlinkError::new(
                    SymlinkErrorKind::ExistingRealFile,
                    provider.backend(),
                )
                .with_source(absolute_source_path)
                .with_link(absolute_link_path)
                .into());
            }
            LinkStatus::ExistingRealDirectory => {
                return Err(SymlinkError::new(
                    SymlinkErrorKind::ExistingRealDirectory,
                    provider.backend(),
                )
                .with_source(absolute_source_path)
                .with_link(absolute_link_path)
                .into());
            }
            LinkStatus::UnsupportedFileType { path } => {
                return Err(SymlinkError::new(
                    SymlinkErrorKind::UnsupportedLinkKind,
                    provider.backend(),
                )
                .with_source(absolute_source_path)
                .with_link(absolute_link_path)
                .with_detail(format!("unsupported file type at {}", path.display()))
                .into());
            }
            LinkStatus::Missing
            | LinkStatus::CorrectSymlink { .. }
            | LinkStatus::WrongSymlinkTarget { .. }
            | LinkStatus::BrokenSymlink { .. } => {}
        }
    }

    Ok(())
}

fn planned_remove_outcome(
    project_root: &Path,
    provider: &dyn SymlinkProvider,
    record: &LinkRecord,
) -> Result<RemoveSymlinkOutcome> {
    let absolute_source_path = project_path(project_root, &record.source_path)?;
    let absolute_link_path = resolve_managed_project_path(project_root, &record.link_path)?;
    match provider.link_status(&absolute_source_path, &absolute_link_path, record.link_kind)? {
        LinkStatus::Missing => Ok(RemoveSymlinkOutcome::Missing),
        LinkStatus::CorrectSymlink { .. }
        | LinkStatus::WrongSymlinkTarget { .. }
        | LinkStatus::BrokenSymlink { .. } => Ok(RemoveSymlinkOutcome::Removed),
        LinkStatus::ExistingRealFile => Err(SymlinkError::new(
            SymlinkErrorKind::ExistingRealFile,
            provider.backend(),
        )
        .with_source(absolute_source_path)
        .with_link(absolute_link_path)
        .into()),
        LinkStatus::ExistingRealDirectory => Err(SymlinkError::new(
            SymlinkErrorKind::ExistingRealDirectory,
            provider.backend(),
        )
        .with_source(absolute_source_path)
        .with_link(absolute_link_path)
        .into()),
        LinkStatus::UnsupportedFileType { path } => Err(SymlinkError::new(
            SymlinkErrorKind::UnsupportedLinkKind,
            provider.backend(),
        )
        .with_source(absolute_source_path)
        .with_link(absolute_link_path)
        .with_detail(format!("unsupported file type at {}", path.display()))
        .into()),
    }
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

fn plan_symlink(
    provider: &dyn SymlinkProvider,
    source: &Path,
    link: &Path,
    kind: crate::core::symlink::LinkKind,
    options: CreateSymlinkOptions,
) -> Result<CreateSymlinkOutcome> {
    match provider.link_status(source, link, kind)? {
        LinkStatus::Missing => Ok(CreateSymlinkOutcome::Created),
        LinkStatus::CorrectSymlink { .. } => Ok(CreateSymlinkOutcome::AlreadyCorrect),
        LinkStatus::WrongSymlinkTarget { actual, expected } => {
            if options.force {
                Ok(CreateSymlinkOutcome::ReplacedWrongSymlink)
            } else {
                Err(
                    SymlinkError::new(SymlinkErrorKind::WrongSymlinkTarget, provider.backend())
                        .with_source(expected)
                        .with_link(link)
                        .with_detail(format!("existing link points to {}", actual.display()))
                        .into(),
                )
            }
        }
        LinkStatus::BrokenSymlink { target } => Err(SymlinkError::new(
            SymlinkErrorKind::SourceNotFound,
            provider.backend(),
        )
        .with_source(source)
        .with_link(link)
        .with_detail(format!(
            "existing symlink points to missing source {}",
            target.display()
        ))
        .into()),
        LinkStatus::ExistingRealFile => Err(SymlinkError::new(
            SymlinkErrorKind::ExistingRealFile,
            provider.backend(),
        )
        .with_source(source)
        .with_link(link)
        .into()),
        LinkStatus::ExistingRealDirectory => Err(SymlinkError::new(
            SymlinkErrorKind::ExistingRealDirectory,
            provider.backend(),
        )
        .with_source(source)
        .with_link(link)
        .into()),
        LinkStatus::UnsupportedFileType { path } => Err(SymlinkError::new(
            SymlinkErrorKind::UnsupportedLinkKind,
            provider.backend(),
        )
        .with_source(source)
        .with_link(link)
        .with_detail(format!("unsupported file type at {}", path.display()))
        .into()),
    }
}

fn create_symlink_options(force: bool) -> CreateSymlinkOptions {
    if force {
        CreateSymlinkOptions::force_wrong_symlink()
    } else {
        CreateSymlinkOptions::new()
    }
}

fn prepare_link_parent(
    project_root: &Path,
    relative_link_path: &Path,
    create: bool,
) -> Result<Vec<PathBuf>> {
    resolve_managed_project_path(project_root, relative_link_path)?;

    let Some(parent) = relative_link_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    else {
        return Ok(Vec::new());
    };

    let mut created_dirs = Vec::new();
    let mut current_relative = PathBuf::new();
    for component in parent
        .components()
        .filter(|component| *component != std::path::Component::CurDir)
    {
        current_relative.push(component.as_os_str());
        let current = resolve_managed_project_path(project_root, &current_relative)?;
        match fs::symlink_metadata(&current) {
            Ok(_) => {} // Already checked as a parent of the full link path above.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                created_dirs.push(current_relative.clone());
                if create {
                    fs::create_dir(&current)?;
                }
            }
            Err(error) => return Err(error.into()),
        }
    }

    Ok(created_dirs)
}

fn source_is_missing(path: &Path) -> Result<bool> {
    match fs::metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(Error::project(format!(
            "cannot determine source status for {}: {error}",
            path.display()
        ))),
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
        scope: "project".to_string(),
        harness_name: "registry".to_string(),
        item_id: item.id.clone(),
        item_name: item.name.clone(),
        source_path: item.source_path.clone(),
        link_path: link_path.to_path_buf(),
        link_kind: item.source_kind,
        provider_backend: backend,
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
        return Ok((0..records.len()).collect());
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
