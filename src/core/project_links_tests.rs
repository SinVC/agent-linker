use super::{CleanMode, CleanOptions, LinkItemRequest, LinkOptions, ProjectCtx, UnlinkOptions};

use crate::core::{
    linkable::LinkableItemType,
    manifest::{load_manifest, save_manifest, LinkOrigin},
    paths::{DbPathReason, DbPathResolution},
    registry::AddLinkableItem,
    store::Store,
    symlink::{CreateSymlinkOutcome, LinkKind, LinkStatus, MockEntry, MockSymlinkProvider},
    test_support::TestDir,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn database(temp_dir: &TestDir) -> DbPathResolution {
    let resolution = DbPathResolution {
        path: temp_dir.path().join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    Store::migrate(&resolution).unwrap();
    resolution
}

// Compatibility helpers are confined to tests; every operation enters via ProjectCtx.
fn link_project_with_provider_and_db(
    root: &Path,
    provider: &mut MockSymlinkProvider,
    db: &DbPathResolution,
    request: LinkItemRequest,
) -> Result<super::LinkItemReport, crate::core::error::Error> {
    link_project_with_provider_and_db_with_options(
        root,
        provider,
        db,
        request,
        LinkOptions::default(),
    )
}
fn link_project_with_provider_and_db_with_options(
    root: &Path,
    provider: &mut MockSymlinkProvider,
    db: &DbPathResolution,
    request: LinkItemRequest,
    options: LinkOptions,
) -> Result<super::LinkItemReport, crate::core::error::Error> {
    let store = Store::open_ro(db)?;
    super::link_item(
        &mut ProjectCtx { root, provider },
        &store,
        &request,
        options,
    )
}
fn link_group_project_with_provider_and_db(
    root: &Path,
    provider: &mut MockSymlinkProvider,
    db: &DbPathResolution,
    group: &str,
) -> Result<super::LinkGroupReport, crate::core::error::Error> {
    link_group_project_with_provider_and_db_with_options(
        root,
        provider,
        db,
        group,
        LinkOptions::default(),
    )
}
fn link_group_project_with_provider_and_db_with_options(
    root: &Path,
    provider: &mut MockSymlinkProvider,
    db: &DbPathResolution,
    group: &str,
    options: LinkOptions,
) -> Result<super::LinkGroupReport, crate::core::error::Error> {
    let store = Store::open_ro(db)?;
    super::link_group(&mut ProjectCtx { root, provider }, &store, group, options)
}
fn status_project_with_provider(
    root: &Path,
    provider: &mut MockSymlinkProvider,
) -> Result<super::StatusReport, crate::core::error::Error> {
    super::status(&mut ProjectCtx { root, provider })
}
fn unlink_project_with_provider(
    root: &Path,
    provider: &mut MockSymlinkProvider,
    identifier: Option<String>,
) -> Result<super::UnlinkReport, crate::core::error::Error> {
    unlink_project_with_provider_with_options(root, provider, identifier, UnlinkOptions::default())
}
fn unlink_project_with_provider_with_options(
    root: &Path,
    provider: &mut MockSymlinkProvider,
    identifier: Option<String>,
    options: UnlinkOptions,
) -> Result<super::UnlinkReport, crate::core::error::Error> {
    super::unlink(
        &mut ProjectCtx { root, provider },
        identifier.as_deref(),
        options,
    )
}
fn unlink_group_project_with_provider_and_db(
    root: &Path,
    provider: &mut MockSymlinkProvider,
    db: &DbPathResolution,
    group: &str,
) -> Result<super::UnlinkReport, crate::core::error::Error> {
    unlink_group_project_with_provider_and_db_with_options(
        root,
        provider,
        db,
        group,
        UnlinkOptions::default(),
    )
}
fn unlink_group_project_with_provider_and_db_with_options(
    root: &Path,
    provider: &mut MockSymlinkProvider,
    db: &DbPathResolution,
    group: &str,
    options: UnlinkOptions,
) -> Result<super::UnlinkReport, crate::core::error::Error> {
    let store = Store::open_ro(db)?;
    super::unlink_group(&mut ProjectCtx { root, provider }, &store, group, options)
}
fn clean_project_with_provider(
    root: &Path,
    provider: &mut MockSymlinkProvider,
    mode: CleanMode,
) -> Result<super::CleanReport, crate::core::error::Error> {
    super::clean(&mut ProjectCtx { root, provider }, CleanOptions::new(mode))
}

#[test]
#[cfg_attr(
    windows,
    ignore = "requires running WinSymlinksBroker; run cargo test --locked -- --ignored --test-threads=1"
)]
fn symlink_parent_escape_is_rejected_by_every_project_operation() {
    for hijacked in [".agents", "controlled", "relative-source"] {
        let project = TestDir::new("project-escape");
        let outside = TestDir::new("outside-escape");
        let data = TestDir::new("escape-db");
        let resolution = database(&data);
        let source = outside.path().join("source.txt");
        fs::write(&source, "external source").unwrap();
        let item = Store::open_rw(&resolution)
            .unwrap()
            .add_item(AddLinkableItem {
                name: "asset".to_string(),
                alias: None,
                item_type: LinkableItemType::Resource,
                source_path: source,
                default_target_dir: Some(PathBuf::from("controlled")),
                description: None,
            })
            .unwrap();
        let relative_link = Path::new("controlled/asset");
        let mut record = super::link_record_from_item(
            &item,
            relative_link,
            crate::core::symlink::SymlinkBackend::Mock,
            "test:asset",
        );
        if hijacked == "relative-source" {
            record.source_path = PathBuf::from("relative-source/source.txt");
        }
        let manifest = crate::core::manifest::Manifest {
            schema_version: 1,
            links: vec![record],
        };
        crate::core::manifest::save_manifest(project.path(), &manifest).unwrap();
        let original = fs::read(project.path().join(".agents/links.toml")).unwrap();
        fs::write(outside.path().join("links.toml"), &original).unwrap();
        fs::write(outside.path().join("asset"), "external leaf").unwrap();
        if hijacked == ".agents" {
            fs::rename(
                project.path().join(".agents"),
                project.path().join(".agents-original"),
            )
            .unwrap();
        }
        crate::core::test_support::directory_symlink(
            outside.path(),
            &project.path().join(hijacked),
        );
        let mut provider = MockSymlinkProvider::new();
        provider.add_dir(project.path());
        provider.add_dir(project.path().join("controlled"));
        provider.add_file(&item.source_path);
        let absolute_link = project.path().join(relative_link);
        provider.add_symlink(&absolute_link, &item.source_path, LinkKind::File);
        let original_entry = provider.entry(&absolute_link).cloned();
        assert!(
            crate::core::manifest::load_manifest(project.path()).is_err(),
            "load: {hijacked}"
        );
        assert!(
            crate::core::manifest::save_manifest(project.path(), &manifest).is_err(),
            "save: {hijacked}"
        );
        assert!(
            status_project_with_provider(project.path(), &mut provider).is_err(),
            "status: {hijacked}"
        );
        assert!(
            link_project_with_provider_and_db(
                project.path(),
                &mut provider,
                &resolution,
                LinkItemRequest {
                    identifier: "asset".to_string(),
                    link_name_override: None,
                    target_dir_override: None,
                }
            )
            .is_err(),
            "link: {hijacked}"
        );
        assert!(
            unlink_project_with_provider(project.path(), &mut provider, None).is_err(),
            "unlink: {hijacked}"
        );
        assert!(
            clean_project_with_provider(project.path(), &mut provider, CleanMode::Default).is_err(),
            "clean: {hijacked}"
        );
        assert_eq!(provider.entry(&absolute_link), original_entry.as_ref());
        assert_eq!(
            fs::read(outside.path().join("links.toml")).unwrap(),
            original
        );
        assert_eq!(
            fs::read_to_string(outside.path().join("asset")).unwrap(),
            "external leaf"
        );
        assert_eq!(
            fs::read_to_string(outside.path().join("source.txt")).unwrap(),
            "external source"
        );
    }

    // The manifest leaf is a real-file control boundary, unlike managed link leaves.
    let project = TestDir::new("manifest-leaf-symlink");
    let outside = TestDir::new("outside-manifest-leaf");
    fs::create_dir(project.path().join(".agents")).unwrap();
    let external_manifest = outside.path().join("links.toml");
    fs::write(&external_manifest, "schema_version = 1\n").unwrap();
    let mut filesystem_provider = crate::core::symlink::default_provider();
    filesystem_provider
        .create_symlink(
            &external_manifest,
            &project.path().join(".agents/links.toml"),
            LinkKind::File,
        )
        .unwrap();
    assert!(crate::core::manifest::load_manifest(project.path()).is_err());
    assert!(crate::core::manifest::save_manifest(
        project.path(),
        &crate::core::manifest::Manifest::empty()
    )
    .is_err());
    assert_eq!(
        fs::read_to_string(&external_manifest).unwrap(),
        "schema_version = 1\n"
    );
}

fn seed_skill_source(project: &TestDir) -> PathBuf {
    let source = project.path().join("writer-skill");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("SKILL.md"), "write\n").unwrap();
    source
}

fn seed_provider(project_root: &Path, source: &Path) -> MockSymlinkProvider {
    let mut provider = MockSymlinkProvider::new();
    provider.add_dir(project_root);
    provider.add_dir(project_root.join(".agents"));
    provider.add_dir(project_root.join(".agents").join("skills"));
    provider.add_dir(crate::core::test_support::canonical_path(source));
    provider
}

#[test]
fn link_status_and_unlink_round_trip_for_skill() {
    let temp_dir = TestDir::new("project-links");
    fs::create_dir(temp_dir.path().join(".agents")).unwrap();
    fs::create_dir(temp_dir.path().join(".agents").join("skills")).unwrap();
    let source = seed_skill_source(&temp_dir);
    let resolution = database(&temp_dir);
    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "writer".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();

    let mut provider = seed_provider(temp_dir.path(), &source);
    let report = link_project_with_provider_and_db(
        temp_dir.path(),
        &mut provider,
        &resolution,
        LinkItemRequest {
            identifier: "writer".to_string(),
            link_name_override: None,
            target_dir_override: None,
        },
    )
    .unwrap();

    assert_eq!(report.outcome, CreateSymlinkOutcome::Created);
    assert_eq!(
        provider.entry(
            &temp_dir
                .path()
                .join(".agents")
                .join("skills")
                .join("writer")
        ),
        Some(&MockEntry::Symlink {
            target: crate::core::test_support::canonical_path(&source),
            kind: LinkKind::Directory
        })
    );
    assert_eq!(load_manifest(temp_dir.path()).unwrap().links.len(), 1);

    let status = status_project_with_provider(temp_dir.path(), &mut provider).unwrap();
    assert!(matches!(
        status.entries[0].status,
        LinkStatus::CorrectSymlink { .. }
    ));

    let unlink =
        unlink_project_with_provider(temp_dir.path(), &mut provider, Some("writer".to_string()))
            .unwrap();
    assert_eq!(unlink.outcomes.len(), 1);
    assert_eq!(
        provider.entry(
            &temp_dir
                .path()
                .join(".agents")
                .join("skills")
                .join("writer")
        ),
        None
    );
    assert!(load_manifest(temp_dir.path()).unwrap().links.is_empty());
}

#[test]
fn link_dry_run_reports_without_creating_link_or_manifest_record() {
    let temp_dir = TestDir::new("link-dry-run");
    fs::create_dir(temp_dir.path().join(".agents")).unwrap();
    fs::create_dir(temp_dir.path().join(".agents").join("skills")).unwrap();
    let source = seed_skill_source(&temp_dir);
    let resolution = database(&temp_dir);
    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "writer".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();

    let writer = rusqlite::Connection::open(&resolution.path).unwrap();
    writer.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let mut provider = seed_provider(temp_dir.path(), &source);
    let report = link_project_with_provider_and_db_with_options(
        temp_dir.path(),
        &mut provider,
        &resolution,
        LinkItemRequest {
            identifier: "writer".to_string(),
            link_name_override: None,
            target_dir_override: None,
        },
        LinkOptions {
            dry_run: true,
            force: false,
        },
    )
    .unwrap();

    assert!(report.dry_run);
    assert_eq!(report.outcome, CreateSymlinkOutcome::Created);
    assert_eq!(
        provider.entry(
            &temp_dir
                .path()
                .join(".agents")
                .join("skills")
                .join("writer")
        ),
        None
    );
    assert!(load_manifest(temp_dir.path()).unwrap().links.is_empty());
    writer.execute_batch("ROLLBACK;").unwrap();
}

#[test]
fn link_force_replaces_wrong_symlink_only_through_project_flow() {
    let temp_dir = TestDir::new("link-force");
    fs::create_dir(temp_dir.path().join(".agents")).unwrap();
    fs::create_dir(temp_dir.path().join(".agents").join("skills")).unwrap();
    let source = seed_skill_source(&temp_dir);
    let other = temp_dir.path().join("other-skill");
    fs::create_dir(&other).unwrap();
    fs::write(other.join("SKILL.md"), "other\n").unwrap();
    let resolution = database(&temp_dir);
    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "writer".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();

    let mut provider = seed_provider(temp_dir.path(), &source);
    provider.add_dir(crate::core::test_support::canonical_path(&other));
    provider.add_symlink(
        temp_dir
            .path()
            .join(".agents")
            .join("skills")
            .join("writer"),
        crate::core::test_support::canonical_path(&other),
        LinkKind::Directory,
    );

    let without_force = link_project_with_provider_and_db(
        temp_dir.path(),
        &mut provider,
        &resolution,
        LinkItemRequest {
            identifier: "writer".to_string(),
            link_name_override: None,
            target_dir_override: None,
        },
    )
    .unwrap_err();
    assert!(without_force.to_string().contains("wrong_symlink_target"));

    let report = link_project_with_provider_and_db_with_options(
        temp_dir.path(),
        &mut provider,
        &resolution,
        LinkItemRequest {
            identifier: "writer".to_string(),
            link_name_override: None,
            target_dir_override: None,
        },
        LinkOptions {
            dry_run: false,
            force: true,
        },
    )
    .unwrap();

    assert_eq!(report.outcome, CreateSymlinkOutcome::ReplacedWrongSymlink);
    assert_eq!(
        provider.entry(
            &temp_dir
                .path()
                .join(".agents")
                .join("skills")
                .join("writer")
        ),
        Some(&MockEntry::Symlink {
            target: crate::core::test_support::canonical_path(&source),
            kind: LinkKind::Directory
        })
    );
}

#[test]
fn link_as_and_resource_target_dir_override_do_not_change_registry_defaults() {
    let temp_dir = TestDir::new("resource-links");
    let source = temp_dir.path().join("notes.md");
    fs::write(&source, "notes\n").unwrap();
    let resolution = database(&temp_dir);
    let item = Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Resource,
            name: "notes".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: Some(PathBuf::from(".agents").join("resources")),
            description: None,
        })
        .unwrap();

    let mut provider = MockSymlinkProvider::new();
    provider.add_dir(temp_dir.path());
    provider.add_dir(temp_dir.path().join("docs"));
    provider.add_file(crate::core::test_support::canonical_path(&source));

    let report = link_project_with_provider_and_db(
        temp_dir.path(),
        &mut provider,
        &resolution,
        LinkItemRequest {
            identifier: "notes".to_string(),
            link_name_override: Some("project-notes.md".to_string()),
            target_dir_override: Some(PathBuf::from("docs")),
        },
    )
    .unwrap();

    assert_eq!(
        report.link_path,
        PathBuf::from("docs").join("project-notes.md")
    );
    assert_eq!(
        item.default_project_link_path().unwrap(),
        PathBuf::from(".agents").join("resources").join("notes.md")
    );
}

#[test]
fn group_link_and_unlink_reuse_project_link_flow() {
    let temp_dir = TestDir::new("group-links");
    fs::create_dir(temp_dir.path().join(".agents")).unwrap();
    fs::create_dir(temp_dir.path().join(".agents").join("skills")).unwrap();
    let skill_source = seed_skill_source(&temp_dir);
    let resource_source = temp_dir.path().join("notes.md");
    fs::write(&resource_source, "notes\n").unwrap();
    let resolution = database(&temp_dir);

    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "writer".to_string(),
            alias: None,
            source_path: skill_source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();
    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Resource,
            name: "notes".to_string(),
            alias: None,
            source_path: resource_source.clone(),
            default_target_dir: Some(PathBuf::from("docs")),
            description: None,
        })
        .unwrap();
    Store::open_rw(&resolution)
        .unwrap()
        .create_group("daily", None)
        .unwrap();
    Store::open_rw(&resolution)
        .unwrap()
        .add_group_items("daily", &["writer".to_string(), "notes".to_string()])
        .unwrap();

    let mut provider = seed_provider(temp_dir.path(), &skill_source);
    provider.add_dir(temp_dir.path().join("docs"));
    provider.add_file(crate::core::test_support::canonical_path(&resource_source));

    let writer = rusqlite::Connection::open(&resolution.path).unwrap();
    writer.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let preview = link_group_project_with_provider_and_db_with_options(
        temp_dir.path(),
        &mut provider,
        &resolution,
        "daily",
        LinkOptions {
            dry_run: true,
            force: false,
        },
    )
    .unwrap();
    assert_eq!(preview.reports.len(), 2);
    let link_report = link_group_project_with_provider_and_db(
        temp_dir.path(),
        &mut provider,
        &resolution,
        "daily",
    )
    .unwrap();
    assert_eq!(link_report.reports.len(), 2);
    assert_eq!(load_manifest(temp_dir.path()).unwrap().links.len(), 2);

    let preview = unlink_group_project_with_provider_and_db_with_options(
        temp_dir.path(),
        &mut provider,
        &resolution,
        "daily",
        UnlinkOptions { dry_run: true },
    )
    .unwrap();
    assert_eq!(preview.outcomes.len(), 2);
    let unlink_report = unlink_group_project_with_provider_and_db(
        temp_dir.path(),
        &mut provider,
        &resolution,
        "daily",
    )
    .unwrap();
    assert_eq!(unlink_report.outcomes.len(), 2);
    assert!(load_manifest(temp_dir.path()).unwrap().links.is_empty());
    writer.execute_batch("ROLLBACK;").unwrap();
}

#[test]
fn unlink_refuses_real_file_at_managed_path() {
    let temp_dir = TestDir::new("unlink-real-file");
    fs::create_dir(temp_dir.path().join(".agents")).unwrap();
    fs::create_dir(temp_dir.path().join(".agents").join("skills")).unwrap();
    let source = seed_skill_source(&temp_dir);
    let resolution = database(&temp_dir);
    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "writer".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();

    let mut provider = seed_provider(temp_dir.path(), &source);
    link_project_with_provider_and_db(
        temp_dir.path(),
        &mut provider,
        &resolution,
        LinkItemRequest {
            identifier: "writer".to_string(),
            link_name_override: None,
            target_dir_override: None,
        },
    )
    .unwrap();
    provider.add_file(
        temp_dir
            .path()
            .join(".agents")
            .join("skills")
            .join("writer"),
    );

    let error =
        unlink_project_with_provider(temp_dir.path(), &mut provider, Some("writer".to_string()))
            .unwrap_err();

    assert!(matches!(error, crate::core::Error::Symlink(_)));
    assert_eq!(load_manifest(temp_dir.path()).unwrap().links.len(), 1);
}

#[test]
fn unlink_dry_run_keeps_symlink_and_manifest_record() {
    let temp_dir = TestDir::new("unlink-dry-run");
    fs::create_dir(temp_dir.path().join(".agents")).unwrap();
    fs::create_dir(temp_dir.path().join(".agents").join("skills")).unwrap();
    let source = seed_skill_source(&temp_dir);
    let resolution = database(&temp_dir);
    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "writer".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();

    let mut provider = seed_provider(temp_dir.path(), &source);
    link_project_with_provider_and_db(
        temp_dir.path(),
        &mut provider,
        &resolution,
        LinkItemRequest {
            identifier: "writer".to_string(),
            link_name_override: None,
            target_dir_override: None,
        },
    )
    .unwrap();

    let report = unlink_project_with_provider_with_options(
        temp_dir.path(),
        &mut provider,
        Some("writer".to_string()),
        UnlinkOptions { dry_run: true },
    )
    .unwrap();

    assert!(report.dry_run);
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(load_manifest(temp_dir.path()).unwrap().links.len(), 1);
    assert!(provider
        .entry(
            &temp_dir
                .path()
                .join(".agents")
                .join("skills")
                .join("writer")
        )
        .is_some());
}

#[test]
fn clean_removes_only_manifest_managed_stale_symlinks() {
    let temp_dir = TestDir::new("clean");
    fs::create_dir(temp_dir.path().join(".agents")).unwrap();
    fs::create_dir(temp_dir.path().join(".agents").join("skills")).unwrap();
    let source = seed_skill_source(&temp_dir);
    let resolution = database(&temp_dir);
    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "writer".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();

    let mut provider = seed_provider(temp_dir.path(), &source);
    link_project_with_provider_and_db(
        temp_dir.path(),
        &mut provider,
        &resolution,
        LinkItemRequest {
            identifier: "writer".to_string(),
            link_name_override: None,
            target_dir_override: None,
        },
    )
    .unwrap();
    let mut clean_provider = MockSymlinkProvider::new();
    clean_provider.add_dir(temp_dir.path());
    clean_provider.add_dir(temp_dir.path().join(".agents"));
    clean_provider.add_dir(temp_dir.path().join(".agents").join("skills"));
    clean_provider.add_symlink(
        temp_dir
            .path()
            .join(".agents")
            .join("skills")
            .join("writer"),
        crate::core::test_support::canonical_path(&source),
        LinkKind::Directory,
    );
    clean_provider.add_symlink(
        temp_dir
            .path()
            .join(".agents")
            .join("skills")
            .join("unmanaged"),
        temp_dir.path().join("missing-source"),
        LinkKind::Directory,
    );

    let report =
        clean_project_with_provider(temp_dir.path(), &mut clean_provider, CleanMode::Broken)
            .unwrap();

    assert_eq!(report.removed.len(), 1);
    assert!(load_manifest(temp_dir.path()).unwrap().links.is_empty());
    assert!(clean_provider
        .entry(
            &temp_dir
                .path()
                .join(".agents")
                .join("skills")
                .join("unmanaged")
        )
        .is_some());
}

#[test]
fn clean_missing_source_removes_managed_link_even_if_link_is_not_broken() {
    let temp_dir = TestDir::new("clean-missing-source");
    fs::create_dir(temp_dir.path().join(".agents")).unwrap();
    fs::create_dir(temp_dir.path().join(".agents").join("skills")).unwrap();
    let source = seed_skill_source(&temp_dir);
    let canonical_source = crate::core::test_support::canonical_path(&source);
    let resolution = database(&temp_dir);
    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "writer".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();

    let mut provider = seed_provider(temp_dir.path(), &source);
    link_project_with_provider_and_db(
        temp_dir.path(),
        &mut provider,
        &resolution,
        LinkItemRequest {
            identifier: "writer".to_string(),
            link_name_override: None,
            target_dir_override: None,
        },
    )
    .unwrap();
    fs::remove_dir_all(&source).unwrap();

    let mut clean_provider = MockSymlinkProvider::new();
    clean_provider.add_dir(temp_dir.path());
    clean_provider.add_dir(temp_dir.path().join(".agents"));
    clean_provider.add_dir(temp_dir.path().join(".agents").join("skills"));
    clean_provider.add_dir(&canonical_source);
    clean_provider.add_symlink(
        temp_dir
            .path()
            .join(".agents")
            .join("skills")
            .join("writer"),
        canonical_source,
        LinkKind::Directory,
    );

    let report = clean_project_with_provider(
        temp_dir.path(),
        &mut clean_provider,
        CleanMode::MissingSource,
    )
    .unwrap();

    assert_eq!(report.removed.len(), 1);
    assert!(load_manifest(temp_dir.path()).unwrap().links.is_empty());
}

fn save_empty_manifest(project_root: &Path) {
    crate::core::manifest::save_manifest(project_root, &crate::core::manifest::Manifest::empty())
        .unwrap();
}

#[test]
fn link_identity_alias_and_rename_preserve_existing_project_state() {
    for item_type in [LinkableItemType::Skill, LinkableItemType::Resource] {
        let project = TestDir::new("link-identity");
        let resolution = database(&project);
        let store = Store::open_rw(&resolution).unwrap();
        fs::create_dir(project.path().join(".agents")).unwrap();
        let mut provider = MockSymlinkProvider::new();
        provider.add_dir(project.path());
        provider.add_dir(project.path().join(".agents"));
        provider.add_dir(project.path().join(".agents/skills"));
        provider.add_dir(project.path().join("docs"));
        let mut items = Vec::new();
        for (name, alias) in [("alpha", "beta"), ("beta", "gamma")] {
            let source = project.path().join(format!("source-{name}"));
            if item_type == LinkableItemType::Skill {
                fs::create_dir(&source).unwrap();
                fs::write(source.join("SKILL.md"), name).unwrap();
                provider.add_dir(crate::core::test_support::canonical_path(&source));
            } else {
                fs::write(&source, name).unwrap();
                provider.add_file(crate::core::test_support::canonical_path(&source));
            }
            items.push(
                store
                    .add_item(AddLinkableItem {
                        item_type,
                        name: name.into(),
                        alias: Some(alias.into()),
                        source_path: source,
                        default_target_dir: (item_type == LinkableItemType::Resource)
                            .then(|| PathBuf::from("docs")),
                        description: None,
                    })
                    .unwrap(),
            );
        }
        let request = |identifier: &str, override_name: Option<&str>| LinkItemRequest {
            identifier: identifier.into(),
            link_name_override: override_name.map(str::to_string),
            target_dir_override: None,
        };
        assert!(link_project_with_provider_and_db(
            project.path(),
            &mut provider,
            &resolution,
            request("gamma", None),
        )
        .is_err());
        let beta_link = link_project_with_provider_and_db(
            project.path(),
            &mut provider,
            &resolution,
            request("beta", None),
        )
        .unwrap();
        assert_eq!(beta_link.item_name, "beta");
        assert_eq!(beta_link.source_path, items[1].source_path);
        assert_eq!(
            beta_link.link_path,
            items[1].default_project_link_path().unwrap()
        );
        let alpha_link = link_project_with_provider_and_db(
            project.path(),
            &mut provider,
            &resolution,
            request(&items[0].id, None),
        )
        .unwrap();
        assert_eq!(
            alpha_link.link_path,
            items[0].default_project_link_path().unwrap()
        );
        store.create_group("daily", None).unwrap();
        assert_eq!(
            store
                .add_group_items("daily", &["beta".into()])
                .unwrap()
                .group
                .items[0]
                .id,
            items[1].id
        );
        let before_manifest = fs::read(super::manifest_path(project.path())).unwrap();
        let before_entries: Vec<_> = [&alpha_link, &beta_link]
            .iter()
            .map(|link| {
                provider
                    .entry(&project.path().join(&link.link_path))
                    .cloned()
            })
            .collect();
        let renamed = store.rename_item(item_type, "beta", "renamed").unwrap();
        assert_eq!(renamed.id, items[1].id);
        assert_eq!(renamed.alias, items[1].alias);
        assert_eq!(
            fs::read(super::manifest_path(project.path())).unwrap(),
            before_manifest
        );
        for (link, entry) in [&alpha_link, &beta_link].iter().zip(before_entries) {
            assert_eq!(
                provider
                    .entry(&project.path().join(&link.link_path))
                    .cloned(),
                entry
            );
        }
        for item in &items {
            let content_path = if item_type == LinkableItemType::Skill {
                item.source_path.join("SKILL.md")
            } else {
                item.source_path.clone()
            };
            assert_eq!(fs::read_to_string(content_path).unwrap(), item.name);
        }
        let removed =
            unlink_project_with_provider(project.path(), &mut provider, Some(items[1].id.clone()))
                .unwrap();
        assert_eq!(removed.outcomes.len(), 1);
        assert!(provider
            .entry(&project.path().join(&beta_link.link_path))
            .is_none());
        let overridden = link_project_with_provider_and_db(
            project.path(),
            &mut provider,
            &resolution,
            request("renamed", Some("custom")),
        )
        .unwrap();
        assert_eq!(overridden.link_path.file_name().unwrap(), "custom");
        assert_eq!(
            store
                .find_item(item_type, "renamed")
                .unwrap()
                .alias
                .as_deref(),
            Some("gamma")
        );
        let removed = unlink_group_project_with_provider_and_db(
            project.path(),
            &mut provider,
            &resolution,
            "daily",
        )
        .unwrap();
        assert_eq!(removed.outcomes.len(), 1);
        assert!(provider
            .entry(&project.path().join(&overridden.link_path))
            .is_none());
        assert!(provider
            .entry(&project.path().join(&alpha_link.link_path))
            .is_some());
        let manifest = load_manifest(project.path()).unwrap();
        assert_eq!(manifest.links.len(), 1);
        assert_eq!(manifest.links[0].item_id, items[0].id);
    }
}

#[test]
fn force_dry_run_and_execution_agree_and_preserve_state_on_rejection() {
    for scenario in [
        "wrong",
        "dangling",
        "missing-source",
        "wrong-kind",
        "file",
        "directory",
    ] {
        for force in [false, true] {
            let project = TestDir::new("force-parity");
            fs::create_dir_all(project.path().join(".agents/skills")).unwrap();
            save_empty_manifest(project.path());
            let source = seed_skill_source(&project);
            let resolution = database(&project);
            let store = Store::open_rw(&resolution).unwrap();
            let item = store
                .add_item(AddLinkableItem {
                    item_type: LinkableItemType::Skill,
                    name: "writer".to_string(),
                    alias: None,
                    source_path: source.clone(),
                    default_target_dir: None,
                    description: None,
                })
                .unwrap();
            let mut provider = seed_provider(project.path(), &source);
            let link = project.path().join(".agents/skills/writer");
            let old = project.path().join("old");
            if scenario == "wrong" {
                provider.add_dir(&old);
            }
            match scenario {
                "file" => provider.add_file(&link),
                "directory" => provider.add_dir(&link),
                _ => provider.add_symlink(&link, &old, LinkKind::Directory),
            }
            if scenario == "missing-source" {
                fs::remove_dir_all(&source).unwrap();
            } else if scenario == "wrong-kind" {
                fs::remove_dir_all(&source).unwrap();
                fs::write(&source, "wrong kind").unwrap();
                provider.add_file(&item.source_path);
            }
            let original = provider.entry(&link).cloned();
            let manifest_before = fs::read(super::manifest_path(project.path())).unwrap();
            let request = || LinkItemRequest {
                identifier: "writer".to_string(),
                link_name_override: None,
                target_dir_override: None,
            };
            let planned = link_project_with_provider_and_db_with_options(
                project.path(),
                &mut provider,
                &resolution,
                request(),
                LinkOptions {
                    dry_run: true,
                    force,
                },
            );
            assert_eq!(provider.entry(&link), original.as_ref(), "{scenario}");
            assert_eq!(
                fs::read(super::manifest_path(project.path())).unwrap(),
                manifest_before,
                "{scenario}"
            );
            let actual = link_project_with_provider_and_db_with_options(
                project.path(),
                &mut provider,
                &resolution,
                request(),
                LinkOptions {
                    dry_run: false,
                    force,
                },
            );
            match (planned, actual) {
                (Ok(planned), Ok(actual)) => {
                    assert!(force && matches!(scenario, "wrong" | "dangling"));
                    assert_eq!(planned.outcome, CreateSymlinkOutcome::ReplacedWrongSymlink);
                    assert_eq!(actual.outcome, planned.outcome);
                    assert_eq!(
                        provider.entry(&link),
                        Some(&MockEntry::Symlink {
                            target: item.source_path.clone(),
                            kind: LinkKind::Directory,
                        })
                    );
                    assert_eq!(load_manifest(project.path()).unwrap().links.len(), 1);
                }
                (Err(planned), Err(actual)) => {
                    assert_eq!(planned.to_string(), actual.to_string(), "{scenario}");
                    assert_eq!(provider.entry(&link), original.as_ref(), "{scenario}");
                    assert_eq!(
                        fs::read(super::manifest_path(project.path())).unwrap(),
                        manifest_before,
                        "{scenario}"
                    );
                }
                _ => panic!("dry-run and execution disagree: {scenario}, force={force}"),
            }
        }
    }
}

#[test]
fn project_replacement_failure_keeps_old_link_and_manifest_bytes() {
    let project = TestDir::new("force-failure");
    fs::create_dir_all(project.path().join(".agents/skills")).unwrap();
    save_empty_manifest(project.path());
    let source = seed_skill_source(&project);
    let resolution = database(&project);
    let store = Store::open_rw(&resolution).unwrap();
    store
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "writer".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();
    let mut provider = seed_provider(project.path(), &source);
    let link = project.path().join(".agents/skills/writer");
    provider.add_symlink(
        &link,
        project.path().join("old-missing"),
        LinkKind::Directory,
    );
    let before = provider.entry(&link).cloned();
    let manifest_before = fs::read(super::manifest_path(project.path())).unwrap();
    provider.fail_next_replacement();
    let error = link_project_with_provider_and_db_with_options(
        project.path(),
        &mut provider,
        &resolution,
        LinkItemRequest {
            identifier: "writer".to_string(),
            link_name_override: None,
            target_dir_override: None,
        },
        LinkOptions {
            dry_run: false,
            force: true,
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("injected replacement failure"));
    assert_eq!(provider.entry(&link), before.as_ref());
    assert_eq!(
        fs::read(super::manifest_path(project.path())).unwrap(),
        manifest_before
    );
}

#[test]
fn group_and_all_unlink_preserve_harness_and_follow_current_member_ids() {
    let project = TestDir::new("unlink-origins");
    let resolution = database(&project);
    let store = Store::open_rw(&resolution).unwrap();
    let source = seed_skill_source(&project);
    let writer = store
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "writer".into(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();
    let notes_source = project.path().join("notes.md");
    fs::write(&notes_source, "notes\n").unwrap();
    store
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Resource,
            name: "notes".into(),
            alias: None,
            source_path: notes_source.clone(),
            default_target_dir: Some(PathBuf::from("docs")),
            description: None,
        })
        .unwrap();
    let mut provider = seed_provider(project.path(), &source);
    provider.add_file(project.path().join("AGENTS.md"));
    provider.add_dir(project.path().join(".claude"));
    provider.add_dir(project.path().join("docs"));
    provider.add_file(crate::core::test_support::canonical_path(&notes_source));
    crate::core::manifest::init_project_with_mappings(
        project.path(),
        &mut provider,
        &crate::core::harness::default_init_mappings(),
    )
    .unwrap();
    for (identifier, alias) in [
        ("writer", Some("writer-a")),
        ("writer", Some("writer-b")),
        ("notes", None),
    ] {
        link_project_with_provider_and_db(
            project.path(),
            &mut provider,
            &resolution,
            LinkItemRequest {
                identifier: identifier.into(),
                link_name_override: alias.map(str::to_string),
                target_dir_override: None,
            },
        )
        .unwrap();
    }
    store.create_group("daily", None).unwrap();
    store
        .add_group_items("daily", &["writer".into(), "notes".into()])
        .unwrap();
    store
        .remove_group_items("daily", &["notes".into()])
        .unwrap();
    let before_rename = fs::read(super::manifest_path(project.path())).unwrap();
    let renamed = store
        .rename_item(LinkableItemType::Skill, "writer", "author")
        .unwrap();
    assert_eq!(renamed.id, writer.id);
    assert_eq!(
        fs::read(super::manifest_path(project.path())).unwrap(),
        before_rename
    );
    let mut manifest = load_manifest(project.path()).unwrap();
    assert_eq!(
        manifest
            .links
            .iter()
            .filter(|record| record.item_id == writer.id)
            .count(),
        2
    );
    // Even an infrastructure record sharing a member ID is outside group unlink.
    manifest.links[0].item_id = writer.id.clone();
    save_manifest(project.path(), &manifest).unwrap();
    let harness_records: Vec<_> = manifest
        .links
        .iter()
        .filter(|r| r.origin == LinkOrigin::Harness)
        .cloned()
        .collect();
    let harness_entries: Vec<_> = harness_records
        .iter()
        .map(|r| provider.entry(&project.path().join(&r.link_path)).cloned())
        .collect();
    let report = unlink_group_project_with_provider_and_db(
        project.path(),
        &mut provider,
        &resolution,
        "daily",
    )
    .unwrap();
    assert_eq!(report.outcomes.len(), 2);
    assert!(provider
        .entry(&project.path().join(".agents/skills/writer-a"))
        .is_none());
    assert!(provider
        .entry(&project.path().join(".agents/skills/writer-b"))
        .is_none());
    let remaining = load_manifest(project.path()).unwrap();
    assert_eq!(remaining.links.len(), 3);
    assert!(remaining.links.iter().any(|r| r.item_name == "notes"));
    let before_dry_run = fs::read(super::manifest_path(project.path())).unwrap();
    let dry_run = unlink_project_with_provider_with_options(
        project.path(),
        &mut provider,
        None,
        UnlinkOptions { dry_run: true },
    )
    .unwrap();
    assert_eq!(dry_run.outcomes.len(), 1);
    assert_eq!(
        fs::read(super::manifest_path(project.path())).unwrap(),
        before_dry_run
    );
    let report = unlink_project_with_provider(project.path(), &mut provider, None).unwrap();
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(
        load_manifest(project.path()).unwrap().links,
        harness_records
    );
    for (record, entry) in harness_records.iter().zip(harness_entries) {
        assert_eq!(
            provider
                .entry(&project.path().join(&record.link_path))
                .cloned(),
            entry
        );
    }
}
