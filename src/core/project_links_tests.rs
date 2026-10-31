use super::{
    clean_project_with_provider, doctor_project, link_group_project_with_provider_and_db,
    link_project_with_provider_and_db, link_project_with_provider_and_db_with_options,
    status_project_with_provider, unlink_group_project_with_provider_and_db,
    unlink_project_with_provider, unlink_project_with_provider_with_options, CleanMode,
    LinkItemRequest, LinkOptions, UnlinkOptions,
};
use crate::core::{
    linkable::LinkableItemType,
    manifest::load_manifest,
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
            status_project_with_provider(project.path(), &provider).is_err(),
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

    let status = status_project_with_provider(temp_dir.path(), &provider).unwrap();
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
    let preview = super::link_group_project_with_provider_and_db_with_options(
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

    let preview = super::unlink_group_project_with_provider_and_db_with_options(
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

#[test]
fn doctor_reports_database_manifest_harness_and_backend() {
    let temp_dir = TestDir::new("doctor");
    fs::create_dir(temp_dir.path().join(".agents")).unwrap();
    save_empty_manifest(temp_dir.path());
    let resolution = database(&temp_dir);

    let report = doctor_project(temp_dir.path(), &resolution, true);

    assert!(report.ok);
    assert!(report.checks.iter().any(|check| check.name == "database"));
    assert!(report.checks.iter().any(|check| check.name == "manifest"));
    assert!(report
        .checks
        .iter()
        .any(|check| check.name == "harness-adapter"));
    assert!(report
        .checks
        .iter()
        .any(|check| check.name == "symlink-backend"));
}

fn save_empty_manifest(project_root: &Path) {
    crate::core::manifest::save_manifest(project_root, &crate::core::manifest::Manifest::empty())
        .unwrap();
}

#[test]
fn doctor_accepts_valid_database_with_no_harness_rows() {
    let project = TestDir::new("doctor-empty-harnesses");
    fs::create_dir(project.path().join(".agents")).unwrap();
    save_empty_manifest(project.path());
    let resolution = database(&project);
    let c = rusqlite::Connection::open(&resolution.path).unwrap();
    c.execute("DELETE FROM harnesses", []).unwrap();
    drop(c);
    let report = doctor_project(project.path(), &resolution, true);
    assert!(
        report.ok,
        "valid empty harness registry must remain healthy"
    );
}
