use std::path::{Path, PathBuf};

use crate::core::{
    error::{Error, Result},
    harness::{AddHarnessMapping, StoredHarness, StoredHarnessMapping},
    linkable::{LinkableItem, LinkableItemType},
    manifest::{init_current_project, InitProjectReport},
    paths::{self, DbPathResolution},
    project_links::{
        clean_current_project_with_options, doctor_current_project,
        link_current_project_with_options, link_group_current_project_with_options,
        status_current_project, unlink_current_project_with_options,
        unlink_group_current_project_with_options, CleanOptions, CleanReport, DoctorReport,
        LinkGroupReport, LinkItemReport, LinkItemRequest, LinkOptions, StatusReport, UnlinkOptions,
        UnlinkReport,
    },
    registry::{AddLinkableItem, Group, GroupSummary, GroupUpdateReport},
    store::{self, DbBackupReport, DbCheckReport, MigrationReport, Store},
    symlink::LinkKind,
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
    Create {
        name: String,
    },
    List,
    Show {
        name: String,
    },
    Rename {
        old: String,
        new: String,
    },
    Delete {
        name: String,
    },
    Add {
        group: String,
        items: Vec<String>,
    },
    Remove {
        group: String,
        items: Vec<String>,
    },
    Link {
        group: String,
        options: LinkOptions,
    },
    Unlink {
        group: String,
        options: UnlinkOptions,
    },
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
        Command::Init => Ok(CommandReport::Init(init_current_project()?)),
        Command::Db(command) => run_db(command),
        Command::Harness(command) => run_harness(command),
        Command::Skill(command) => run_linkable(command, LinkableItemType::Skill),
        Command::Resource(command) => run_linkable(command, LinkableItemType::Resource),
        Command::Group(command) => run_group(command),
        Command::Link(command) => run_link(command),
        Command::Unlink(command) => run_unlink(command),
        Command::Status(command) => {
            let report = status_current_project()?;
            Ok(CommandReport::Status {
                report,
                json: command.json,
            })
        }
        Command::Clean(command) => Ok(CommandReport::Clean(clean_current_project_with_options(
            command.options,
        )?)),
        Command::Doctor(command) => Ok(CommandReport::Doctor(doctor_current_project(
            command.verbose,
        ))),
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
    if let GroupCommand::Link { group, options } = command {
        return Ok(CommandReport::LinkGroup(
            link_group_current_project_with_options(&group, options)?,
        ));
    }
    if let GroupCommand::Unlink { group, options } = command {
        return Ok(CommandReport::Unlink(
            unlink_group_current_project_with_options(&group, options)?,
        ));
    }
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
        GroupCommand::Link { group, options } => Ok(CommandReport::LinkGroup(
            link_group_current_project_with_options(&group, options)?,
        )),
        GroupCommand::Unlink { group, options } => Ok(CommandReport::Unlink(
            unlink_group_current_project_with_options(&group, options)?,
        )),
    }
}

fn run_link(command: LinkCommand) -> Result<CommandReport> {
    match command {
        LinkCommand::Item { request, options } => Ok(CommandReport::LinkItem(
            link_current_project_with_options(request, options)?,
        )),
        LinkCommand::Group { group, options } => Ok(CommandReport::LinkGroup(
            link_group_current_project_with_options(&group, options)?,
        )),
    }
}

fn run_unlink(command: UnlinkCommand) -> Result<CommandReport> {
    match command.target {
        UnlinkTarget::Item(identifier) => Ok(CommandReport::Unlink(
            unlink_current_project_with_options(identifier, command.options)?,
        )),
        UnlinkTarget::Group(group) => Ok(CommandReport::Unlink(
            unlink_group_current_project_with_options(&group, command.options)?,
        )),
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
