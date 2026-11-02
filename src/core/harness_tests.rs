use super::{default_init_mappings, AddHarnessMapping};
use crate::core::symlink::LinkKind;
use crate::core::test_support::TestDir;
use crate::core::{
    paths::{DbPathReason, DbPathResolution},
    store::Store,
    Error,
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

#[test]
fn new_harness_mappings_receive_random_ids_and_preserve_uniqueness() {
    let temp = TestDir::new("harness-random-id");
    let resolution = DbPathResolution {
        path: temp.path().join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    Store::migrate(&resolution).unwrap();

    let store = Store::open_rw(&resolution).unwrap();
    let m1 = store
        .add_mapping(AddHarnessMapping {
            harness: "claude".to_string(),
            source_path: PathBuf::from("AGENTS.md"),
            link_path: PathBuf::from(".claude/extra1.md"),
            link_kind: LinkKind::File,
        })
        .unwrap();

    let m2 = store
        .add_mapping(AddHarnessMapping {
            harness: "claude".to_string(),
            source_path: PathBuf::from("AGENTS.md"),
            link_path: PathBuf::from(".claude/extra2.md"),
            link_kind: LinkKind::File,
        })
        .unwrap();

    assert_ne!(m1.id, m2.id);
    assert!(m1.id.starts_with("map_claude_"));
    assert!(m2.id.starts_with("map_claude_"));

    let suffix1 = m1.id.strip_prefix("map_claude_").unwrap();
    let suffix2 = m2.id.strip_prefix("map_claude_").unwrap();
    assert_eq!(suffix1.len(), 16);
    assert_eq!(suffix2.len(), 16);
    assert!(suffix1.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(suffix2.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn legacy_and_builtin_mapping_ids_remain_compatible() {
    let temp = TestDir::new("harness-legacy-compat");
    let resolution = DbPathResolution {
        path: temp.path().join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    Store::migrate(&resolution).unwrap();

    let store = Store::open_rw(&resolution).unwrap();
    // Built-ins exist and use stable IDs
    let all = store.list_all_mappings().unwrap();
    assert!(all.iter().any(|m| m.id == "init:claude:agents-md"));
    assert!(all.iter().any(|m| m.id == "init:claude:skills-dir"));

    // Insert legacy custom mapping
    store
        .write(|conn| {
            conn.execute(
                "INSERT INTO harness_mappings(id, harness_id, source_path, link_path, link_kind, required, created_at, updated_at)
                 VALUES('legacy-map-custom', 'claude', 'AGENTS.md', '.claude/legacy.md', 'file', 0, 100, 100)",
                [],
            )?;
            Ok(())
        })
        .unwrap();

    let all_after = store.list_all_mappings().unwrap();
    let legacy = all_after
        .iter()
        .find(|m| m.id == "legacy-map-custom")
        .expect("legacy mapping must be listed");
    assert_eq!(legacy.harness_id, "claude");
    assert_eq!(legacy.link_path, PathBuf::from(".claude/legacy.md"));

    let project_mapping = legacy.for_project("claude");
    assert_eq!(project_mapping.item_id, "harness:legacy-map-custom");

    // Remove legacy mapping by link_path
    let removed = store
        .remove_mapping("claude", &PathBuf::from(".claude/legacy.md"))
        .unwrap();
    assert_eq!(removed.id, "legacy-map-custom");
}

#[test]
fn harness_mapping_primary_key_collision_fails_and_rolls_back() {
    let temp = TestDir::new("harness-collision-rollback");
    let resolution = DbPathResolution {
        path: temp.path().join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    Store::migrate(&resolution).unwrap();

    let store = Store::open_rw(&resolution).unwrap();

    // Verify inserting duplicate primary key fails with Error::Conflict
    let result = store.write(|conn| {
        conn.execute(
            "INSERT INTO harness_mappings(id, harness_id, source_path, link_path, link_kind, required, created_at, updated_at)
             VALUES('duplicate-id', 'claude', 'AGENTS.md', '.claude/dup1.md', 'file', 0, 100, 100)",
            [],
        )?;
        // Second insert in same transaction with identical primary key 'duplicate-id'
        conn.execute(
            "INSERT INTO harness_mappings(id, harness_id, source_path, link_path, link_kind, required, created_at, updated_at)
             VALUES('duplicate-id', 'claude', 'AGENTS.md', '.claude/dup2.md', 'file', 0, 100, 100)",
            [],
        )?;
        Ok(())
    });

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, Error::Conflict { .. }),
        "expected Error::Conflict, got {err:?}"
    );

    // Verify transaction rolled back: 'duplicate-id' is NOT in the database
    let count: i64 = store
        .write(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM harness_mappings WHERE id='duplicate-id'",
                [],
                |r| r.get(0),
            )
            .map_err(Error::from)
        })
        .unwrap();
    assert_eq!(count, 0, "transaction must roll back completely on collision");
}
