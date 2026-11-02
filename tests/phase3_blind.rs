//! Independent phase-three acceptance tests, authored from the public contract.
use agent_linker::core::symlink::{
    evaluate_create, evaluate_remove, CreateSymlinkOptions, CreateSymlinkOutcome, LinkStatus,
    RemoveSymlinkOutcome, SymlinkBackend, SymlinkErrorKind,
};
use agent_linker::core::{
    linkable::{LinkableItem, LinkableItemType},
    paths::{DbPathReason, DbPathResolution},
    project_links::{
        self, CleanMode, CleanOptions, LinkItemRequest, LinkOptions, ProjectCtx, UnlinkOptions,
    },
    registry::AddLinkableItem,
    store::Store,
};
use agent_linker::core::{
    manifest::{load_manifest, manifest_path, save_manifest, LinkOrigin, LinkRecord, Manifest},
    symlink::LinkKind,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "support/mock.rs"]
mod mock;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "aglink-phase3-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = root.with_file_name(format!(
            "{}-{}",
            root.file_name().unwrap().to_string_lossy(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn root(&self) -> &Path {
        &self.0
    }
    fn write_manifest(&self, text: &str) {
        fs::create_dir_all(self.0.join(".agents")).unwrap();
        fs::write(manifest_path(self.root()), text).unwrap();
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
    fn resource(
        &self,
        store: &Store,
        name: &str,
        basename: &str,
        target: &str,
        directory: bool,
    ) -> LinkableItem {
        let source_dir = self.0.join("sources").join(name);
        fs::create_dir_all(&source_dir).unwrap();
        let source = source_dir.join(basename);
        if directory {
            fs::create_dir(&source).unwrap();
        } else {
            fs::write(&source, name).unwrap();
        }
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
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn record(source: PathBuf, backend: &str) -> LinkRecord {
    LinkRecord {
        id: "opaque-link".into(),
        origin: LinkOrigin::Item,
        harness_name: None,
        item_id: "opaque-item".into(),
        item_name: "quote\"雪\nline".into(),
        source_path: source,
        link_path: PathBuf::from("docs/雪 asset.txt"),
        link_kind: LinkKind::File,
        provider_backend: backend.into(),
        created_by_command: "link \"雪\"".into(),
        created_at: -123,
        updated_at: i64::MAX,
    }
}

#[test]
fn toml_native_absolute_source_and_unknown_audit_backend_round_trip() {
    for backend in ["external-ln", "future-provider/v9", "custom \"quoted\" 雪"] {
        let f = Fixture::new();
        let expected = Manifest {
            schema_version: 1,
            links: vec![record(
                f.0.join("source '雪' dir").join("native.txt"),
                backend,
            )],
        };
        save_manifest(f.root(), &expected).unwrap();
        let loaded = load_manifest(f.root()).unwrap();
        assert_eq!(loaded, expected);
        assert_eq!(
            loaded.links[0].source_path.as_os_str(),
            expected.links[0].source_path.as_os_str()
        );
        let text = fs::read_to_string(manifest_path(f.root())).unwrap();
        assert!(!text.contains("scope ="));
        assert!(text.lines().any(|line| line.trim() == "created_at = -123"));
        assert!(text
            .lines()
            .any(|line| line.trim() == format!("updated_at = {}", i64::MAX)));
        save_manifest(f.root(), &loaded).unwrap();
        assert_eq!(load_manifest(f.root()).unwrap(), expected);
    }
}

#[test]
fn toml_accepts_legal_comments_literal_strings_and_mixed_timestamp_representations() {
    let f = Fixture::new();
    f.write_manifest(
        r#"# legal TOML, including literal strings and trailing comments
schema_version = 1 # current
[[links]]
id = 'item-one'
origin = 'item'
item_id = 'stable-id'
item_name = "hello\t雪"
source_path = 'relative source/file.txt'
link_path = 'docs/item.txt'
link_kind = 'file'
provider_backend = 'external-ln' # audit only
created_by_command = 'external'
created_at = -17
updated_at = 'unix:9223372036854775807'
"#,
    );
    let loaded = load_manifest(f.root()).unwrap();
    assert_eq!(loaded.links[0].created_at, -17);
    assert_eq!(loaded.links[0].updated_at, i64::MAX);
    assert_eq!(loaded.links[0].item_name, "hello\t雪");
    assert_eq!(loaded.links[0].provider_backend, "external-ln");
    save_manifest(f.root(), &loaded).unwrap();
    assert_eq!(load_manifest(f.root()).unwrap(), loaded);
    let text = fs::read_to_string(manifest_path(f.root())).unwrap();
    assert!(!text.contains("unix:"));
}

#[test]
fn legacy_registry_and_harness_inference_preserve_all_identity_and_audit_fields() {
    let f = Fixture::new();
    let entries: String = [
        ("registry", "item", "items/a"),
        ("claude", "harness", "CLAUDE.md"),
        ("custom", "harness", "CUSTOM.md"),
    ]
    .iter()
    .enumerate()
    .map(|(i, (framework, _, link))| {
        format!(
            r#"
[[links]]
id = 'link-{i}'
scope = 'project'
framework_name = '{framework}'
item_id = 'stable-{i}'
item_name = 'name-{i}'
source_path = 'AGENTS.md'
link_path = '{link}'
link_kind = 'file'
provider_backend = 'external-ln'
created_by_command = 'legacy-custom'
created_at = 'unix:-11'
updated_at = 22
"#
        )
    })
    .collect();
    f.write_manifest(&format!("schema_version = 1\n{entries}"));
    let loaded = load_manifest(f.root()).unwrap();
    assert_eq!(loaded.links.len(), 3);
    assert_eq!(loaded.links[0].origin, LinkOrigin::Item);
    assert_eq!(loaded.links[0].harness_name, None);
    for (i, r) in loaded.links.iter().enumerate() {
        assert_eq!(r.id, format!("link-{i}"));
        assert_eq!(r.item_id, format!("stable-{i}"));
        assert_eq!(r.provider_backend, "external-ln");
        assert_eq!(r.created_by_command, "legacy-custom");
        assert_eq!((r.created_at, r.updated_at), (-11, 22));
    }
    assert_eq!(loaded.links[1].origin, LinkOrigin::Harness);
    assert_eq!(loaded.links[1].harness_name.as_deref(), Some("claude"));
    assert_eq!(loaded.links[2].origin, LinkOrigin::Harness);
    assert_eq!(loaded.links[2].harness_name.as_deref(), Some("custom"));
    save_manifest(f.root(), &loaded).unwrap();
    assert_eq!(load_manifest(f.root()).unwrap(), loaded);
    let text = fs::read_to_string(manifest_path(f.root())).unwrap();
    assert!(!text.contains("scope ="));
    assert!(!text.contains("unix:"));
}

#[test]
fn malformed_toml_never_overwrites_the_original_manifest() {
    for text in [
        "schema_version = 1\nschema_version = 1\n",
        "schema_version = '1'\n",
        "schema_version = 1\n[[links]]\nid = 'unfinished\n",
        "schema_version = 2\n",
    ] {
        let f = Fixture::new();
        f.write_manifest(text);
        let before = fs::read(manifest_path(f.root())).unwrap();
        assert!(load_manifest(f.root()).is_err(), "{text}");
        assert_eq!(fs::read(manifest_path(f.root())).unwrap(), before);
    }
}

#[test]
fn pure_create_rules_exhaustively_cover_status_force_source_and_backend() {
    let source = Path::new("source");
    let link = Path::new("link");
    let statuses = [
        LinkStatus::Missing,
        LinkStatus::CorrectSymlink {
            target: source.into(),
        },
        LinkStatus::WrongSymlinkTarget {
            actual: "other".into(),
            expected: source.into(),
        },
        LinkStatus::BrokenSymlink {
            target: source.into(),
        },
        LinkStatus::ExistingRealFile,
        LinkStatus::ExistingRealDirectory,
        LinkStatus::UnsupportedFileType { path: link.into() },
    ];
    for backend in [
        SymlinkBackend::Std,
        SymlinkBackend::WindowsBroker,
        SymlinkBackend::WindowsStdFallback,
        SymlinkBackend::Mock,
    ] {
        for status in &statuses {
            for force in [false, true] {
                for source_valid in [false, true] {
                    let result = evaluate_create(
                        status,
                        source_valid,
                        source,
                        link,
                        backend,
                        CreateSymlinkOptions { force },
                    );
                    let expected = if !source_valid {
                        Err(SymlinkErrorKind::SourceNotFound)
                    } else {
                        match status {
                            LinkStatus::Missing => Ok(CreateSymlinkOutcome::Created),
                            LinkStatus::CorrectSymlink { .. } => {
                                Ok(CreateSymlinkOutcome::AlreadyCorrect)
                            }
                            LinkStatus::WrongSymlinkTarget { .. }
                            | LinkStatus::BrokenSymlink { .. }
                                if force =>
                            {
                                Ok(CreateSymlinkOutcome::ReplacedWrongSymlink)
                            }
                            LinkStatus::WrongSymlinkTarget { .. } => {
                                Err(SymlinkErrorKind::WrongSymlinkTarget)
                            }
                            LinkStatus::BrokenSymlink { .. } => {
                                Err(SymlinkErrorKind::SourceNotFound)
                            }
                            LinkStatus::ExistingRealFile => Err(SymlinkErrorKind::ExistingRealFile),
                            LinkStatus::ExistingRealDirectory => {
                                Err(SymlinkErrorKind::ExistingRealDirectory)
                            }
                            LinkStatus::UnsupportedFileType { .. } => {
                                Err(SymlinkErrorKind::UnsupportedLinkKind)
                            }
                        }
                    };
                    match (result, expected) {
                        (Ok(actual), Ok(expected)) => assert_eq!(actual, expected),
                        (Err(actual), Err(expected)) => {
                            assert_eq!(actual.kind(), expected, "{status:?}, force={force}, valid={source_valid}");
                            assert_eq!(actual.backend(), backend);
                            assert_eq!(actual.source(), Some(source));
                            assert_eq!(actual.link(), Some(link));
                        }
                        (actual, expected) => panic!("{status:?}, force={force}, valid={source_valid}: {actual:?} != {expected:?}"),
                    }
                }
            }
        }
    }
    assert!(!CreateSymlinkOptions::default().force);
}

#[test]
fn pure_remove_rules_preserve_real_paths_and_remove_every_symlink_status() {
    let link = Path::new("link");
    let statuses = [
        LinkStatus::Missing,
        LinkStatus::CorrectSymlink {
            target: "source".into(),
        },
        LinkStatus::WrongSymlinkTarget {
            actual: "other".into(),
            expected: "source".into(),
        },
        LinkStatus::BrokenSymlink {
            target: "source".into(),
        },
    ];
    for backend in [
        SymlinkBackend::Std,
        SymlinkBackend::WindowsBroker,
        SymlinkBackend::WindowsStdFallback,
        SymlinkBackend::Mock,
    ] {
        for status in &statuses {
            let expected = if matches!(status, LinkStatus::Missing) {
                RemoveSymlinkOutcome::Missing
            } else {
                RemoveSymlinkOutcome::Removed
            };
            assert_eq!(evaluate_remove(status, link, backend).unwrap(), expected);
        }
        for (status, kind) in [
            (
                LinkStatus::ExistingRealFile,
                SymlinkErrorKind::ExistingRealFile,
            ),
            (
                LinkStatus::ExistingRealDirectory,
                SymlinkErrorKind::ExistingRealDirectory,
            ),
            (
                LinkStatus::UnsupportedFileType { path: link.into() },
                SymlinkErrorKind::UnsupportedLinkKind,
            ),
        ] {
            let error = evaluate_remove(&status, link, backend).unwrap_err();
            assert_eq!(error.kind(), kind);
            assert_eq!(error.backend(), backend);
            assert_eq!(error.link(), Some(link));
        }
    }
}

fn tree(root: &Path) -> Vec<PathBuf> {
    fn walk(root: &Path, entries: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                walk(&path, entries);
            }
            entries.push(path);
        }
    }
    let mut entries = vec![];
    walk(root, &mut entries);
    entries.sort();
    entries
}
fn request(id: &str) -> LinkItemRequest {
    LinkItemRequest {
        identifier: id.into(),
        link_name_override: None,
        target_dir_override: None,
    }
}

#[test]
fn every_group_preflight_rejection_has_identical_dry_run_and_real_no_effects() {
    for scenario in [
        "missing-later-source",
        "real-file",
        "real-directory",
        "duplicate",
        "ancestor",
        "manifest-occupied",
    ] {
        for force in [false, true] {
            let f = Fixture::new();
            let store = f.store();
            let first = f.resource(&store, "alpha", "first.txt", "new/inner", false);
            let (basename, target, directory) = match scenario {
                "duplicate" => ("first.txt", "new/other", false),
                "ancestor" => ("nested.txt", "new/inner/first.txt", false),
                _ => ("second.txt", "new/inner", false),
            };
            let second = f.resource(&store, "beta", basename, target, directory);
            if scenario == "duplicate" {
                // Model a historical noncanonical destination that passes literal SQL uniqueness.
                rusqlite::Connection::open(f.resolution().path)
                    .unwrap()
                    .execute(
                        "UPDATE linkable_items SET target_dir = 'new/inner/.' WHERE id = ?1",
                        [&second.id],
                    )
                    .unwrap();
            }
            store.create_group("batch", None).unwrap();
            store
                .add_group_items("batch", &[first.id.clone(), second.id.clone()])
                .unwrap();
            save_manifest(f.root(), &Manifest::empty()).unwrap();
            let mut provider = mock::MockSymlinkProvider::new();
            provider.add_file(&first.source_path);
            provider.add_file(&second.source_path);
            match scenario {
                "missing-later-source" => {
                    fs::remove_file(&second.source_path).unwrap();
                    provider.entries.remove(&second.source_path);
                }
                "real-file" => provider.add_file(f.0.join("new/inner/second.txt")),
                "real-directory" => provider.add_dir(f.0.join("new/inner/second.txt")),

                "manifest-occupied" => {
                    let mut occupied = record(first.source_path.clone(), "external-ln");
                    occupied.link_path = "new/inner/second.txt".into();
                    save_manifest(
                        f.root(),
                        &Manifest {
                            schema_version: 1,
                            links: vec![occupied],
                        },
                    )
                    .unwrap();
                }
                _ => {}
            }
            let before_manifest = fs::read(manifest_path(f.root())).unwrap();
            let before_tree = tree(f.root());
            let before_entries = provider.entries.clone();
            for dry_run in [true, false] {
                let result = project_links::link_group(
                    &mut ProjectCtx {
                        root: f.root(),
                        provider: &mut provider,
                    },
                    &store,
                    "batch",
                    LinkOptions { dry_run, force },
                );
                assert!(
                    result.is_err(),
                    "scenario={scenario}, dry={dry_run}, force={force}"
                );
                assert_eq!(provider.entries, before_entries, "scenario={scenario}");
                assert!(provider.mutations.is_empty(), "scenario={scenario}");
                assert_eq!(
                    tree(f.root()),
                    before_tree,
                    "scenario={scenario}, dry={dry_run}"
                );
                assert_eq!(
                    fs::read(manifest_path(f.root())).unwrap(),
                    before_manifest,
                    "scenario={scenario}"
                );
            }
        }
    }
}

#[test]
fn group_manifest_commit_occurs_after_all_provider_mutations_and_unlink_preserves_harness() {
    let f = Fixture::new();
    let store = f.store();
    let first = f.resource(&store, "alpha", "one.txt", "new/inner", false);
    let second = f.resource(&store, "beta", "two.txt", "new/inner", false);
    store.create_group("batch", None).unwrap();
    store
        .add_group_items("batch", &[first.id.clone(), second.id.clone()])
        .unwrap();
    let mut harness = record(f.0.join("AGENTS.md"), "external-ln");
    harness.id = "harness-link".into();
    harness.origin = LinkOrigin::Harness;
    harness.harness_name = Some("custom".into());
    harness.item_id = first.id.clone();
    harness.link_path = "CUSTOM.md".into();
    save_manifest(
        f.root(),
        &Manifest {
            schema_version: 1,
            links: vec![harness.clone()],
        },
    )
    .unwrap();
    let mut provider = mock::MockSymlinkProvider::new();
    provider.add_file(&first.source_path);
    provider.add_file(&second.source_path);
    provider.add_file(&harness.source_path);
    provider.add_symlink(f.0.join("CUSTOM.md"), &harness.source_path, LinkKind::File);
    provider.observe_manifest = Some((
        manifest_path(f.root()),
        fs::read(manifest_path(f.root())).unwrap(),
    ));
    let preview = project_links::link_group(
        &mut ProjectCtx {
            root: f.root(),
            provider: &mut provider,
        },
        &store,
        "batch",
        LinkOptions {
            dry_run: true,
            force: false,
        },
    )
    .unwrap();
    assert_eq!(preview.reports.len(), 2);
    assert!(provider.mutations.is_empty());
    assert!(!f.0.join("new").exists());
    let report = project_links::link_group(
        &mut ProjectCtx {
            root: f.root(),
            provider: &mut provider,
        },
        &store,
        "batch",
        LinkOptions::default(),
    )
    .unwrap();
    assert_eq!(report.reports.len(), 2);
    assert_eq!(provider.mutations.len(), 2);
    assert_eq!(load_manifest(f.root()).unwrap().links.len(), 3);
    provider.observe_manifest = Some((
        manifest_path(f.root()),
        fs::read(manifest_path(f.root())).unwrap(),
    ));
    let removed = project_links::unlink_group(
        &mut ProjectCtx {
            root: f.root(),
            provider: &mut provider,
        },
        &store,
        "batch",
        UnlinkOptions::default(),
    )
    .unwrap();
    assert_eq!(removed.outcomes.len(), 2);
    assert_eq!(
        load_manifest(f.root()).unwrap().links,
        vec![harness.clone()]
    );
    project_links::unlink(
        &mut ProjectCtx {
            root: f.root(),
            provider: &mut provider,
        },
        None,
        UnlinkOptions::default(),
    )
    .unwrap();
    assert_eq!(load_manifest(f.root()).unwrap().links, vec![harness]);
}

#[test]
fn unknown_backend_is_audit_only_for_status_unlink_and_clean() {
    for operation in ["status", "unlink", "clean"] {
        let f = Fixture::new();
        let mut item = record(f.0.join("missing-source"), "future/external-ln");
        item.link_path = "docs/a.txt".into();
        save_manifest(
            f.root(),
            &Manifest {
                schema_version: 1,
                links: vec![item.clone()],
            },
        )
        .unwrap();
        let mut provider = mock::MockSymlinkProvider::new();
        provider.add_symlink(f.0.join(&item.link_path), &item.source_path, LinkKind::File);
        let mut ctx = ProjectCtx {
            root: f.root(),
            provider: &mut provider,
        };
        match operation {
            "status" => {
                let report = project_links::status(&mut ctx).unwrap();
                assert_eq!(report.entries.len(), 1);
                assert_eq!(
                    report.entries[0].record.provider_backend,
                    "future/external-ln"
                );
                assert_eq!(load_manifest(f.root()).unwrap().links, vec![item]);
                assert!(provider.mutations.is_empty());
            }
            "unlink" => {
                project_links::unlink(&mut ctx, Some(&item.item_id), UnlinkOptions::default())
                    .unwrap();
                assert!(load_manifest(f.root()).unwrap().links.is_empty());
                assert_eq!(provider.mutations.len(), 1);
            }
            "clean" => {
                project_links::clean(&mut ctx, CleanOptions::new(CleanMode::Broken)).unwrap();
                assert!(load_manifest(f.root()).unwrap().links.is_empty());
                assert_eq!(provider.mutations.len(), 1);
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn single_link_failed_preflight_creates_no_parent_directories() {
    for dry_run in [true, false] {
        let f = Fixture::new();
        let store = f.store();
        let item = f.resource(&store, "asset", "item.txt", "uncreated/deep", false);
        save_manifest(f.root(), &Manifest::empty()).unwrap();
        let mut provider = mock::MockSymlinkProvider::new();
        provider.add_file(&item.source_path);
        provider.add_file(f.0.join("uncreated/deep/item.txt"));
        let before = fs::read(manifest_path(f.root())).unwrap();
        assert!(project_links::link_item(
            &mut ProjectCtx {
                root: f.root(),
                provider: &mut provider
            },
            &store,
            &request(&item.id),
            LinkOptions {
                dry_run,
                force: true
            }
        )
        .is_err());
        assert!(!f.0.join("uncreated").exists());
        assert!(provider.mutations.is_empty());
        assert_eq!(fs::read(manifest_path(f.root())).unwrap(), before);
    }
}

#[test]
fn cli_argument_normalization_and_usage_failures_are_consistent_across_output_modes() {
    for flags in [&[][..], &["--quiet"][..], &["--verbose"][..]] {
        let f = Fixture::new();
        for args in [
            &["link"][..],
            &["unlink"][..],
            &["unlink", "asset", "--all"][..],
            &["clean", "--broken", "--missing-source"][..],
            &["link", "--group", "batch", "--as", "alias"][..],
        ] {
            let output = std::process::Command::new(env!("CARGO_BIN_EXE_aglink"))
                .current_dir(f.root())
                .env("AGLINK_DATA_DIR", f.0.join("absent-data"))
                .args(flags)
                .args(args)
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(1),
                "{flags:?} {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                !output.stderr.is_empty(),
                "invalid arguments must remain visible with {flags:?}"
            );
            assert!(!f.0.join("absent-data").exists());
            assert!(!f.0.join(".agents").exists());
        }
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_aglink"))
            .current_dir(f.root())
            .args(flags)
            .arg("--help")
            .output()
            .unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("harness"));
        assert!(!help.contains("framework"));
    }
}

#[test]
#[cfg_attr(
    windows,
    ignore = "requires an available Windows Broker for real ancestor links"
)]
fn group_real_symlink_ancestor_preflight_rejects_dry_and_real_without_effects() {
    let f = Fixture::new();
    let store = f.store();
    let first = f.resource(&store, "alpha", "first.txt", "new/inner", false);
    let second = f.resource(&store, "beta", "second.txt", "new/inner", false);
    store.create_group("batch", None).unwrap();
    store
        .add_group_items("batch", &[first.id.clone(), second.id.clone()])
        .unwrap();
    save_manifest(f.root(), &Manifest::empty()).unwrap();
    let outside = f.0.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("sentinel"), "untouched").unwrap();
    agent_linker::core::symlink::default_provider()
        .create_symlink(&outside, &f.0.join("new"), LinkKind::Directory)
        .unwrap();
    let mut provider = mock::MockSymlinkProvider::new();
    provider.add_file(&first.source_path);
    provider.add_file(&second.source_path);
    let before = fs::read(manifest_path(f.root())).unwrap();
    let entries = tree(f.root());
    for force in [false, true] {
        for dry_run in [true, false] {
            assert!(project_links::link_group(
                &mut ProjectCtx {
                    root: f.root(),
                    provider: &mut provider
                },
                &store,
                "batch",
                LinkOptions { dry_run, force }
            )
            .is_err());
            assert!(provider.mutations.is_empty());
            assert_eq!(fs::read(manifest_path(f.root())).unwrap(), before);
            assert_eq!(tree(f.root()), entries);
            assert_eq!(
                fs::read_to_string(outside.join("sentinel")).unwrap(),
                "untouched"
            );
        }
    }
}

#[test]
fn public_error_conversions_preserve_sources_paths_and_system_codes() {
    use agent_linker::core::{error::Error, symlink::SymlinkError};
    use std::error::Error as _;
    let path = Path::new("diagnostic/path");
    let io = Error::io(path, std::io::Error::from_raw_os_error(5));
    assert_eq!(io.path(), Some(path));
    assert!(io
        .source()
        .unwrap()
        .downcast_ref::<std::io::Error>()
        .is_some());
    let denied: Error =
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission cause").into();
    assert!(matches!(denied, Error::PermissionDenied { .. }));
    assert_eq!(denied.source().unwrap().to_string(), "permission cause");
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    connection.execute_batch("CREATE TABLE values_to_test (value TEXT UNIQUE); INSERT INTO values_to_test VALUES ('duplicate');").unwrap();
    let failure = connection
        .execute("INSERT INTO values_to_test VALUES ('duplicate')", [])
        .unwrap_err();
    let conflict: Error = failure.into();
    assert!(matches!(conflict, Error::Conflict { .. }));
    assert!(conflict
        .source()
        .unwrap()
        .downcast_ref::<rusqlite::Error>()
        .is_some());
    let symlink = SymlinkError::new(
        SymlinkErrorKind::PermissionDenied,
        SymlinkBackend::WindowsBroker,
    )
    .with_source("source")
    .with_link("link")
    .with_system_code(Some(5))
    .with_cause(std::io::Error::from_raw_os_error(5));
    assert_eq!(symlink.system_code(), Some(5));
    let top = Error::Symlink(symlink);
    let middle = top.source().unwrap();
    assert!(middle.downcast_ref::<SymlinkError>().is_some());
    assert_eq!(
        middle
            .source()
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(5)
    );
    let f = Fixture::new();
    f.write_manifest("schema_version = [broken\n");
    let parse = load_manifest(f.root()).unwrap_err();
    assert!(matches!(parse, Error::Parse { .. }));
    assert_eq!(parse.path(), Some(manifest_path(f.root()).as_path()));
    assert!(parse.source().is_some());
}

#[test]
fn cli_parse_error_keeps_quiet_error_visible_and_verbose_adds_source_chain() {
    let f = Fixture::new();
    f.write_manifest("schema_version = [broken\n");
    let before = fs::read(manifest_path(f.root())).unwrap();
    for flags in [&[][..], &["--quiet"][..], &["--verbose"][..]] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_aglink"))
            .current_dir(f.root())
            .env("AGLINK_DATA_DIR", f.0.join("absent-data"))
            .args(flags)
            .arg("status")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let text = String::from_utf8_lossy(&output.stderr);
        assert!(!text.is_empty());
        if flags == ["--verbose"] {
            assert!(text.contains("caused by:"), "{text}");
        } else {
            assert!(!text.contains("caused by:"), "{text}");
        }
        assert_eq!(fs::read(manifest_path(f.root())).unwrap(), before);
        assert!(!f.0.join("absent-data").exists());
    }
}

#[test]
fn doctor_reports_five_checks_and_never_creates_missing_database() {
    use agent_linker::core::doctor::doctor_project;
    let f = Fixture::new();
    save_manifest(f.root(), &Manifest::empty()).unwrap();
    let before = fs::read(manifest_path(f.root())).unwrap();
    for verbose in [false, true] {
        let report = doctor_project(f.root(), &f.resolution(), verbose);
        assert!(!report.ok);
        for name in [
            "database",
            "manifest",
            "harness-adapter",
            "symlink-backend",
            "windows-broker",
        ] {
            assert_eq!(
                report
                    .checks
                    .iter()
                    .filter(|check| check.name == name)
                    .count(),
                1,
                "{name}: {:?}",
                report.checks
            );
        }
        assert!(
            report
                .checks
                .iter()
                .find(|check| check.name == "manifest")
                .unwrap()
                .ok
        );
        assert!(
            !report
                .checks
                .iter()
                .find(|check| check.name == "database")
                .unwrap()
                .ok
        );
        assert!(!f.resolution().path.parent().unwrap().exists());
        assert_eq!(fs::read(manifest_path(f.root())).unwrap(), before);
    }
}

#[test]
fn injected_replacement_failure_preserves_original_link_and_manifest() {
    let f = Fixture::new();
    let store = f.store();
    let item = f.resource(&store, "asset", "item.txt", "docs", false);
    save_manifest(f.root(), &Manifest::empty()).unwrap();
    let before = fs::read(manifest_path(f.root())).unwrap();
    let mut provider = mock::MockSymlinkProvider::new();
    provider.add_file(&item.source_path);
    provider.add_symlink(
        f.0.join("docs/item.txt"),
        f.0.join("old-source"),
        LinkKind::File,
    );
    let old_entries = provider.entries.clone();
    provider.fail_next_replacement();
    let result = project_links::link_item(
        &mut ProjectCtx {
            root: f.root(),
            provider: &mut provider,
        },
        &store,
        &request(&item.id),
        LinkOptions {
            dry_run: false,
            force: true,
        },
    );
    assert!(result.is_err());
    assert_eq!(provider.entries, old_entries);
    assert!(provider.mutations.is_empty());
    assert_eq!(fs::read(manifest_path(f.root())).unwrap(), before);
}

#[test]
fn ordinary_link_unlink_and_clean_commit_after_all_mutations_and_dry_run_never_saves() {
    for operation in ["link", "unlink", "clean"] {
        let f = Fixture::new();
        let store = f.store();
        let item = f.resource(&store, "asset", "item.txt", "docs", false);
        let mut provider = mock::MockSymlinkProvider::new();
        let records = if operation == "link" {
            provider.add_file(&item.source_path);
            vec![]
        } else {
            if operation == "unlink" {
                provider.add_file(&item.source_path);
            } else {
                fs::remove_file(&item.source_path).unwrap();
            }
            ["one.txt", "two.txt"]
                .into_iter()
                .enumerate()
                .map(|(i, name)| {
                    let mut r = record(item.source_path.clone(), "external-ln");
                    r.id = format!("record-{i}");
                    r.item_id = item.id.clone();
                    r.item_name = item.name.clone();
                    r.link_path = PathBuf::from("docs").join(name);
                    provider.add_symlink(f.0.join(&r.link_path), &r.source_path, LinkKind::File);
                    r
                })
                .collect()
        };
        save_manifest(
            f.root(),
            &Manifest {
                schema_version: 1,
                links: records,
            },
        )
        .unwrap();
        let before = fs::read(manifest_path(f.root())).unwrap();
        let before_entries = provider.entries.clone();
        let before_tree = tree(f.root());
        provider.observe_manifest = Some((manifest_path(f.root()), before.clone()));
        for dry_run in [true, false] {
            let mut ctx = ProjectCtx {
                root: f.root(),
                provider: &mut provider,
            };
            match operation {
                "link" => {
                    project_links::link_item(
                        &mut ctx,
                        &store,
                        &request(&item.id),
                        LinkOptions {
                            dry_run,
                            force: false,
                        },
                    )
                    .unwrap();
                }
                "unlink" => {
                    project_links::unlink(&mut ctx, None, UnlinkOptions { dry_run }).unwrap();
                }
                "clean" => {
                    project_links::clean(
                        &mut ctx,
                        CleanOptions {
                            mode: CleanMode::Broken,
                            dry_run,
                        },
                    )
                    .unwrap();
                }
                _ => unreachable!(),
            }
            if dry_run {
                assert!(provider.mutations.is_empty());
                assert_eq!(provider.entries, before_entries);
                assert_eq!(tree(f.root()), before_tree);
                assert_eq!(fs::read(manifest_path(f.root())).unwrap(), before);
            }
        }
        assert_eq!(
            provider.mutations.len(),
            if operation == "link" { 1 } else { 2 }
        );
        assert_eq!(
            load_manifest(f.root()).unwrap().links.len(),
            if operation == "link" { 1 } else { 0 }
        );
    }
}
