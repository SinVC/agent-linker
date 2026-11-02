use std::path::{Path, PathBuf};

use crate::core::{
    doctor::{doctor_project, DoctorCheck, DoctorReport},
    error::{Error, Result},
    harness::{AddHarnessMapping, StoredHarness, StoredHarnessMapping},
    linkable::{LinkableItem, LinkableItemType},
    manifest::{init_project_with_store, InitProjectReport},
    paths::{self, DbPathResolution},
    project_links::{
        self, CleanOptions, CleanReport, LinkGroupReport, LinkItemReport, LinkItemRequest,
        LinkOptions, ProjectCtx, StatusReport, UnlinkOptions, UnlinkReport,
    },
    registry::{AddLinkableItem, Group, GroupSummary, GroupUpdateReport},
    store::{self, DbBackupReport, DbCheckReport, MigrationReport, Store},
    symlink::{default_provider, LinkKind},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Init,
    Db(DbCommand),
    Harness(HarnessCommand),
    Skill(LinkableCommand),
    Resource(LinkableCommand),
    Group(GroupCommand),
    Link(LinkCommand),
    Unlink(UnlinkCommand),
    Status(StatusCommand),
    Clean(CleanCommand),
    Doctor(DoctorCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbCommand {
    Path,
    Migrate,
    Backup { path: Option<PathBuf> },
    Check,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessCommand {
    List,
    Show {
        name: String,
    },
    Enable {
        name: String,
    },
    Disable {
        name: String,
    },
    MappingList {
        name: Option<String>,
    },
    MappingAdd {
        name: String,
        source: PathBuf,
        link: PathBuf,
        kind: LinkKind,
    },
    MappingRemove {
        name: String,
        link: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkableCommand {
    Add {
        path: PathBuf,
        name: Option<String>,
        alias: Option<String>,
        target_dir: Option<PathBuf>,
    },
    List,
    Show {
        name: String,
    },
    Rename {
        old: String,
        new: String,
    },
    Remove {
        name: String,
    },
    Refresh {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupCommand {
    Create { name: String },
    List,
    Show { name: String },
    Rename { old: String, new: String },
    Delete { name: String },
    Add { group: String, items: Vec<String> },
    Remove { group: String, items: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkCommand {
    Item {
        request: LinkItemRequest,
        options: LinkOptions,
    },
    Group {
        group: String,
        options: LinkOptions,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnlinkTarget {
    Item(Option<String>),
    Group(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlinkCommand {
    pub target: UnlinkTarget,
    pub options: UnlinkOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCommand {
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanCommand {
    pub options: CleanOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCommand {
    pub verbose: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandReport {
    Init(InitProjectReport),
    DbPath(DbPathResolution),
    DbMigrate(MigrationReport),
    DbBackup(DbBackupReport),
    DbCheck(DbCheckReport),
    HarnessList(Vec<StoredHarness>),
    HarnessShow(StoredHarness),
    HarnessEnabled {
        name: String,
        enabled: bool,
    },
    HarnessMappingList(Vec<StoredHarnessMapping>),
    HarnessMappingAdded(StoredHarnessMapping),
    HarnessMappingRemoved(StoredHarnessMapping),
    LinkableAdded(LinkableItem),
    LinkableList {
        item_type: LinkableItemType,
        items: Vec<LinkableItem>,
    },
    LinkableShow(LinkableItem),
    LinkableRenamed(LinkableItem),
    LinkableRemoved(LinkableItem),
    LinkableRefreshed(LinkableItem),
    GroupCreated(Group),
    GroupList(Vec<GroupSummary>),
    GroupShow(Group),
    GroupRenamed(Group),
    GroupDeleted(Group),
    GroupUpdated(GroupUpdateReport),
    LinkItem(LinkItemReport),
    LinkGroup(LinkGroupReport),
    Unlink(UnlinkReport),
    Status {
        report: StatusReport,
        json: bool,
    },
    Clean(CleanReport),
    Doctor(DoctorReport),
}

impl CommandReport {
    pub fn is_success(&self) -> bool {
        match self {
            CommandReport::Doctor(report) => report.ok,
            CommandReport::DbCheck(report) => report.is_ok(),
            _ => true,
        }
    }
}

pub fn run(command: Command) -> Result<CommandReport> {
    match command {
        Command::Init => {
            let root = std::env::current_dir()?;
            let mut provider = default_provider();
            let resolution = paths::resolve_database_path()?;
            Ok(CommandReport::Init(init_project_with_store(
                &root,
                provider.as_mut(),
                &resolution,
            )?))
        }
        Command::Db(command) => run_db(command),
        Command::Harness(command) => run_harness(command),
        Command::Skill(command) => run_linkable(command, LinkableItemType::Skill),
        Command::Resource(command) => run_linkable(command, LinkableItemType::Resource),
        Command::Group(command) => run_group(command),
        Command::Link(command) => run_link(command),
        Command::Unlink(command) => run_unlink(command),
        Command::Status(command) => {
            let root = paths::current_project_root()?;
            let mut provider = default_provider();
            let report = project_links::status(&mut ProjectCtx {
                root: &root,
                provider: provider.as_mut(),
            })?;
            Ok(CommandReport::Status {
                report,
                json: command.json,
            })
        }
        Command::Clean(command) => {
            let root = paths::current_project_root()?;
            let mut provider = default_provider();
            Ok(CommandReport::Clean(project_links::clean(
                &mut ProjectCtx {
                    root: &root,
                    provider: provider.as_mut(),
                },
                command.options,
            )?))
        }
        Command::Doctor(command) => {
            let report = match paths::current_project_root() {
                Ok(root) => match paths::resolve_database_path() {
                    Ok(resolution) => doctor_project(&root, &resolution, command.verbose),
                    Err(error) => failed_doctor("database-path", error),
                },
                Err(error) => failed_doctor("project-root", error),
            };
            Ok(CommandReport::Doctor(report))
        }
    }
}

fn run_db(command: DbCommand) -> Result<CommandReport> {
    match command {
        DbCommand::Path => Ok(CommandReport::DbPath(paths::resolve_database_path()?)),
        DbCommand::Migrate => Ok(CommandReport::DbMigrate(store::migrate_default_database()?)),
        DbCommand::Backup { path } => Ok(CommandReport::DbBackup(store::backup_default_database(
            path.as_deref(),
        )?)),
        DbCommand::Check => Ok(CommandReport::DbCheck(store::check_default_database()?)),
    }
}

fn run_harness(command: HarnessCommand) -> Result<CommandReport> {
    let resolution = paths::resolve_database_path()?;
    let store = match &command {
        HarnessCommand::Enable { .. }
        | HarnessCommand::Disable { .. }
        | HarnessCommand::MappingAdd { .. }
        | HarnessCommand::MappingRemove { .. } => Store::open_rw(&resolution)?,
        _ => Store::open_ro(&resolution)?,
    };
    match command {
        HarnessCommand::List => Ok(CommandReport::HarnessList(store.list_harnesses()?)),
        HarnessCommand::Show { name } => Ok(CommandReport::HarnessShow(store.show_harness(&name)?)),
        HarnessCommand::Enable { name } => {
            store.enable_harness(&name)?;
            Ok(CommandReport::HarnessEnabled {
                name,
                enabled: true,
            })
        }
        HarnessCommand::Disable { name } => {
            store.disable_harness(&name)?;
            Ok(CommandReport::HarnessEnabled {
                name,
                enabled: false,
            })
        }
        HarnessCommand::MappingList { name } => {
            let mappings = if let Some(name) = name {
                store.list_mappings_for_harness(&name)?
            } else {
                store.list_all_mappings()?
            };
            Ok(CommandReport::HarnessMappingList(mappings))
        }
        HarnessCommand::MappingAdd {
            name,
            source,
            link,
            kind,
        } => Ok(CommandReport::HarnessMappingAdded(store.add_mapping(
            AddHarnessMapping {
                harness: name,
                source_path: source,
                link_path: link,
                link_kind: kind,
            },
        )?)),
        HarnessCommand::MappingRemove { name, link } => Ok(CommandReport::HarnessMappingRemoved(
            store.remove_mapping(&name, &link)?,
        )),
    }
}

fn run_linkable(command: LinkableCommand, item_type: LinkableItemType) -> Result<CommandReport> {
    let resolution = paths::resolve_database_path()?;
    let store = match &command {
        LinkableCommand::List | LinkableCommand::Show { .. } => Store::open_ro(&resolution)?,
        _ => Store::open_rw(&resolution)?,
    };
    match command {
        LinkableCommand::Add {
            path,
            name,
            alias,
            target_dir,
        } => {
            let name = match name {
                Some(name) => name,
                None => default_item_name(&path)?,
            };
            let item = store.add_item(AddLinkableItem {
                item_type,
                name,
                alias,
                source_path: path,
                default_target_dir: target_dir,
                description: None,
            })?;
            Ok(CommandReport::LinkableAdded(item))
        }
        LinkableCommand::List => Ok(CommandReport::LinkableList {
            item_type,
            items: store.list_items(item_type)?,
        }),
        LinkableCommand::Show { name } => Ok(CommandReport::LinkableShow(
            store.find_item(item_type, &name)?,
        )),
        LinkableCommand::Rename { old, new } => Ok(CommandReport::LinkableRenamed(
            store.rename_item(item_type, &old, &new)?,
        )),
        LinkableCommand::Remove { name } => Ok(CommandReport::LinkableRemoved(
            store.remove_item(item_type, &name)?,
        )),
        LinkableCommand::Refresh { name } => Ok(CommandReport::LinkableRefreshed(
            store.refresh_item(item_type, &name)?,
        )),
    }
}

fn run_group(command: GroupCommand) -> Result<CommandReport> {
    let resolution = paths::resolve_database_path()?;
    let store = match &command {
        GroupCommand::List | GroupCommand::Show { .. } => Store::open_ro(&resolution)?,
        _ => Store::open_rw(&resolution)?,
    };
    match command {
        GroupCommand::Create { name } => Ok(CommandReport::GroupCreated(
            store.create_group(&name, None)?,
        )),
        GroupCommand::List => Ok(CommandReport::GroupList(store.list_group_summaries()?)),
        GroupCommand::Show { name } => Ok(CommandReport::GroupShow(store.show_group(&name)?)),
        GroupCommand::Rename { old, new } => {
            Ok(CommandReport::GroupRenamed(store.rename_group(&old, &new)?))
        }
        GroupCommand::Delete { name } => {
            Ok(CommandReport::GroupDeleted(store.delete_group(&name)?))
        }
        GroupCommand::Add { group, items } => Ok(CommandReport::GroupUpdated(
            store.add_group_items(&group, &items)?,
        )),
        GroupCommand::Remove { group, items } => {
            Ok(CommandReport::GroupUpdated(GroupUpdateReport {
                group: store.remove_group_items(&group, &items)?,
                skipped: Vec::new(),
            }))
        }
    }
}

fn run_link(command: LinkCommand) -> Result<CommandReport> {
    let root = paths::current_project_root()?;
    let mut provider = default_provider();
    let mut ctx = ProjectCtx {
        root: &root,
        provider: provider.as_mut(),
    };
    let store = Store::open_ro(&paths::resolve_database_path()?)?;
    match command {
        LinkCommand::Item { request, options } => Ok(CommandReport::LinkItem(
            project_links::link_item(&mut ctx, &store, &request, options)?,
        )),
        LinkCommand::Group { group, options } => Ok(CommandReport::LinkGroup(
            project_links::link_group(&mut ctx, &store, &group, options)?,
        )),
    }
}

fn run_unlink(command: UnlinkCommand) -> Result<CommandReport> {
    let root = paths::current_project_root()?;
    let mut provider = default_provider();
    let mut ctx = ProjectCtx {
        root: &root,
        provider: provider.as_mut(),
    };
    let report = match command.target {
        UnlinkTarget::Item(identifier) => {
            project_links::unlink(&mut ctx, identifier.as_deref(), command.options)?
        }
        UnlinkTarget::Group(group) => {
            let store = Store::open_ro(&paths::resolve_database_path()?)?;
            project_links::unlink_group(&mut ctx, &store, &group, command.options)?
        }
    };
    Ok(CommandReport::Unlink(report))
}

fn failed_doctor(name: &str, error: Error) -> DoctorReport {
    DoctorReport {
        checks: vec![DoctorCheck {
            name: name.to_string(),
            ok: false,
            summary: error.to_string(),
            details: Vec::new(),
        }],
        ok: false,
    }
}

fn default_item_name(path: &Path) -> Result<String> {
    path.file_stem()
        .or_else(|| path.file_name())
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .ok_or_else(|| Error::invalid_arguments("source path must have a valid UTF-8 file name"))
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
