use super::{default_init_mappings, AddHarnessMapping};
use crate::core::symlink::LinkKind;
use crate::core::{
    paths::{DbPathReason, DbPathResolution},
    store::Store,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn claude_init_mappings_are_built_in_defaults() {
    let mappings = default_init_mappings();

    assert_eq!(mappings.len(), 2);
    assert_eq!(mappings[0].harness_name, "claude");
    assert_eq!(mappings[0].source_path, PathBuf::from("AGENTS.md"));
    assert_eq!(mappings[0].link_path, PathBuf::from("CLAUDE.md"));
    assert_eq!(mappings[0].link_kind, LinkKind::File);
    assert_eq!(
        mappings[1].source_path,
        PathBuf::from(".agents").join("skills")
    );
    assert_eq!(
        mappings[1].link_path,
        PathBuf::from(".claude").join("skills")
    );
    assert_eq!(mappings[1].link_kind, LinkKind::Directory);
}

#[test]
fn harness_api_lists_shows_toggles_and_returns_mappings() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "agent-linker-harness-test-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&temp_dir).unwrap();
    let resolution = DbPathResolution {
        path: temp_dir.join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };

    Store::migrate(&resolution).unwrap();

    let harnesses = Store::open_ro(&resolution)
        .unwrap()
        .list_harnesses()
        .unwrap();
    assert_eq!(harnesses.len(), 1);
    assert_eq!(harnesses[0].name, "claude");
    assert!(harnesses[0].enabled);
    assert_eq!(harnesses[0].mappings.len(), 2);

    let claude = Store::open_ro(&resolution)
        .unwrap()
        .show_harness("claude")
        .unwrap();
    assert_eq!(claude.display_name, "Claude Code");

    let mappings = Store::open_ro(&resolution)
        .unwrap()
        .list_all_mappings()
        .unwrap();
    assert_eq!(mappings.len(), 2);
    assert!(mappings.iter().any(|mapping| {
        mapping.source_path == PathBuf::from("AGENTS.md")
            && mapping.link_path == PathBuf::from("CLAUDE.md")
            && mapping.link_kind == LinkKind::File
    }));

    Store::open_rw(&resolution)
        .unwrap()
        .disable_harness("claude")
        .unwrap();
    let harnesses = Store::open_ro(&resolution)
        .unwrap()
        .list_harnesses()
        .unwrap();
    let claude = harnesses.iter().find(|h| h.name == "claude").unwrap();
    assert!(!claude.enabled);
    assert_eq!(claude.mappings.len(), 2);
    Store::open_rw(&resolution)
        .unwrap()
        .enable_harness("claude")
        .unwrap();
    let harnesses = Store::open_ro(&resolution)
        .unwrap()
        .list_harnesses()
        .unwrap();
    let claude = harnesses.iter().find(|h| h.name == "claude").unwrap();
    assert!(claude.enabled);
    assert_eq!(claude.mappings.len(), 2);

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn harness_mapping_add_and_remove_manage_non_required_mappings() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "agent-linker-harness-mapping-test-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&temp_dir).unwrap();
    let resolution = DbPathResolution {
        path: temp_dir.join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    Store::migrate(&resolution).unwrap();

    let mapping = Store::open_rw(&resolution)
        .unwrap()
        .add_mapping(AddHarnessMapping {
            harness: "claude".to_string(),
            source_path: PathBuf::from("AGENTS.md"),
            link_path: PathBuf::from(".claude").join("extra.md"),
            link_kind: LinkKind::File,
        })
        .unwrap();

    assert!(!mapping.required);
    assert!(Store::open_ro(&resolution)
        .unwrap()
        .list_all_mappings()
        .unwrap()
        .iter()
        .any(|stored| stored.id == mapping.id));

    let removed = Store::open_rw(&resolution)
        .unwrap()
        .remove_mapping("claude", &mapping.link_path)
        .unwrap();
    assert_eq!(removed.id, mapping.id);

    let required = Store::open_rw(&resolution)
        .unwrap()
        .remove_mapping("claude", &PathBuf::from("CLAUDE.md"))
        .unwrap_err();
    assert!(required.to_string().contains("required"));

    let _ = fs::remove_dir_all(temp_dir);
}
