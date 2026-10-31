use super::{
    load_manifest, managed_gitignore_block, parse_manifest, replace_or_append_managed_block,
    save_manifest, serialize_manifest, Manifest,
};
use crate::core::{
    error::Error,
    symlink::{LinkKind, MockEntry, MockSymlinkProvider},
    test_support::TestDir,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

// The original mock tests exercise the default mapping contract independently
// of any global store installed on the machine running the test suite.
fn init_project_with_provider(
    project_root: &Path,
    provider: &mut MockSymlinkProvider,
) -> Result<super::InitProjectReport, Error> {
    super::init_project_with_mappings(
        project_root,
        provider,
        &crate::core::harness::default_init_mappings(),
    )
}

fn seeded_provider(project_root: &Path) -> MockSymlinkProvider {
    let mut provider = MockSymlinkProvider::new();
    provider.add_dir(project_root);
    provider.add_file(project_root.join("AGENTS.md"));
    provider.add_dir(project_root.join(".agents"));
    provider.add_dir(project_root.join(".agents").join("skills"));
    provider.add_dir(project_root.join(".claude"));
    provider
}

#[test]
fn temporary_manifests_reject_unsafe_and_duplicate_link_paths() {
    let project = TestDir::new("manifest-path-validation");
    let record = super::record_from_mapping(
        &crate::core::harness::default_init_mappings()[0],
        crate::core::symlink::SymlinkBackend::Mock,
    );
    fs::create_dir(project.path().join(".agents")).unwrap();
    let disk_path = super::manifest_path(project.path());
    for path in [
        "",
        "/outside",
        "C:outside",
        "C:/outside",
        r"C:\outside",
        "../outside",
        "safe/../outside",
    ] {
        let mut record = record.clone();
        record.link_path = PathBuf::from(path);
        let manifest = Manifest {
            schema_version: 1,
            links: vec![record],
        };
        fs::write(&disk_path, serialize_manifest(&manifest)).unwrap();
        assert!(load_manifest(project.path()).is_err(), "{path}");
        let original = fs::read(&disk_path).unwrap();
        assert!(save_manifest(project.path(), &manifest).is_err(), "{path}");
        assert_eq!(fs::read(&disk_path).unwrap(), original);
    }
    let mut duplicate = record.clone();
    duplicate.id.push_str(":duplicate");
    for path in ["CLAUDE.md", "./CLAUDE.md"] {
        duplicate.link_path = PathBuf::from(path);
        assert!(Manifest {
            schema_version: 1,
            links: vec![record.clone(), duplicate.clone()]
        }
        .validate()
        .is_err());
    }
    #[cfg(windows)]
    {
        duplicate.link_path = PathBuf::from("claude.MD");
        assert!(Manifest {
            schema_version: 1,
            links: vec![record, duplicate]
        }
        .validate()
        .is_err());
    }
}

#[test]
fn init_uses_the_given_directory_even_when_a_parent_project_exists() {
    let project = TestDir::new("init-current-only");
    let mut outer_provider = seeded_provider(project.path());
    init_project_with_provider(project.path(), &mut outer_provider).unwrap();
    let original = fs::read(super::manifest_path(project.path())).unwrap();
    let child = project.path().join("child");
    fs::create_dir(&child).unwrap();
    let mut child_provider = seeded_provider(&child);
    let report = init_project_with_provider(&child, &mut child_provider).unwrap();
    assert_eq!(report.project_root, child);
    assert!(super::manifest_path(&child).is_file());
    assert_eq!(
        fs::read(super::manifest_path(project.path())).unwrap(),
        original
    );
}

#[test]
fn manifest_round_trips_generated_schema() {
    let temp_dir = TestDir::new("manifest");
    let mut provider = seeded_provider(temp_dir.path());

    init_project_with_provider(temp_dir.path(), &mut provider).unwrap();
    let manifest = load_manifest(temp_dir.path()).unwrap();
    save_manifest(temp_dir.path(), &manifest).unwrap();
    let reparsed = load_manifest(temp_dir.path()).unwrap();

    assert_eq!(manifest, reparsed);
    assert_eq!(manifest.links.len(), 2);
}

#[test]
fn legacy_framework_name_manifest_remains_readable_and_writable() {
    let legacy = r#"schema_version = 1
[[links]]
id = "init:claude:agents-md"
scope = "project"
framework_name = "claude"
item_id = "project:agents-md"
item_name = "AGENTS.md"
source_path = "AGENTS.md"
link_path = "CLAUDE.md"
link_kind = "file"
provider_backend = "mock"
created_by_command = "init"
created_at = "unix:1"
updated_at = "unix:1"
"#;
    let manifest = parse_manifest(legacy).unwrap();
    assert_eq!(manifest.links[0].harness_name, "claude");
    assert_eq!(manifest.links[0].id, "init:claude:agents-md");
    let saved = serialize_manifest(&manifest);
    assert!(saved.contains("framework_name = \"claude\""));
    assert!(!saved.contains("harness_name ="));
    assert_eq!(parse_manifest(&saved).unwrap(), manifest);
}

#[test]
fn damaged_manifest_fails_without_overwrite() {
    let temp_dir = TestDir::new("manifest");
    fs::create_dir(temp_dir.path().join(".agents")).unwrap();
    let manifest_path = temp_dir.path().join(".agents").join("links.toml");
    fs::write(&manifest_path, "not valid").unwrap();

    let mut provider = seeded_provider(temp_dir.path());
    let error = init_project_with_provider(temp_dir.path(), &mut provider).unwrap_err();

    assert!(matches!(error, Error::Manifest(_)));
    assert_eq!(fs::read_to_string(manifest_path).unwrap(), "not valid");
}

#[test]
fn init_creates_project_files_links_manifest_and_gitignore() {
    let temp_dir = TestDir::new("manifest");
    let mut provider = seeded_provider(temp_dir.path());

    let report = init_project_with_provider(temp_dir.path(), &mut provider).unwrap();

    assert!(temp_dir.path().join("AGENTS.md").is_file());
    assert!(temp_dir.path().join(".agents").is_dir());
    assert!(temp_dir.path().join(".agents").join("skills").is_dir());
    assert!(temp_dir.path().join(".claude").is_dir());
    assert!(temp_dir.path().join(".agents").join("links.toml").is_file());
    assert_eq!(report.link_outcomes.len(), 2);
    assert!(matches!(
        provider.entry(&temp_dir.path().join("CLAUDE.md")),
        Some(MockEntry::Symlink {
            kind: LinkKind::File,
            ..
        })
    ));
    assert!(matches!(
        provider.entry(&temp_dir.path().join(".claude").join("skills")),
        Some(MockEntry::Symlink {
            kind: LinkKind::Directory,
            ..
        })
    ));

    let gitignore = fs::read_to_string(temp_dir.path().join(".gitignore")).unwrap();
    assert_eq!(gitignore, managed_gitignore_block());

    let manifest = load_manifest(temp_dir.path()).unwrap();
    assert_eq!(manifest.links.len(), 2);
    assert!(manifest.links.iter().any(|link| {
        link.harness_name == "claude"
            && link.source_path == PathBuf::from("AGENTS.md")
            && link.link_path == PathBuf::from("CLAUDE.md")
    }));
}

#[test]
fn repeated_init_is_idempotent_and_preserves_agents_content() {
    let temp_dir = TestDir::new("manifest");
    fs::write(temp_dir.path().join("AGENTS.md"), "user content\n").unwrap();
    fs::write(temp_dir.path().join(".gitignore"), "target/\n").unwrap();

    let mut provider = seeded_provider(temp_dir.path());
    init_project_with_provider(temp_dir.path(), &mut provider).unwrap();
    let first_manifest =
        fs::read_to_string(temp_dir.path().join(".agents").join("links.toml")).unwrap();

    let mut provider = provider;
    init_project_with_provider(temp_dir.path(), &mut provider).unwrap();

    assert_eq!(
        fs::read_to_string(temp_dir.path().join("AGENTS.md")).unwrap(),
        "user content\n"
    );
    let gitignore = fs::read_to_string(temp_dir.path().join(".gitignore")).unwrap();
    assert!(gitignore.starts_with("target/\n"));
    assert_eq!(gitignore.matches("# BEGIN aglink managed").count(), 1);
    assert!(
        fs::read_to_string(temp_dir.path().join(".agents").join("links.toml"))
            .unwrap()
            .contains("init:claude:agents-md")
    );
    assert!(parse_manifest(&first_manifest).is_ok());
}

#[test]
fn init_rejects_real_file_at_claude_link() {
    let temp_dir = TestDir::new("manifest");
    let mut provider = seeded_provider(temp_dir.path());
    provider.add_file(temp_dir.path().join("CLAUDE.md"));

    let error = init_project_with_provider(temp_dir.path(), &mut provider).unwrap_err();

    assert!(matches!(error, Error::Symlink(_)));
}

#[test]
fn init_rejects_real_directory_at_claude_skills_link() {
    let temp_dir = TestDir::new("manifest");
    let mut provider = seeded_provider(temp_dir.path());
    provider.add_dir(temp_dir.path().join(".claude").join("skills"));

    let error = init_project_with_provider(temp_dir.path(), &mut provider).unwrap_err();

    assert!(matches!(error, Error::Symlink(_)));
}

#[test]
fn init_rejects_wrong_symlink_target() {
    let temp_dir = TestDir::new("manifest");
    let mut provider = seeded_provider(temp_dir.path());
    provider.add_file(temp_dir.path().join("OTHER.md"));
    provider.add_symlink(
        temp_dir.path().join("CLAUDE.md"),
        temp_dir.path().join("OTHER.md"),
        LinkKind::File,
    );

    let error = init_project_with_provider(temp_dir.path(), &mut provider).unwrap_err();

    assert!(matches!(error, Error::Symlink(_)));
}

#[test]
fn gitignore_managed_block_preserves_user_content() {
    let original = "target/\n\n# BEGIN aglink managed\nold\n# END aglink managed\nlogs/\n";
    let updated = replace_or_append_managed_block(original, &managed_gitignore_block()).unwrap();

    assert!(updated.starts_with("target/\n\n"));
    assert!(updated.contains(".claude/"));
    assert!(updated.ends_with("logs/\n"));
    assert_eq!(updated.matches("# BEGIN aglink managed").count(), 1);
}

#[test]
fn existing_empty_manifest_is_invalid() {
    let error = parse_manifest("").unwrap_err();

    assert!(matches!(error, Error::Manifest(_)));
    assert_eq!(Manifest::empty().links.len(), 0);
}

#[test]
fn explicit_mappings_drive_mock_init_and_preserve_harness_provenance() {
    let project = TestDir::new("init-custom-mappings");
    let mut provider = seeded_provider(project.path());
    let mut mapping = crate::core::harness::default_init_mappings()[0].clone();
    mapping.harness_name = "custom-harness".into();
    mapping.link_path = PathBuf::from("CUSTOM.md");
    let report =
        super::init_project_with_mappings(project.path(), &mut provider, &[mapping.clone()])
            .unwrap();
    assert_eq!(report.link_outcomes.len(), 1);
    assert!(matches!(
        provider.entry(&project.path().join("CUSTOM.md")),
        Some(MockEntry::Symlink {
            kind: LinkKind::File,
            ..
        })
    ));
    assert!(provider.entry(&project.path().join("CLAUDE.md")).is_none());
    let manifest = load_manifest(project.path()).unwrap();
    assert_eq!(manifest.links.len(), 1);
    assert_eq!(manifest.links[0].harness_name, "custom-harness");
    let first = fs::read(super::manifest_path(project.path())).unwrap();
    super::init_project_with_mappings(project.path(), &mut provider, &[mapping]).unwrap();
    assert_eq!(
        fs::read(super::manifest_path(project.path())).unwrap(),
        first
    );
}

#[test]
fn custom_mock_init_preserves_real_conflicts_and_rejects_escaping_paths() {
    let project = TestDir::new("init-custom-conflict");
    let mut provider = seeded_provider(project.path());
    let mut mapping = crate::core::harness::default_init_mappings()[0].clone();
    mapping.link_path = PathBuf::from("CUSTOM.md");
    provider.add_file(project.path().join("CUSTOM.md"));
    assert!(matches!(
        super::init_project_with_mappings(project.path(), &mut provider, &[mapping.clone()]),
        Err(Error::Symlink(_))
    ));
    assert!(matches!(
        provider.entry(&project.path().join("CUSTOM.md")),
        Some(MockEntry::File)
    ));
    for (source, link) in [("AGENTS.md", "../outside.md"), ("../outside.md", "safe.md")] {
        mapping.source_path = PathBuf::from(source);
        mapping.link_path = PathBuf::from(link);
        assert!(super::init_project_with_mappings(
            project.path(),
            &mut provider,
            &[mapping.clone()]
        )
        .is_err());
    }
}

#[test]
fn mock_init_store_missing_falls_back_disabled_stays_empty_and_old_schema_fails() {
    use crate::core::{
        paths::{DbPathReason, DbPathResolution},
        store::Store,
    };
    let project = TestDir::new("init-store-fallback");
    let data = TestDir::new("init-store-data");
    let resolution = DbPathResolution {
        path: data.path().join("missing/agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    let mut provider = seeded_provider(project.path());
    super::init_project_with_store(project.path(), &mut provider, &resolution).unwrap();
    assert_eq!(load_manifest(project.path()).unwrap().links.len(), 2);
    assert!(!resolution.path.parent().unwrap().exists());
    Store::migrate(&resolution).unwrap();
    Store::open_rw(&resolution)
        .unwrap()
        .disable_harness("claude")
        .unwrap();
    let disabled = TestDir::new("init-store-disabled");
    let mut provider = seeded_provider(disabled.path());
    let original = fs::read(&resolution.path).unwrap();
    super::init_project_with_store(disabled.path(), &mut provider, &resolution).unwrap();
    assert!(load_manifest(disabled.path()).unwrap().links.is_empty());
    assert!(provider.entry(&disabled.path().join("CLAUDE.md")).is_none());
    assert_eq!(fs::read(&resolution.path).unwrap(), original);
    rusqlite::Connection::open(&resolution.path)
        .unwrap()
        .execute_batch("PRAGMA user_version=2;")
        .unwrap();
    let old = TestDir::new("init-store-old");
    let mut provider = seeded_provider(old.path());
    let error = super::init_project_with_store(old.path(), &mut provider, &resolution).unwrap_err();
    assert!(error.to_string().contains("db migrate"));
    assert!(provider.entry(&old.path().join("CLAUDE.md")).is_none());
}

#[test]
fn mock_init_reads_custom_enabled_mappings_while_another_writer_is_active() {
    use crate::core::{
        paths::{DbPathReason, DbPathResolution},
        store::Store,
    };
    let project = TestDir::new("init-store-custom");
    let data = TestDir::new("init-custom-db");
    let resolution = DbPathResolution {
        path: data.path().join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    Store::migrate(&resolution).unwrap();
    let c = rusqlite::Connection::open(&resolution.path).unwrap();
    c.execute_batch("UPDATE harnesses SET enabled=0; INSERT INTO harnesses VALUES('custom','custom','Custom',0,1,10,10); INSERT INTO harness_mappings VALUES('custom-map','custom','AGENTS.md','CUSTOM.md','file',0,10,10); BEGIN IMMEDIATE;").unwrap();
    let mut provider = seeded_provider(project.path());
    super::init_project_with_store(project.path(), &mut provider, &resolution).unwrap();
    let manifest = load_manifest(project.path()).unwrap();
    assert_eq!(manifest.links.len(), 1);
    assert_eq!(manifest.links[0].harness_name, "custom");
    assert_eq!(manifest.links[0].link_path, PathBuf::from("CUSTOM.md"));
    assert!(provider.entry(&project.path().join("CLAUDE.md")).is_none());
    c.execute_batch("ROLLBACK;").unwrap();
}
