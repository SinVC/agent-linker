use agent_linker::core::{
    error::Error,
    linkable::{LinkableItem, LinkableItemType},
    manifest::{load_manifest, manifest_path, save_manifest, LinkOrigin, LinkRecord, Manifest},
    paths::{managed_relative_path_key, DbPathReason, DbPathResolution},
    project_links::{self, CleanMode, CleanOptions, LinkItemRequest, LinkOptions, ProjectCtx},
    registry::AddLinkableItem,
    store::Store,
    symlink::{default_provider, CreateSymlinkOutcome, LinkKind, LinkStatus},
};
use std::{
    collections::HashSet,
    error::Error as _,
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "support/mock.rs"]
mod mock;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "aglink-review-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn store(&self) -> Store {
        let db = DbPathResolution {
            path: self.0.join("data/agent-linker.db"),
            reason: DbPathReason::ExplicitDataDirEnv,
        };
        Store::migrate(&db).unwrap();
        Store::open_rw(&db).unwrap()
    }
    fn resource(&self, store: &Store, name: &str, filename: &str, target: &str) -> LinkableItem {
        let dir = self.0.join("sources").join(name);
        fs::create_dir_all(&dir).unwrap();
        let source = dir.join(filename);
        fs::write(&source, name).unwrap();
        store
            .add_item(AddLinkableItem {
                item_type: LinkableItemType::Resource,
                name: name.into(),
                alias: None,
                source_path: source,
                default_target_dir: Some(target.into()),
                description: None,
            })
            .unwrap()
    }
    fn cli(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_aglink"))
            .current_dir(&self.0)
            .env("AGLINK_DATA_DIR", self.0.join("data"))
            .args(args)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn req(id: &str, target: Option<&str>) -> LinkItemRequest {
    LinkItemRequest {
        identifier: id.into(),
        link_name_override: None,
        target_dir_override: target.map(PathBuf::from),
    }
}
fn record(item: &LinkableItem, path: &str) -> LinkRecord {
    LinkRecord {
        id: "old-arbitrary-id".into(),
        origin: LinkOrigin::Item,
        harness_name: None,
        item_id: item.id.clone(),
        item_name: item.name.clone(),
        source_path: item.source_path.clone(),
        link_path: path.into(),
        link_kind: LinkKind::File,
        provider_backend: "old-backend".into(),
        created_by_command: "old-command".into(),
        created_at: 123,
        updated_at: 456,
    }
}
fn put(f: &Fixture, r: LinkRecord) {
    save_manifest(
        &f.0,
        &Manifest {
            schema_version: 1,
            links: vec![r],
        },
    )
    .unwrap();
}

#[test]
#[cfg(windows)]
fn real_broker_case_equivalent_group_dirs_and_dry_run() {
    let f = Fixture::new();
    let store = f.store();
    let one = f.resource(&store, "one", "one.txt", "Config/Sub");
    let two = f.resource(&store, "two", "two.txt", "config/sub");
    store.create_group("batch", None).unwrap();
    store
        .add_group_items("batch", &[one.id.clone(), two.id.clone()])
        .unwrap();
    save_manifest(&f.0, &Manifest::empty()).unwrap();
    let before = fs::read(manifest_path(&f.0)).unwrap();
    let mut provider = default_provider();
    let mut ctx = ProjectCtx {
        root: &f.0,
        provider: &mut *provider,
    };
    let preview = project_links::link_group(
        &mut ctx,
        &store,
        "batch",
        LinkOptions {
            dry_run: true,
            force: false,
        },
    )
    .unwrap();
    assert_eq!(preview.reports.len(), 2);
    assert!(!f.0.join("Config").exists());
    assert_eq!(fs::read(manifest_path(&f.0)).unwrap(), before);
    let report =
        project_links::link_group(&mut ctx, &store, "batch", LinkOptions::default()).unwrap();
    assert_eq!(report.reports.len(), 2);
    let dirs: Vec<_> = report
        .reports
        .iter()
        .flat_map(|r| r.created_dirs.iter())
        .map(|p| managed_relative_path_key(p.strip_prefix(&f.0).unwrap_or(p)))
        .collect();
    assert_eq!(
        dirs.len(),
        dirs.iter().collect::<HashSet<_>>().len(),
        "duplicate created dirs: {dirs:?}"
    );
    assert_eq!(dirs.len(), 2, "created dirs: {dirs:?}");
    for (name, source) in [("one.txt", &one.source_path), ("two.txt", &two.source_path)] {
        let path = f.0.join("Config/Sub").join(name);
        assert!(fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            fs::canonicalize(path).unwrap(),
            fs::canonicalize(source).unwrap()
        );
    }
    let links = load_manifest(&f.0).unwrap().links;
    assert_eq!(links.len(), 2);
    assert!(links.iter().any(|r| r.item_id == one.id));
    assert!(links.iter().any(|r| r.item_id == two.id));
}

#[test]
fn old_record_identity_is_preserved_for_single_and_group_links() {
    for group in [false, true] {
        let f = Fixture::new();
        let store = f.store();
        let item = f.resource(&store, "one", "one.txt", "docs");
        let old = record(&item, "docs/one.txt");
        put(&f, old.clone());
        let mut provider = mock::MockSymlinkProvider::new();
        provider.add_file(&item.source_path);
        provider.add_symlink(f.0.join(&old.link_path), &item.source_path, LinkKind::File);
        let mut ctx = ProjectCtx {
            root: &f.0,
            provider: &mut provider,
        };
        if group {
            store.create_group("batch", None).unwrap();
            store
                .add_group_items("batch", std::slice::from_ref(&item.id))
                .unwrap();
            let report =
                project_links::link_group(&mut ctx, &store, "batch", LinkOptions::default())
                    .unwrap();
            assert_eq!(
                report.reports[0].outcome,
                CreateSymlinkOutcome::AlreadyCorrect
            );
        } else {
            let report = project_links::link_item(
                &mut ctx,
                &store,
                &req(&item.id, Some("./docs")),
                LinkOptions::default(),
            )
            .unwrap();
            assert_eq!(report.outcome, CreateSymlinkOutcome::AlreadyCorrect);
        }
        let loaded = load_manifest(&f.0).unwrap();
        assert_eq!(loaded.links.len(), 1);
        assert_eq!(loaded.links[0].id, old.id);
        assert_eq!(loaded.links[0].created_at, old.created_at);
        assert!(provider.mutations.is_empty());
    }
}

#[test]
fn dot_components_are_removed_for_new_paths() {
    for (target, expected) in [
        ("./docs", PathBuf::from("docs/one.txt")),
        (".", PathBuf::from("one.txt")),
    ] {
        let f = Fixture::new();
        let store = f.store();
        let item = f.resource(&store, "one", "one.txt", "unused");
        save_manifest(&f.0, &Manifest::empty()).unwrap();
        let mut provider = mock::MockSymlinkProvider::new();
        provider.add_file(&item.source_path);
        let report = project_links::link_item(
            &mut ProjectCtx {
                root: &f.0,
                provider: &mut provider,
            },
            &store,
            &req(&item.id, Some(target)),
            LinkOptions::default(),
        )
        .unwrap();
        assert_eq!(report.link_path, expected);
        assert_eq!(load_manifest(&f.0).unwrap().links[0].link_path, expected);
    }
}

#[test]
#[cfg(windows)]
fn real_broker_case_equivalent_existing_record_relinks() {
    let f = Fixture::new();
    let store = f.store();
    let item = f.resource(&store, "one", "one.txt", "config/sub");
    fs::create_dir_all(f.0.join("Config/Sub")).unwrap();
    let mut provider = default_provider();
    provider
        .create_symlink(
            &item.source_path,
            &f.0.join("Config/Sub/one.txt"),
            LinkKind::File,
        )
        .unwrap();
    let old = record(&item, "Config/Sub/one.txt");
    put(&f, old.clone());
    let report = project_links::link_item(
        &mut ProjectCtx {
            root: &f.0,
            provider: &mut *provider,
        },
        &store,
        &req(&item.id, None),
        LinkOptions::default(),
    )
    .unwrap();
    assert_eq!(report.outcome, CreateSymlinkOutcome::AlreadyCorrect);
    let links = load_manifest(&f.0).unwrap().links;
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].id, old.id);
    assert_eq!(links[0].created_at, old.created_at);
}

#[test]
#[cfg(windows)]
fn real_broker_broken_symlink_ancestor_case_status_and_clean() {
    for mode in [CleanMode::Default, CleanMode::Broken] {
        let f = Fixture::new();
        let store = f.store();
        let item = f.resource(&store, "broken_item", "broken.txt", "Config/Sub");
        fs::create_dir_all(f.0.join("Config/Sub")).unwrap();

        let mut provider = default_provider();
        let link_path = f.0.join("Config/Sub/broken.txt");
        provider
            .create_symlink(&item.source_path, &link_path, LinkKind::File)
            .unwrap();

        let record = record(&item, "Config/Sub/broken.txt");
        put(&f, record.clone());

        // Delete the real source file to create a broken symlink on disk
        fs::remove_file(&item.source_path).unwrap();

        let mut ctx = ProjectCtx {
            root: &f.0,
            provider: &mut *provider,
        };

        // 1. Verify status detects BrokenSymlink despite ancestor casing differences
        let status_report = project_links::status(&mut ctx).unwrap();
        assert_eq!(status_report.entries.len(), 1);
        assert!(
            matches!(status_report.entries[0].status, LinkStatus::BrokenSymlink { .. }),
            "expected BrokenSymlink, got {:?}",
            status_report.entries[0].status
        );

        // 2. Verify clean in this mode successfully removes the broken link
        let clean_report = project_links::clean(&mut ctx, CleanOptions::new(mode)).unwrap();
        assert_eq!(clean_report.removed.len(), 1);
        assert_eq!(clean_report.removed[0].record.id, record.id);
        assert!(link_path.symlink_metadata().is_err());

        // Manifest is now cleanly empty
        let loaded = load_manifest(&f.0).unwrap();
        assert!(loaded.links.is_empty());
    }
}

#[test]
fn foreign_item_and_harness_occupants_conflict_without_effects() {
    for harness in [false, true] {
        for group in [false, true] {
            for dry_run in [false, true] {
                let f = Fixture::new();
                let store = f.store();
                let item = f.resource(&store, "one", "one.txt", "new/deep");
                let mut old = record(&item, "new/deep/one.txt");
                if harness {
                    old.origin = LinkOrigin::Harness;
                    old.harness_name = Some("custom".into());
                } else {
                    old.item_id = "another-item".into();
                }
                put(&f, old);
                let before = fs::read(manifest_path(&f.0)).unwrap();
                let mut provider = mock::MockSymlinkProvider::new();
                provider.add_file(&item.source_path);
                let mut ctx = ProjectCtx {
                    root: &f.0,
                    provider: &mut provider,
                };
                let options = LinkOptions {
                    dry_run,
                    force: false,
                };
                let err = if group {
                    store.create_group("batch", None).unwrap();
                    store
                        .add_group_items("batch", std::slice::from_ref(&item.id))
                        .unwrap();
                    project_links::link_group(&mut ctx, &store, "batch", options).unwrap_err()
                } else {
                    project_links::link_item(&mut ctx, &store, &req(&item.id, None), options)
                        .unwrap_err()
                };
                assert!(matches!(err, Error::Conflict { .. }), "{err:?}");
                assert!(!f.0.join("new").exists());
                assert!(provider.mutations.is_empty());
                assert_eq!(fs::read(manifest_path(&f.0)).unwrap(), before);
            }
        }
    }
}

fn assert_manifest_error(f: &Fixture, error: &Error) {
    let path = manifest_path(&f.0);
    let rendered = error.to_string().replace('/', "\\");
    let expected = path.display().to_string().replace('/', "\\");
    assert!(rendered.contains(&expected), "{error}");
}

#[test]
fn manifest_business_validation_errors_include_manifest_path() {
    let f = Fixture::new();
    fs::create_dir_all(f.0.join(".agents")).unwrap();
    fs::write(manifest_path(&f.0), "schema_version = 999\n").unwrap();
    assert_manifest_error(&f, &load_manifest(&f.0).unwrap_err());
    let invalid = Manifest {
        schema_version: 999,
        links: vec![],
    };
    assert_manifest_error(&f, &save_manifest(&f.0, &invalid).unwrap_err());
    let store = f.store();
    let item = f.resource(&store, "one", "one.txt", "docs");
    let one = record(&item, "docs/one.txt");
    let mut two = one.clone();
    two.id = "other-id".into();
    let duplicate = Manifest {
        schema_version: 1,
        links: vec![one, two],
    };
    assert_manifest_error(&f, &save_manifest(&f.0, &duplicate).unwrap_err());
    let mut text = toml::to_string(&duplicate).unwrap();
    text.push('\n');
    fs::write(manifest_path(&f.0), text).unwrap();
    assert_manifest_error(&f, &load_manifest(&f.0).unwrap_err());
}

#[test]
fn manifest_parse_errors_keep_path_and_source() {
    let f = Fixture::new();
    fs::create_dir_all(f.0.join(".agents")).unwrap();
    fs::write(manifest_path(&f.0), "schema_version = [broken\n").unwrap();
    let error = load_manifest(&f.0).unwrap_err();
    assert_manifest_error(&f, &error);
    assert_eq!(error.path(), Some(manifest_path(&f.0).as_path()));
    assert!(error.source().is_some());
}

#[test]
fn manifest_directory_leaf_errors_include_path() {
    let f = Fixture::new();
    fs::create_dir_all(manifest_path(&f.0)).unwrap();
    assert_manifest_error(&f, &load_manifest(&f.0).unwrap_err());
    assert_manifest_error(&f, &save_manifest(&f.0, &Manifest::empty()).unwrap_err());
}

#[test]
#[cfg(windows)]
fn manifest_symlink_leaf_errors_include_path() {
    let f = Fixture::new();
    fs::create_dir_all(f.0.join(".agents")).unwrap();
    let source = f.0.join("external.toml");
    fs::write(&source, "schema_version = 1\n").unwrap();
    default_provider()
        .create_symlink(&source, &manifest_path(&f.0), LinkKind::File)
        .unwrap();
    assert_manifest_error(&f, &load_manifest(&f.0).unwrap_err());
    assert_manifest_error(&f, &save_manifest(&f.0, &Manifest::empty()).unwrap_err());
}

#[test]
#[cfg(windows)]
fn init_locked_gitignore_io_error_keeps_path_and_source() {
    use std::os::windows::fs::OpenOptionsExt;
    let f = Fixture::new();
    let path = f.0.join(".gitignore");
    fs::write(&path, "existing\n").unwrap();
    let _locked = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    let mut provider = mock::MockSymlinkProvider::new();
    let error = agent_linker::core::manifest::init_project_with_mappings(&f.0, &mut provider, &[])
        .unwrap_err();
    assert_eq!(error.path(), Some(path.as_path()));
    assert!(
        error.to_string().contains(&path.display().to_string()),
        "{error}"
    );
    assert!(error.source().is_some(), "{error:?}");
}

#[test]
fn cli_verbose_does_not_duplicate_symlink_error_and_preserves_parse_cause() {
    let f = Fixture::new();
    fs::write(f.0.join("CLAUDE.md"), "user file").unwrap();
    let plain = f.cli(&["init"]);
    assert!(!plain.status.success());
    let plain_text = String::from_utf8_lossy(&plain.stderr);
    assert!(plain_text.contains("CLAUDE.md"), "{plain_text}");
    assert!(!plain_text.contains("caused by:"), "{plain_text}");
    let verbose = f.cli(&["--verbose", "init"]);
    assert!(!verbose.status.success());
    let text = String::from_utf8_lossy(&verbose.stderr);
    let top = text
        .lines()
        .find_map(|line| line.strip_prefix("aglink: "))
        .unwrap();
    for cause in text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("caused by: "))
    {
        assert_ne!(cause, top, "{text}");
    }
    fs::create_dir_all(f.0.join(".agents")).unwrap();
    fs::write(manifest_path(&f.0), "schema_version = [broken\n").unwrap();
    let parse = f.cli(&["--verbose", "status"]);
    assert!(!parse.status.success());
    let parse_text = String::from_utf8_lossy(&parse.stderr);
    assert!(parse_text.contains("caused by:"), "{parse_text}");
    assert!(parse_text.contains("links.toml"), "{parse_text}");
}
