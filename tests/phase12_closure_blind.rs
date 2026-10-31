//! Independent acceptance tests derived only from the supplied closure contract.
use agent_linker::core::{
    harness::{default_init_mappings, AddHarnessMapping},
    linkable::LinkableItemType,
    manifest::{
        init_project_with_mappings, init_project_with_store, load_manifest, manifest_path,
        save_manifest, LinkOrigin, LinkRecord, Manifest,
    },
    paths::{normalize_windows_path, DbPathReason, DbPathResolution},
    project_links::{
        link_project_with_provider_and_db_with_options, unlink_group_project_with_provider_and_db,
        unlink_project_with_provider, LinkItemRequest, LinkOptions,
    },
    registry::AddLinkableItem,
    store::Store,
    symlink::{LinkKind, MockSymlinkProvider, SymlinkBackend, SymlinkProvider},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "aglink-phase12-blind-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn root(&self) -> &Path {
        &self.0
    }
    fn resolution(&self) -> DbPathResolution {
        DbPathResolution {
            path: self.0.join("data/agent-linker.db"),
            reason: DbPathReason::ExplicitDataDirEnv,
        }
    }
    fn store(&self) -> Store {
        let db = self.resolution();
        Store::migrate(&db).unwrap();
        Store::open_rw(&db).unwrap()
    }
    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_aglink"))
            .current_dir(self.root())
            .env("AGLINK_DATA_DIR", self.0.join("data"))
            .args(args)
            .output()
            .unwrap()
    }
    fn provider(&self) -> MockSymlinkProvider {
        let mut p = MockSymlinkProvider::new();
        for name in ["", ".agents", ".agents/skills", ".claude", "docs"] {
            p.add_dir(self.0.join(name));
        }
        p.add_file(self.0.join("AGENTS.md"));
        p
    }
    fn skill(
        &self,
        store: &Store,
        name: &str,
        alias: Option<&str>,
    ) -> agent_linker::core::linkable::LinkableItem {
        let source = self.0.join(format!("source-{name}"));
        fs::create_dir(&source).unwrap();
        fs::write(source.join("SKILL.md"), name).unwrap();
        store
            .add_item(AddLinkableItem {
                item_type: LinkableItemType::Skill,
                name: name.into(),
                alias: alias.map(str::to_owned),
                source_path: source,
                default_target_dir: None,
                description: None,
            })
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn request(identifier: &str, alias: Option<&str>) -> LinkItemRequest {
    LinkItemRequest {
        identifier: identifier.into(),
        link_name_override: alias.map(str::to_owned),
        target_dir_override: None,
    }
}
fn options(dry_run: bool, force: bool) -> LinkOptions {
    LinkOptions { dry_run, force }
}
fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn mapping_normalizes_only_destination_and_initializes_dot_prefixed_paths() {
    let f = Fixture::new();
    let store = f.store();
    let mapping = store
        .add_mapping(AddHarnessMapping {
            harness: "claude".into(),
            source_path: "./AGENTS.md".into(),
            link_path: "./GEMINI.md".into(),
            link_kind: LinkKind::File,
        })
        .unwrap();
    assert_eq!(mapping.source_path, PathBuf::from("./AGENTS.md"));
    assert_eq!(mapping.link_path, PathBuf::from("GEMINI.md"));
    assert!(store
        .add_mapping(AddHarnessMapping {
            harness: "claude".into(),
            source_path: "AGENTS.md".into(),
            link_path: "GEMINI.md".into(),
            link_kind: LinkKind::File
        })
        .is_err());
    let mut provider = f.provider();
    init_project_with_store(f.root(), &mut provider, &f.resolution()).unwrap();
    assert_eq!(
        provider.read_link(&f.0.join("GEMINI.md")).unwrap(),
        f.0.join("AGENTS.md")
    );
    assert_eq!(
        load_manifest(f.root())
            .unwrap()
            .links
            .iter()
            .find(|r| r.link_path == PathBuf::from("GEMINI.md"))
            .unwrap()
            .origin,
        LinkOrigin::Harness
    );
    assert_eq!(
        store
            .remove_mapping("claude", Path::new("./GEMINI.md"))
            .unwrap()
            .id,
        mapping.id
    );
    assert!(store
        .remove_mapping("claude", Path::new("./CLAUDE.md"))
        .is_err());
}

fn insert_old_mapping(store: &Store, id: &str, path: &str) {
    let harness = store.show_harness("claude").unwrap();
    let harness_id = &harness.mappings[0].harness_id;
    store.connection().execute("INSERT INTO harness_mappings (id,harness_id,source_path,link_path,link_kind,required,created_at,updated_at) VALUES (?1,?2,'AGENTS.md',?3,'file',0,17,19)",rusqlite::params![id,harness_id,path]).unwrap();
}

#[test]
fn legacy_mapping_equivalence_rejects_add_and_remove_accepts_both_spellings() {
    for spelling in ["./OLD.md", "OLD.md"] {
        let f = Fixture::new();
        let store = f.store();
        insert_old_mapping(&store, "old", "./OLD.md");
        assert!(store
            .add_mapping(AddHarnessMapping {
                harness: "claude".into(),
                source_path: "AGENTS.md".into(),
                link_path: "OLD.md".into(),
                link_kind: LinkKind::File
            })
            .is_err());
        let old = store.remove_mapping("claude", Path::new(spelling)).unwrap();
        assert_eq!(old.id, "old");
        assert_eq!(old.created_at, 17);
        assert_eq!(old.updated_at, 19);
        assert!(store
            .show_harness("claude")
            .unwrap()
            .mappings
            .iter()
            .all(|m| m.id != "old"));
    }
}

#[test]
fn historical_mapping_duplicates_remain_distinct_and_literal_removal_wins() {
    let f = Fixture::new();
    let store = f.store();
    insert_old_mapping(&store, "legacy-dot", "./DUP.md");
    insert_old_mapping(&store, "legacy-plain", "DUP.md");
    let count = || {
        store
            .show_harness("claude")
            .unwrap()
            .mappings
            .iter()
            .filter(|m| m.id.starts_with("legacy-"))
            .count()
    };
    assert_eq!(count(), 2);
    assert_eq!(
        store
            .remove_mapping("claude", Path::new("DUP.md"))
            .unwrap()
            .id,
        "legacy-plain"
    );
    assert_eq!(count(), 1);
    assert_eq!(
        store
            .remove_mapping("claude", Path::new("DUP.md"))
            .unwrap()
            .id,
        "legacy-dot"
    );
    assert_eq!(count(), 0);
    insert_old_mapping(&store, "again-dot", "./DUP.md");
    insert_old_mapping(&store, "again-plain", "DUP.md");
    assert_eq!(
        store
            .remove_mapping("claude", Path::new("./DUP.md"))
            .unwrap()
            .id,
        "again-dot"
    );
    assert_eq!(
        store
            .remove_mapping("claude", Path::new("./DUP.md"))
            .unwrap()
            .id,
        "again-plain"
    );
}

#[test]
fn group_create_and_rename_return_written_identity_when_name_is_another_id() {
    let f = Fixture::new();
    let store = f.store();
    let item = f.skill(&store, "writer", None);
    let first = store.create_group("first", Some("original")).unwrap();
    store
        .add_group_items(&first.id, &[item.id.clone()])
        .unwrap();
    let second = store.create_group(&first.id, Some("new")).unwrap();
    assert_ne!(second.id, first.id);
    assert_eq!(second.name, first.id);
    assert!(second.items.is_empty());
    assert_eq!(second.description.as_deref(), Some("new"));
    assert_eq!(store.show_group(&second.id).unwrap(), second);
    assert_eq!(store.show_group(&first.id).unwrap().items[0].id, item.id);
    let third = store.create_group("third", None).unwrap();
    store.add_group_items(&third.id, &[item.id]).unwrap();
    let renamed = store.rename_group(&third.id, &second.id).unwrap();
    assert_eq!(renamed.id, third.id);
    assert_eq!(renamed.name, second.id);
    assert_eq!(renamed.items.len(), 1);
    assert_eq!(store.show_group(&third.id).unwrap(), renamed);
    assert!(store.show_group(&second.id).unwrap().items.is_empty());
}

fn legacy_record(id: &str, framework: Option<&str>, link: &str) -> String {
    format!("[[links]]\nid = \"{id}\"\nscope = \"project\"\n{}item_id = \"stable-{id}\"\nitem_name = \"{id}\"\nsource_path = \"AGENTS.md\"\nlink_path = \"{link}\"\nlink_kind = \"file\"\nprovider_backend = \"mock\"\ncreated_by_command = \"link\"\ncreated_at = \"unix:123\"\nupdated_at = \"unix:456\"\n",framework.map(|f|format!("framework_name = \"{f}\"\n")).unwrap_or_default())
}

#[test]
fn legacy_manifest_origin_inference_and_new_serialization_preserve_identity() {
    let f = Fixture::new();
    fs::create_dir(f.0.join(".agents")).unwrap();
    let text = format!(
        "schema_version = 1\n{}{}{}",
        legacy_record("registry", Some("registry"), "docs/item"),
        legacy_record("claude", Some("claude"), "CLAUDE.md"),
        legacy_record("other", Some("custom"), "CUSTOM.md")
    );
    fs::write(manifest_path(f.root()), text).unwrap();
    let parsed = load_manifest(f.root()).unwrap();
    assert_eq!(parsed.links.len(), 3);
    assert_eq!(parsed.links[0].origin, LinkOrigin::Item);
    assert_eq!(parsed.links[0].harness_name, None);
    for record in &parsed.links {
        assert_eq!(record.item_id, format!("stable-{}", record.id));
        assert_eq!(record.created_at, 123);
        assert_eq!(record.updated_at, 456);
        assert_eq!(record.source_path, PathBuf::from("AGENTS.md"));
    }
    assert!(parsed.links[1..]
        .iter()
        .all(|r| r.origin == LinkOrigin::Harness));
    save_manifest(f.root(), &parsed).unwrap();
    assert_eq!(load_manifest(f.root()).unwrap(), parsed);
    let saved = fs::read_to_string(manifest_path(f.root())).unwrap();
    assert!(!saved.contains("scope ="));
    assert!(!saved.contains("framework_name = \"registry\""));
    let item_section = saved
        .split("[[links]]")
        .find(|s| s.contains("id = \"registry\""))
        .unwrap();
    assert!(!item_section.contains("harness_name"));
    assert!(!item_section.contains("framework_name"));
}

#[test]
fn new_manifest_roundtrip_replaces_existing_file_and_retains_epoch_integers() {
    let f = Fixture::new();
    save_manifest(f.root(), &Manifest::empty()).unwrap();
    let item = LinkRecord {
        id: "link:opaque".into(),
        origin: LinkOrigin::Item,
        harness_name: None,
        item_id: "opaque".into(),
        item_name: "alpha".into(),
        source_path: f.0.join("source"),
        link_path: "docs/alias".into(),
        link_kind: LinkKind::File,
        provider_backend: SymlinkBackend::Mock,
        created_by_command: "link".into(),
        created_at: 1_234_567,
        updated_at: 2_345_678,
    };
    let desired = Manifest {
        schema_version: 1,
        links: vec![item],
    };
    save_manifest(f.root(), &desired).unwrap();
    assert_eq!(load_manifest(f.root()).unwrap(), desired);
    assert_eq!(fs::read_dir(f.0.join(".agents")).unwrap().count(), 1);
}

#[test]
fn init_preserves_crlf_bytes_outside_managed_gitignore_block() {
    let f = Fixture::new();
    let prefix = b"# user \xE9\x9B\xAA\r\nkeep/\r\n\r\n";
    let suffix = b"\r\n# tail\r\nlast/";
    let original = [
        prefix.as_slice(),
        b"# BEGIN aglink managed\r\nold/\r\n# END aglink managed",
        suffix.as_slice(),
    ]
    .concat();
    fs::write(f.0.join(".gitignore"), original).unwrap();
    let mut provider = f.provider();
    init_project_with_mappings(f.root(), &mut provider, &default_init_mappings()).unwrap();
    let saved = fs::read(f.0.join(".gitignore")).unwrap();
    assert!(saved.starts_with(prefix));
    assert!(saved.ends_with(suffix));
    let text = String::from_utf8(saved).unwrap();
    assert!(text.lines().any(|s| s == ".claude/"));
    assert!(text.lines().any(|s| s == ".agents/"));
    assert_eq!(text.matches("# BEGIN aglink managed").count(), 1);
}

#[test]
fn force_replacement_failure_and_dryrun_preserve_original_link_and_manifest() {
    let f = Fixture::new();
    let store = f.store();
    let item = f.skill(&store, "writer", None);
    let mut provider = f.provider();
    provider.add_dir(&item.source_path);
    save_manifest(f.root(), &Manifest::empty()).unwrap();
    let link = f.0.join(".agents/skills/writer");
    let old = f.0.join("missing-old");
    provider.add_symlink(&link, &old, LinkKind::Directory);
    let before = fs::read(manifest_path(f.root())).unwrap();
    let planned = link_project_with_provider_and_db_with_options(
        f.root(),
        &mut provider,
        &f.resolution(),
        request("writer", None),
        options(true, true),
    );
    assert!(planned.is_ok());
    assert_eq!(provider.read_link(&link).unwrap(), old);
    assert_eq!(fs::read(manifest_path(f.root())).unwrap(), before);
    provider.fail_next_replacement();
    assert!(link_project_with_provider_and_db_with_options(
        f.root(),
        &mut provider,
        &f.resolution(),
        request("writer", None),
        options(false, true)
    )
    .is_err());
    assert_eq!(provider.read_link(&link).unwrap(), old);
    assert_eq!(fs::read(manifest_path(f.root())).unwrap(), before);
    link_project_with_provider_and_db_with_options(
        f.root(),
        &mut provider,
        &f.resolution(),
        request("writer", None),
        options(false, true),
    )
    .unwrap();
    assert_eq!(provider.read_link(&link).unwrap(), item.source_path);
}

#[test]
fn group_then_all_unlink_use_item_origin_and_current_ids_for_every_alias() {
    let f = Fixture::new();
    let store = f.store();
    let alpha = f.skill(&store, "alpha", None);
    let beta = f.skill(&store, "beta", None);
    let mut provider = f.provider();
    provider.add_dir(&alpha.source_path);
    provider.add_dir(&beta.source_path);
    init_project_with_mappings(f.root(), &mut provider, &default_init_mappings()).unwrap();
    for (name, alias) in [
        ("alpha", Some("a-one")),
        ("alpha", Some("a-two")),
        ("beta", None),
    ] {
        link_project_with_provider_and_db_with_options(
            f.root(),
            &mut provider,
            &f.resolution(),
            request(name, alias),
            options(false, false),
        )
        .unwrap();
    }
    store.create_group("daily", None).unwrap();
    store
        .add_group_items("daily", &[alpha.id.clone(), beta.id.clone()])
        .unwrap();
    // Rebuild current membership without depending on an additional exported API.
    store.delete_group("daily").unwrap();
    store.create_group("daily", None).unwrap();
    store.add_group_items("daily", &[alpha.id.clone()]).unwrap();
    let mut manifest = load_manifest(f.root()).unwrap();
    manifest
        .links
        .iter_mut()
        .find(|r| r.origin == LinkOrigin::Harness)
        .unwrap()
        .item_id = alpha.id.clone();
    save_manifest(f.root(), &manifest).unwrap();
    let harness: Vec<_> = manifest
        .links
        .iter()
        .filter(|r| r.origin == LinkOrigin::Harness)
        .cloned()
        .collect();
    unlink_group_project_with_provider_and_db(f.root(), &mut provider, &f.resolution(), "daily")
        .unwrap();
    let remaining = load_manifest(f.root()).unwrap();
    assert_eq!(remaining.links.len(), 3);
    assert!(remaining.links.iter().any(|r| r.item_id == beta.id));
    assert!(provider
        .read_link(&f.0.join(".agents/skills/a-one"))
        .is_err());
    assert!(provider
        .read_link(&f.0.join(".agents/skills/a-two"))
        .is_err());
    unlink_project_with_provider(f.root(), &mut provider, None).unwrap();
    assert_eq!(load_manifest(f.root()).unwrap().links, harness);
    for record in harness {
        assert!(provider.read_link(&f.0.join(record.link_path)).is_ok());
    }
}

#[test]
fn alias_query_rejection_and_rename_leave_project_and_source_bytes_untouched() {
    let f = Fixture::new();
    let store = f.store();
    let alpha = f.skill(&store, "alpha", Some("beta"));
    let beta = f.skill(&store, "beta", Some("gamma"));
    let mut provider = f.provider();
    provider.add_dir(&alpha.source_path);
    provider.add_dir(&beta.source_path);
    save_manifest(f.root(), &Manifest::empty()).unwrap();
    assert_eq!(
        store.find_item(LinkableItemType::Skill, "beta").unwrap().id,
        beta.id
    );
    assert_eq!(store.find_any_item(&alpha.id).unwrap().id, alpha.id);
    assert!(store.find_any_item("gamma").is_err());
    assert!(store.remove_item(LinkableItemType::Skill, "gamma").is_err());
    assert!(store
        .refresh_item(LinkableItemType::Skill, "gamma")
        .is_err());
    assert!(store
        .rename_item(LinkableItemType::Skill, "gamma", "other")
        .is_err());
    store.create_group("daily", None).unwrap();
    assert!(store.add_group_items("daily", &["gamma".into()]).is_err());
    assert!(link_project_with_provider_and_db_with_options(
        f.root(),
        &mut provider,
        &f.resolution(),
        request("gamma", None),
        options(false, false)
    )
    .is_err());
    link_project_with_provider_and_db_with_options(
        f.root(),
        &mut provider,
        &f.resolution(),
        request("beta", None),
        options(false, false),
    )
    .unwrap();
    let before = fs::read(manifest_path(f.root())).unwrap();
    let old_target = provider
        .read_link(&f.0.join(".agents/skills/gamma"))
        .unwrap();
    let renamed = store
        .rename_item(LinkableItemType::Skill, "beta", "renamed")
        .unwrap();
    assert_eq!(renamed.id, beta.id);
    assert_eq!(renamed.source_path, beta.source_path);
    assert_eq!(renamed.alias, beta.alias);
    assert_eq!(fs::read(manifest_path(f.root())).unwrap(), before);
    assert_eq!(
        provider
            .read_link(&f.0.join(".agents/skills/gamma"))
            .unwrap(),
        old_target
    );
    assert_eq!(
        fs::read(alpha.source_path.join("SKILL.md")).unwrap(),
        b"alpha"
    );
    assert_eq!(
        fs::read(beta.source_path.join("SKILL.md")).unwrap(),
        b"beta"
    );
}

#[test]
fn db_check_does_not_report_aggregate_health_as_writable() {
    let f = Fixture::new();
    let migration = f.cli(&["db", "migrate"]);
    assert!(migration.status.success(), "{}", output_text(&migration));
    let check = f.cli(&["db", "check"]);
    assert!(check.status.success(), "{}", output_text(&check));
    let text = output_text(&check);
    assert!(text.contains("Status: healthy_writable"), "{text}");
    assert!(
        !text
            .lines()
            .any(|line| line.trim_start().starts_with("Writable:")),
        "{text}"
    );
    let path = normalize_windows_path(&fs::canonicalize(f.root()).unwrap());
    assert!(path.is_absolute());
}

#[cfg(windows)]
#[test]
#[ignore = "requires an available Windows Broker"]
fn windows_broker_replaces_file_and_directory_links_natively() {
    use agent_linker::core::symlink::WindowsBrokerSymlinkProvider;
    for kind in [LinkKind::File, LinkKind::Directory] {
        for dangling in [false, true] {
            let f = Fixture::new();
            let old = f.0.join("old-source");
            let new = f.0.join("new-source");
            let link = f.0.join("link");
            if kind == LinkKind::File {
                fs::write(&old, "old").unwrap();
                fs::write(&new, "new").unwrap();
            } else {
                fs::create_dir(&old).unwrap();
                fs::create_dir(&new).unwrap();
                fs::write(new.join("marker"), "new").unwrap();
            }
            let mut broker = WindowsBrokerSymlinkProvider::new();
            broker.create_symlink(&old, &link, kind).unwrap();
            if dangling {
                if kind == LinkKind::File {
                    fs::remove_file(&old).unwrap();
                } else {
                    fs::remove_dir(&old).unwrap();
                }
            }
            broker.replace_symlink(&new, &link, kind).unwrap();
            assert_eq!(normalize_windows_path(&fs::read_link(&link).unwrap()), new);
            let contents = if kind == LinkKind::File {
                link
            } else {
                link.join("marker")
            };
            assert_eq!(fs::read(contents).unwrap(), b"new");
            println!("BROKER NATIVE REPLACEMENT: {kind:?}, dangling_old={dangling}, success");
        }
    }
}

#[cfg(windows)]
#[test]
#[ignore = "requires an available Windows Broker"]
fn windows_native_valid_source_locked_link_preserves_old_link() {
    use agent_linker::core::symlink::WindowsBrokerSymlinkProvider;
    use std::os::windows::fs::OpenOptionsExt;
    let f = Fixture::new();
    let old = f.0.join("old-file");
    let new = f.0.join("new-directory");
    let link = f.0.join("link");
    fs::write(&old, "old bytes").unwrap();
    fs::create_dir(&new).unwrap();
    let mut broker = WindowsBrokerSymlinkProvider::new();
    broker.create_symlink(&old, &link, LinkKind::File).unwrap();
    let locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .custom_flags(0x00200000)
        .open(&link)
        .unwrap();
    let error = broker
        .replace_symlink(&new, &link, LinkKind::Directory)
        .unwrap_err();
    println!("BROKER VALID SOURCE / LOCKED OLD LINK: {error:?} / {error}");
    assert_eq!(normalize_windows_path(&fs::read_link(&link).unwrap()), old);
    assert_eq!(fs::read(&link).unwrap(), b"old bytes");
    assert!(new.is_dir());
    drop(locked);
}

#[cfg(windows)]
#[test]
#[ignore = "observes explicit Windows fallback with Broker-created old link"]
fn windows_fallback_directory_replacement_observation() {
    use agent_linker::core::symlink::{StdSymlinkProvider, WindowsBrokerSymlinkProvider};
    let f = Fixture::new();
    let old = f.0.join("old-directory");
    let new = f.0.join("new-directory");
    let link = f.0.join("link");
    fs::create_dir(&old).unwrap();
    fs::create_dir(&new).unwrap();
    let mut broker = WindowsBrokerSymlinkProvider::new();
    broker
        .create_symlink(&old, &link, LinkKind::Directory)
        .unwrap();
    let mut fallback = StdSymlinkProvider::explicit_windows_fallback();
    match fallback.replace_symlink(&new, &link, LinkKind::Directory) {
        Ok(()) => {
            println!("WINDOWS EXPLICIT FALLBACK DIRECTORY REPLACEMENT: success");
            assert_eq!(normalize_windows_path(&fs::read_link(&link).unwrap()), new);
        }
        Err(error) => {
            println!("WINDOWS EXPLICIT FALLBACK DIRECTORY REPLACEMENT: {error:?} / {error}");
            assert_eq!(normalize_windows_path(&fs::read_link(&link).unwrap()), old);
        }
    }
    assert!(old.is_dir());
    assert!(new.is_dir());
}
