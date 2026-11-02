//! Black-box mapping equivalence acceptance tests; no business source inspection.
use agent_linker::core::{
    harness::AddHarnessMapping,
    paths::{DbPathReason, DbPathResolution},
    store::Store,
    symlink::LinkKind,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "aglink-mapping-case-blind-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn conn(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.0.join("data/agent-linker.db")).unwrap()
    }
    fn store(&self) -> Store {
        let resolution = DbPathResolution {
            path: self.0.join("data/agent-linker.db"),
            reason: DbPathReason::ExplicitDataDirEnv,
        };
        Store::migrate(&resolution).unwrap();
        Store::open_rw(&resolution).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn mapping(path: &str) -> AddHarnessMapping {
    AddHarnessMapping {
        harness: "claude".into(),
        source_path: "AGENTS.md".into(),
        link_path: path.into(),
        link_kind: LinkKind::File,
    }
}
fn insert_historical(db: &rusqlite::Connection, id: &str, path: &str, required: bool) {
    db.execute(
        "INSERT INTO harness_mappings (id,harness_id,source_path,link_path,link_kind,required,created_at,updated_at) SELECT ?1,id,'AGENTS.md',?2,'file',?3,17,19 FROM harnesses WHERE name='claude'",
        rusqlite::params![id, path, required],
    ).unwrap();
}
fn row_exists(db: &rusqlite::Connection, id: &str) -> bool {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM harness_mappings WHERE id=?1)",
        [id],
        |row| row.get(0),
    )
    .unwrap()
}

#[test]
fn ordinary_mapping_rejects_all_ascii_case_and_dot_equivalents() {
    for stored in ["GEMINI.md", "./GEMINI.md"] {
        let f = Fixture::new();
        let store = f.store();
        let added = store.add_mapping(mapping(stored)).unwrap();
        for duplicate in ["gemini.md", "./gemini.md", "GeMiNi.MD", "./GeMiNi.MD"] {
            assert!(
                store.add_mapping(mapping(duplicate)).is_err(),
                "accepted {duplicate} after {stored}"
            );
            assert!(row_exists(&f.conn(), &added.id));
        }
    }
}

#[test]
fn ordinary_mapping_can_be_removed_by_ascii_case_and_dot_equivalents() {
    for spelling in ["gemini.md", "./gemini.md", "GeMiNi.MD", "./GeMiNi.MD"] {
        let f = Fixture::new();
        let store = f.store();
        let added = store.add_mapping(mapping("GEMINI.md")).unwrap();
        let removed = store.remove_mapping("claude", Path::new(spelling)).unwrap();
        assert_eq!(removed.id, added.id, "wrong removal for {spelling}");
        assert!(!row_exists(&f.conn(), &added.id));
    }
}

#[test]
fn historical_dot_mapping_rejects_ascii_case_equivalent_additions() {
    for duplicate in ["gemini.md", "./gemini.md", "GeMiNi.MD", "./GeMiNi.MD"] {
        let f = Fixture::new();
        let store = f.store();
        insert_historical(&f.conn(), "legacy", "./GEMINI.md", false);
        assert!(
            store.add_mapping(mapping(duplicate)).is_err(),
            "accepted historical duplicate {duplicate}"
        );
        assert!(row_exists(&f.conn(), "legacy"));
    }
}

#[test]
fn historical_dot_mapping_can_be_removed_by_ascii_case_equivalents() {
    for spelling in ["gemini.md", "./gemini.md", "GeMiNi.MD", "./GeMiNi.MD"] {
        let f = Fixture::new();
        let store = f.store();
        insert_historical(&f.conn(), "legacy", "./GEMINI.md", false);
        let removed = store.remove_mapping("claude", Path::new(spelling)).unwrap();
        assert_eq!(removed.id, "legacy");
        assert_eq!(removed.created_at, 17);
        assert_eq!(removed.updated_at, 19);
        assert!(!row_exists(&f.conn(), "legacy"));
    }
}

#[test]
fn exact_literal_record_wins_over_equivalent_historical_record() {
    for (spelling, expected, remaining) in [
        ("./GEMINI.md", "dot", "plain"),
        ("GEMINI.md", "plain", "dot"),
    ] {
        let f = Fixture::new();
        let store = f.store();
        insert_historical(&f.conn(), "dot", "./GEMINI.md", false);
        insert_historical(&f.conn(), "plain", "GEMINI.md", false);
        assert_eq!(
            store
                .remove_mapping("claude", Path::new(spelling))
                .unwrap()
                .id,
            expected
        );
        assert!(!row_exists(&f.conn(), expected));
        assert!(row_exists(&f.conn(), remaining));
    }
}

#[test]
fn required_records_are_protected_for_case_and_dot_variants() {
    for spelling in ["claude.md", "./claude.md", "ClAuDe.MD", "./ClAuDe.MD"] {
        let f = Fixture::new();
        let store = f.store();
        assert!(store.remove_mapping("claude", Path::new(spelling)).is_err());
        let required_count: i64 = f
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM harness_mappings WHERE link_path='CLAUDE.md' AND required=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(required_count, 1);
    }
}

#[test]
fn exact_required_record_is_not_bypassed_for_optional_equivalent() {
    let f = Fixture::new();
    let store = f.store();
    insert_historical(&f.conn(), "required-dot", "./GEMINI.md", true);
    insert_historical(&f.conn(), "optional-plain", "GEMINI.md", false);
    assert!(store
        .remove_mapping("claude", Path::new("./GEMINI.md"))
        .is_err());
    assert!(row_exists(&f.conn(), "required-dot"));
    assert!(row_exists(&f.conn(), "optional-plain"));
    assert_eq!(
        store
            .remove_mapping("claude", Path::new("GEMINI.md"))
            .unwrap()
            .id,
        "optional-plain"
    );
    assert!(row_exists(&f.conn(), "required-dot"));
}

#[cfg(unix)]
#[test]
fn equivalence_matches_sqlite_nocase_without_unicode_case_folding() {
    let f = Fixture::new();
    let store = f.store();
    let a = store.add_mapping(mapping("Ä.md")).unwrap();
    let b = store.add_mapping(mapping("ä.md")).unwrap();
    assert_ne!(a.id, b.id);
    assert!(store.add_mapping(mapping("Ä.MD")).is_err());
    assert_eq!(
        store
            .remove_mapping("claude", Path::new("ä.MD"))
            .unwrap()
            .id,
        b.id
    );
    assert!(row_exists(&f.conn(), &a.id));
}

#[cfg(windows)]
#[test]
fn windows_managed_paths_keep_unicode_case_equivalence() {
    let f = Fixture::new();
    let store = f.store();
    let added = store.add_mapping(mapping("Ä.md")).unwrap();
    assert!(store.add_mapping(mapping("ä.md")).is_err());
    assert_eq!(
        store
            .remove_mapping("claude", Path::new("ä.MD"))
            .unwrap()
            .id,
        added.id
    );
    assert!(!row_exists(&f.conn(), &added.id));
}

#[cfg(windows)]
#[test]
fn windows_separators_and_dot_components_are_equivalent() {
    for stored in [
        "docs/GEMINI.md",
        "./docs/./GEMINI.md",
        r".\docs\.\GEMINI.md",
    ] {
        let f = Fixture::new();
        let store = f.store();
        let added = store.add_mapping(mapping(stored)).unwrap();
        assert!(store.add_mapping(mapping(r"docs\gemini.md")).is_err());
        assert_eq!(
            store
                .remove_mapping("claude", Path::new(r".\docs\gemini.md"))
                .unwrap()
                .id,
            added.id
        );
    }
}

#[cfg(unix)]
#[test]
fn unix_backslash_stays_a_filename_character() {
    let f = Fixture::new();
    let store = f.store();
    let backslash = store.add_mapping(mapping(r"docs\GEMINI.md")).unwrap();
    let slash = store.add_mapping(mapping("docs/GEMINI.md")).unwrap();
    assert_ne!(backslash.id, slash.id);
    assert_eq!(
        store
            .remove_mapping("claude", Path::new(r"docs\gemini.md"))
            .unwrap()
            .id,
        backslash.id
    );
    assert!(row_exists(&f.conn(), &slash.id));
    assert_eq!(
        store
            .remove_mapping("claude", Path::new("docs/gemini.md"))
            .unwrap()
            .id,
        slash.id
    );
}
