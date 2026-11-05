//! Acceptance tests written from the user-facing contract without reading src/.
use agent_linker::core::{
    linkable::LinkableItemType,
    manifest::{init_project_with_store, load_manifest},
    paths::{DbPathReason, DbPathResolution},
    registry::AddLinkableItem,
    store::Store,
    symlink::{
        LinkKind, LinkStatus, RemoveSymlinkOutcome, SymlinkBackend, SymlinkError, SymlinkErrorKind,
        SymlinkProvider, SymlinkResult,
    },
};
use std::collections::HashMap;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Sandbox {
    root: PathBuf,
    data: PathBuf,
    project: PathBuf,
}
impl Sandbox {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "aglink-blind-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let data = root.join("global");
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();
        Self {
            root,
            data,
            project,
        }
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_aglink"))
            .args(args)
            .current_dir(&self.project)
            .env("AGLINK_DATA_DIR", &self.data)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "command {args:?} failed: {}",
            display(&out)
        );
        display(&out)
    }
    fn fail(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            !out.status.success(),
            "command {args:?} unexpectedly succeeded: {}",
            display(&out)
        );
        display(&out)
    }
    fn migrated(&self) {
        self.ok(&["db", "migrate"]);
        self.ok(&["db", "check"]);
    }
    fn resolution(&self) -> DbPathResolution {
        DbPathResolution {
            path: self.data.join("aglink.db"),
            reason: DbPathReason::ExplicitDataDirEnv,
        }
    }
    fn skill(&self, folder: &str) -> PathBuf {
        let path = self.root.join(folder);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("SKILL.md"),
            "# Blind acceptance skill\n\nDo the described work.\n",
        )
        .unwrap();
        path
    }
    fn resource(&self, file: &str) -> PathBuf {
        let path = self.root.join(file);
        fs::write(&path, "resource acceptance content\n").unwrap();
        path
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        // This path is allocated directly beneath temp_dir with a unique fixed prefix.
        // remove_dir_all does not follow symlinks inside it.
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn display(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
fn s(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn defaults_follow_stable_ids_through_rename_and_registry_removal() {
    let sb = Sandbox::new();
    let resolution = sb.resolution();
    Store::migrate(&resolution).unwrap();
    let store = Store::open_rw(&resolution).unwrap();
    let skill_id = add_skill(&store, &sb, "before-rename", None);
    let resource_id = add_resource(&store, &sb, "before-resource-rename", None);
    store.add_init_item(&skill_id).unwrap();
    store.add_init_item(&resource_id).unwrap();
    store
        .rename_item(LinkableItemType::Skill, &skill_id, "after-rename")
        .unwrap();
    store
        .rename_item(
            LinkableItemType::Resource,
            &resource_id,
            "after-resource-rename",
        )
        .unwrap();
    let defaults = store.list_init_items().unwrap();
    assert_eq!(defaults.len(), 2);
    assert!(defaults
        .iter()
        .any(|i| i.id == skill_id && i.name == "after-rename"));
    assert!(defaults
        .iter()
        .any(|i| i.id == resource_id && i.name == "after-resource-rename"));
    store
        .remove_item(LinkableItemType::Skill, &skill_id)
        .unwrap();
    store
        .remove_item(LinkableItemType::Resource, &resource_id)
        .unwrap();
    assert!(
        store.list_init_items().unwrap().is_empty(),
        "registry removal left dangling defaults"
    );
}

#[test]
fn harness_stage_preflights_batch_before_defaults_and_preserves_real_directories() {
    let sb = Sandbox::new();
    let resolution = sb.resolution();
    Store::migrate(&resolution).unwrap();
    let store = Store::open_rw(&resolution).unwrap();
    let item = add_skill(&store, &sb, "unreached-default", None);
    store.add_init_item(&item).unwrap();
    fs::create_dir_all(sb.project.join(".claude/skills")).unwrap();
    fs::write(sb.project.join(".claude/skills/user.txt"), "user data").unwrap();
    let mut provider = MockProvider::default();
    assert!(init_project_with_store(&sb.project, &mut provider, &resolution).is_err());
    assert!(
        provider.creations.is_empty(),
        "harness conflict partially created the harness batch or reached defaults"
    );
    assert_eq!(
        fs::read_to_string(sb.project.join(".claude/skills/user.txt")).unwrap(),
        "user data"
    );
    assert!(sb.project.join("AGENTS.md").is_file());
    assert!(sb.project.join(".agents/skills").is_dir());
    assert!(sb.project.join(".gitignore").is_file());
    assert!(load_manifest(&sb.project).unwrap().links.is_empty());
}

#[test]
fn resource_without_alias_uses_source_filename_and_default_directory_conflicts_are_preserved() {
    let sb = Sandbox::new();
    let resolution = sb.resolution();
    Store::migrate(&resolution).unwrap();
    let store = Store::open_rw(&resolution).unwrap();
    store.disable_harness("claude").unwrap();
    let resource = add_resource(&store, &sb, "resource-registry-name", None);
    store.add_init_item(&resource).unwrap();
    let skill = add_skill(&store, &sb, "occupied-skill", None);
    store.add_init_item(&skill).unwrap();
    fs::create_dir_all(sb.project.join(".agents/skills/occupied-skill")).unwrap();
    fs::write(
        sb.project.join(".agents/skills/occupied-skill/user.txt"),
        "keep directory",
    )
    .unwrap();
    let mut provider = MockProvider::default();
    assert!(init_project_with_store(&sb.project, &mut provider, &resolution).is_err());
    assert!(provider.creations.is_empty());
    assert_eq!(
        fs::read_to_string(sb.project.join(".agents/skills/occupied-skill/user.txt")).unwrap(),
        "keep directory"
    );
    store.remove_init_item(&skill).unwrap();
    init_project_with_store(&sb.project, &mut provider, &resolution).unwrap();
    assert!(provider
        .links
        .contains_key(&sb.project.join("docs/shared/resource-registry-name.txt")));
}

#[test]
fn init_defaults_use_aliases_and_preserve_previous_project_links() {
    let sb = Sandbox::new();
    let resolution = sb.resolution();
    Store::migrate(&resolution).unwrap();
    let store = Store::open_rw(&resolution).unwrap();
    store.disable_harness("claude").unwrap();
    let skill_id = add_skill(&store, &sb, "first-skill", Some("friendly-skill"));
    let resource_id = add_resource(&store, &sb, "first-resource", Some("friendly-resource.txt"));
    store.add_init_item(&skill_id).unwrap();
    store.add_init_item(&resource_id).unwrap();
    let mut provider = MockProvider::default();
    let first_report = init_project_with_store(&sb.project, &mut provider, &resolution).unwrap();
    let skill_link = sb.project.join(".agents/skills/friendly-skill");
    let resource_link = sb.project.join("docs/shared/friendly-resource.txt");
    assert!(provider.links.contains_key(&skill_link));
    assert!(provider.links.contains_key(&resource_link));
    let manifest = load_manifest(&sb.project).unwrap();
    assert_eq!(manifest.links.len(), 2);
    assert!(
        first_report.link_outcomes.iter().all(|outcome| manifest
            .links
            .iter()
            .any(|record| record.id == outcome.id && record.link_path == outcome.link_path)),
        "init output IDs must match persisted manifest record IDs"
    );
    assert!(manifest
        .links
        .iter()
        .any(|l| l.item_id == skill_id && sb.project.join(&l.link_path) == skill_link));
    assert!(manifest
        .links
        .iter()
        .any(|l| l.item_id == resource_id && sb.project.join(&l.link_path) == resource_link));
    let repeated_report = init_project_with_store(&sb.project, &mut provider, &resolution).unwrap();
    assert!(
        repeated_report.link_outcomes.iter().all(|outcome| manifest
            .links
            .iter()
            .any(|record| record.id == outcome.id && record.link_path == outcome.link_path)),
        "repeated init must report reused record IDs"
    );
    assert_eq!(
        provider.creations.len(),
        2,
        "repeated init recreated correct links"
    );
    store.remove_init_item(&skill_id).unwrap();
    let next_id = add_skill(&store, &sb, "second-skill", None);
    store.add_init_item(&next_id).unwrap();
    assert_eq!(
        provider.links.len(),
        2,
        "editing defaults changed project state eagerly"
    );
    init_project_with_store(&sb.project, &mut provider, &resolution).unwrap();
    assert_eq!(
        provider.links.len(),
        3,
        "later init must add current defaults and preserve previous items"
    );
    assert!(provider.links.contains_key(&skill_link));
    assert!(provider
        .links
        .contains_key(&sb.project.join(".agents/skills/second-skill")));
    assert_eq!(load_manifest(&sb.project).unwrap().links.len(), 3);
}

#[test]
fn default_stage_preflights_entire_batch_and_keeps_completed_harness_stage() {
    let sb = Sandbox::new();
    let resolution = sb.resolution();
    Store::migrate(&resolution).unwrap();
    let store = Store::open_rw(&resolution).unwrap();
    let first_id = add_skill(&store, &sb, "first-valid", None);
    let second_id = add_resource(&store, &sb, "second-conflicting", Some("occupied.txt"));
    store.add_init_item(&first_id).unwrap();
    store.add_init_item(&second_id).unwrap();
    fs::create_dir_all(sb.project.join("docs/shared")).unwrap();
    fs::write(
        sb.project.join("docs/shared/occupied.txt"),
        "user real resource",
    )
    .unwrap();
    let mut provider = MockProvider::default();
    assert!(init_project_with_store(&sb.project, &mut provider, &resolution).is_err());
    assert!(sb.project.join("AGENTS.md").is_file());
    assert!(sb.project.join(".agents/skills").is_dir());
    assert!(sb.project.join(".gitignore").is_file());
    assert!(
        !provider
            .links
            .contains_key(&sb.project.join(".agents/skills/first-valid")),
        "failed defaults batch partially created a link"
    );
    assert_eq!(
        fs::read_to_string(sb.project.join("docs/shared/occupied.txt")).unwrap(),
        "user real resource"
    );
    assert_eq!(
        provider.creations.len(),
        2,
        "both Claude harness links must complete before defaults"
    );
    let manifest = load_manifest(&sb.project).unwrap();
    assert_eq!(
        manifest.links.len(),
        provider.links.len(),
        "completed harness links were not saved after default-stage failure"
    );
    assert!(manifest
        .links
        .iter()
        .all(|l| l.harness_name.as_deref() == Some("claude")));
}

#[test]
fn invalid_default_source_is_rejected_before_creating_any_default_link() {
    let sb = Sandbox::new();
    let resolution = sb.resolution();
    Store::migrate(&resolution).unwrap();
    let store = Store::open_rw(&resolution).unwrap();
    store.disable_harness("claude").unwrap();
    let valid = add_skill(&store, &sb, "valid", None);
    let stale = add_skill(&store, &sb, "stale", None);
    store.add_init_item(&valid).unwrap();
    store.add_init_item(&stale).unwrap();
    fs::remove_file(sb.root.join("source-stale/SKILL.md")).unwrap();
    let mut provider = MockProvider::default();
    assert!(
        init_project_with_store(&sb.project, &mut provider, &resolution).is_err(),
        "stale skill content was accepted"
    );
    assert!(
        provider.creations.is_empty(),
        "source failure partially created defaults"
    );
    assert!(load_manifest(&sb.project).unwrap().links.is_empty());
}

#[test]
fn missing_global_database_uses_builtin_harness_without_creating_database() {
    let sb = Sandbox::new();
    let resolution = sb.resolution();
    let mut provider = MockProvider::default();
    init_project_with_store(&sb.project, &mut provider, &resolution).unwrap();
    assert!(
        !resolution.path.exists(),
        "init implicitly created the global database"
    );
    assert_eq!(
        provider.creations.len(),
        2,
        "missing DB lost built-in Claude compatibility"
    );
    let manifest = load_manifest(&sb.project).unwrap();
    assert_eq!(manifest.links.len(), provider.links.len());
    assert!(manifest
        .links
        .iter()
        .all(|l| l.harness_name.as_deref() == Some("claude")));
}

#[test]
fn repeated_harness_and_default_init_preserves_manifest_across_clock_ticks() {
    let sb = Sandbox::new();
    let resolution = sb.resolution();
    Store::migrate(&resolution).unwrap();
    let store = Store::open_rw(&resolution).unwrap();
    let item = add_skill(&store, &sb, "repeatable-default", None);
    store.add_init_item(&item).unwrap();
    let mut provider = MockProvider::default();
    init_project_with_store(&sb.project, &mut provider, &resolution).unwrap();
    let first_manifest = fs::read(sb.project.join(".agents/links.toml")).unwrap();
    let creations = provider.creations.len();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    init_project_with_store(&sb.project, &mut provider, &resolution).unwrap();
    assert_eq!(provider.creations.len(), creations);
    assert_eq!(
        fs::read(sb.project.join(".agents/links.toml")).unwrap(),
        first_manifest,
        "unchanged repeated init rewrote link records"
    );
}

#[test]
fn v3_requires_explicit_migration_and_preserves_registry_rows() {
    let sb = Sandbox::new();
    let resolution = sb.resolution();
    Store::migrate(&resolution).unwrap();
    {
        let store = Store::open_rw(&resolution).unwrap();
        let skill = add_skill(&store, &sb, "v3-skill", Some("v3-alias"));
        let resource = add_resource(&store, &sb, "v3-resource", None);
        let group = store
            .create_group("v3-group", Some("preserve this group"))
            .unwrap();
        store
            .add_group_items(&group.id, &[skill, resource])
            .unwrap();
        store.disable_harness("claude").unwrap();
    }
    let tables = [
        "harnesses",
        "harness_mappings",
        "linkable_items",
        "groups",
        "group_items",
    ];
    let connection = rusqlite::Connection::open(&resolution.path).unwrap();
    let snapshot = |db: &rusqlite::Connection| -> Vec<Vec<Vec<String>>> {
        tables
            .iter()
            .map(|table| {
                let mut query = db
                    .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
                    .unwrap();
                let count = query.column_count();
                let rows = query
                    .query_map([], |row| {
                        Ok((0..count)
                            .map(|index| format!("{:?}", row.get_ref(index).unwrap()))
                            .collect::<Vec<_>>())
                    })
                    .unwrap();
                rows.map(Result::unwrap).collect()
            })
            .collect()
    };
    let before = snapshot(&connection);
    connection
        .execute_batch("DROP TABLE init_items; PRAGMA user_version=3;")
        .unwrap();
    drop(connection);
    assert!(
        Store::open_rw(&resolution).is_err(),
        "old schema silently migrated during normal open"
    );
    let mut provider = MockProvider::default();
    assert!(
        init_project_with_store(&sb.project, &mut provider, &resolution).is_err(),
        "old schema silently migrated during init"
    );
    assert!(provider.creations.is_empty());
    Store::migrate(&resolution).unwrap();
    let connection = rusqlite::Connection::open(&resolution.path).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        snapshot(&connection),
        before,
        "v3 migration changed existing registry/harness/group rows"
    );
    assert!(Store::open_ro(&resolution)
        .unwrap()
        .list_init_items()
        .unwrap()
        .is_empty());
}

#[derive(Default)]
struct MockProvider {
    links: HashMap<PathBuf, PathBuf>,
    creations: Vec<PathBuf>,
}
impl SymlinkProvider for MockProvider {
    fn backend(&self) -> SymlinkBackend {
        SymlinkBackend::Mock
    }
    fn create_symlink(&mut self, source: &Path, link: &Path, _kind: LinkKind) -> SymlinkResult<()> {
        assert!(
            !self.links.contains_key(link),
            "provider received duplicate create for {link:?}"
        );
        assert!(
            !link.exists(),
            "provider was asked to overwrite a real path: {link:?}"
        );
        self.links.insert(link.to_owned(), source.to_owned());
        self.creations.push(link.to_owned());
        Ok(())
    }
    fn replace_symlink(
        &mut self,
        source: &Path,
        link: &Path,
        _kind: LinkKind,
    ) -> SymlinkResult<()> {
        assert!(
            self.links.contains_key(link),
            "replacement must refer to a symlink"
        );
        self.links.insert(link.to_owned(), source.to_owned());
        Ok(())
    }
    fn remove_symlink(&mut self, link: &Path) -> SymlinkResult<RemoveSymlinkOutcome> {
        Ok(if self.links.remove(link).is_some() {
            RemoveSymlinkOutcome::Removed
        } else {
            RemoveSymlinkOutcome::Missing
        })
    }
    fn read_link(&self, link: &Path) -> SymlinkResult<PathBuf> {
        self.links
            .get(link)
            .cloned()
            .ok_or_else(|| SymlinkError::new(SymlinkErrorKind::Io, SymlinkBackend::Mock))
    }
    fn link_status(
        &self,
        source: &Path,
        link: &Path,
        _kind: LinkKind,
    ) -> SymlinkResult<LinkStatus> {
        if let Some(target) = self.links.get(link) {
            return Ok(if target == source {
                LinkStatus::CorrectSymlink {
                    target: target.clone(),
                }
            } else {
                LinkStatus::WrongSymlinkTarget {
                    actual: target.clone(),
                    expected: source.to_owned(),
                }
            });
        }
        if link.is_dir() {
            return Ok(LinkStatus::ExistingRealDirectory);
        }
        if link.exists() {
            return Ok(LinkStatus::ExistingRealFile);
        }
        Ok(LinkStatus::Missing)
    }
}

fn add_skill(store: &Store, sb: &Sandbox, name: &str, alias: Option<&str>) -> String {
    let source = sb.skill(&format!("source-{name}"));
    store
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: name.to_owned(),
            alias: alias.map(str::to_owned),
            source_path: source,
            default_target_dir: None,
            description: None,
        })
        .unwrap()
        .id
}
fn add_resource(store: &Store, sb: &Sandbox, name: &str, alias: Option<&str>) -> String {
    let source = sb.resource(&format!("{name}.txt"));
    store
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Resource,
            name: name.to_owned(),
            alias: alias.map(str::to_owned),
            source_path: source,
            default_target_dir: Some(PathBuf::from("docs/shared")),
            description: None,
        })
        .unwrap()
        .id
}

#[test]
fn defaults_are_explicit_shared_idempotent_and_validate_unknown_items() {
    let sb = Sandbox::new();
    sb.migrated();
    let initial = sb.ok(&["init", "defaults", "list"]);
    assert!(
        !initial.contains("aglink-cli"),
        "migration injected a special item: {initial}"
    );
    let skill = sb.skill("source-skill");
    let resource = sb.resource("source-resource.txt");
    sb.ok(&[
        "skill",
        "add",
        s(&skill),
        "--name",
        "blind-default-skill",
        "--alias",
        "skill-alias",
    ]);
    sb.ok(&[
        "resource",
        "add",
        s(&resource),
        "--target-dir",
        "docs/shared",
        "--name",
        "blind-default-resource",
        "--alias",
        "resource-alias.txt",
    ]);
    let unselected = sb.ok(&["init", "defaults", "list"]);
    assert!(
        !unselected.contains("blind-default-skill")
            && !unselected.contains("blind-default-resource"),
        "ordinary registration selected defaults: {unselected}"
    );
    for name in ["blind-default-skill", "blind-default-resource"] {
        sb.ok(&["init", "defaults", "add", name]);
        let once = sb.ok(&["init", "defaults", "list"]);
        let item_type = if name.ends_with("skill") {
            "skill"
        } else {
            "resource"
        };
        let show = sb.ok(&[item_type, "show", name]);
        let id = show
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .find(|token| token.starts_with("itm_"))
            .expect("public item show output lacks an ID");
        assert!(
            once.contains(id),
            "default listing must include stable item IDs: {once}"
        );
        sb.ok(&["init", "defaults", "add", id]);
        assert_eq!(
            once,
            sb.ok(&["init", "defaults", "list"]),
            "repeated add changed configuration"
        );
    }
    let selected = sb.ok(&["init", "defaults", "list"]);
    assert!(
        selected.contains("blind-default-skill") && selected.contains("blind-default-resource"),
        "both item kinds must share defaults: {selected}"
    );
    let lower = selected.to_lowercase();
    assert!(
        lower.contains("skill") && lower.contains("resource"),
        "default listing omits item types: {selected}"
    );
    sb.fail(&["init", "defaults", "add", "blind-unknown-item"]);
    sb.fail(&["init", "defaults", "remove", "blind-unknown-item"]);
    assert_eq!(
        selected,
        sb.ok(&["init", "defaults", "list"]),
        "unknown item operation changed defaults"
    );
    for name in ["blind-default-skill", "blind-default-resource"] {
        sb.ok(&["init", "defaults", "remove", name]);
        let once = sb.ok(&["init", "defaults", "list"]);
        sb.ok(&["init", "defaults", "remove", name]);
        assert_eq!(
            once,
            sb.ok(&["init", "defaults", "list"]),
            "repeated removal changed configuration"
        );
    }
    let empty = sb.ok(&["init", "defaults", "list"]);
    assert!(!empty.contains("blind-default-skill") && !empty.contains("blind-default-resource"));
}

#[test]
fn init_is_cwd_only_repeatable_and_preserves_real_user_content() {
    let sb = Sandbox::new();
    sb.migrated();
    sb.ok(&["harness", "disable", "claude"]);
    fs::write(sb.project.join("AGENTS.md"), "# User-authored guidance\n").unwrap();
    fs::write(sb.project.join(".gitignore"), "user-private/\n").unwrap();
    fs::create_dir_all(sb.project.join(".agents/skills/personal")).unwrap();
    fs::write(sb.project.join(".agents/skills/personal/notes"), "keep me").unwrap();
    sb.ok(&["init"]);
    assert_eq!(
        fs::read_to_string(sb.project.join("AGENTS.md")).unwrap(),
        "# User-authored guidance\n"
    );
    assert!(fs::read_to_string(sb.project.join(".gitignore"))
        .unwrap()
        .contains("user-private/"));
    assert_eq!(
        fs::read_to_string(sb.project.join(".agents/skills/personal/notes")).unwrap(),
        "keep me"
    );
    let skills = fs::symlink_metadata(sb.project.join(".agents/skills")).unwrap();
    assert!(
        skills.is_dir() && !skills.file_type().is_symlink(),
        "skills must be a real directory"
    );
    let manifest = fs::read(sb.project.join(".agents/links.toml")).unwrap();
    let ignore = fs::read(sb.project.join(".gitignore")).unwrap();
    sb.ok(&["init"]);
    assert_eq!(
        manifest,
        fs::read(sb.project.join(".agents/links.toml")).unwrap()
    );
    assert_eq!(ignore, fs::read(sb.project.join(".gitignore")).unwrap());
    let status = sb.ok(&["status", "--json"]);
    // Parse stdout separately: diagnostics may be emitted on stderr.
    let output = sb.run(&["status", "--json"]);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("invalid status JSON {e}: {status}"));
    assert_eq!(value["links"].as_array().unwrap().len(), 0);
    assert_eq!(
        PathBuf::from(value["project_root"].as_str().unwrap())
            .canonicalize()
            .unwrap(),
        sb.project.canonicalize().unwrap()
    );
    assert!(PathBuf::from(value["manifest_path"].as_str().unwrap()).is_file());
    let alternate = sb.root.join("must-not-be-created");
    sb.fail(&["init", s(&alternate)]);
    assert!(
        !alternate.exists(),
        "init accepted a positional project path"
    );
}

#[test]
fn registering_aglink_cli_has_no_special_default_behavior() {
    let sb = Sandbox::new();
    sb.migrated();
    sb.ok(&["harness", "disable", "claude"]);
    let skill = sb.skill("ordinary-aglink-cli-source");
    sb.ok(&["skill", "add", s(&skill), "--name", "aglink-cli"]);
    let defaults = sb.ok(&["init", "defaults", "list"]);
    assert!(
        !defaults.contains("aglink-cli"),
        "registration hard-coded aglink-cli as a default: {defaults}"
    );
    sb.ok(&["init"]);
    assert!(!sb.project.join(".agents/skills/aglink-cli").exists());
}

#[test]
fn skill_registration_requires_a_nonempty_skill_document() {
    let sb = Sandbox::new();
    sb.migrated();
    let source = sb.root.join("invalid-skill");
    fs::create_dir_all(&source).unwrap();
    sb.fail(&[
        "skill",
        "add",
        s(&source),
        "--name",
        "blind-missing-document",
    ]);
    fs::write(source.join("SKILL.md"), "").unwrap();
    sb.fail(&["skill", "add", s(&source), "--name", "blind-empty-document"]);
    fs::write(source.join("SKILL.md"), " \r\n\t").unwrap();
    sb.fail(&[
        "skill",
        "add",
        s(&source),
        "--name",
        "blind-whitespace-document",
    ]);
}
