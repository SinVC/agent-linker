//! CLI acceptance tests written from the Phase 2 contract without reading implementation.
use rusqlite::{Connection, OptionalExtension};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture {
    root: PathBuf,
    data: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "aglink-phase2-blind-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        Self {
            data: root.join("not-created/data"),
            root,
        }
    }
    fn db(&self) -> PathBuf {
        self.data.join("agent-linker.db")
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_aglink"))
            .current_dir(&self.root)
            .env("AGLINK_DATA_DIR", &self.data)
            .args(args)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> String {
        ok(self.run(args), args)
    }
    fn fail(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            !output.status.success(),
            "unexpected success for {args:?}: {}",
            output_text(&output)
        );
        output_text(&output)
    }
    fn migrate(&self) {
        self.ok(&["db", "migrate"]);
    }
    fn conn(&self) -> Connection {
        let c = Connection::open(self.db()).unwrap();
        c.pragma_update(None, "foreign_keys", true).unwrap();
        c
    }
    fn skill(&self, name: &str) -> PathBuf {
        let p = self.root.join(format!("source-{name}"));
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("SKILL.md"), format!("# {name}\n")).unwrap();
        self.ok(&["skill", "add", p.to_str().unwrap(), "--name", name]);
        p
    }
    fn resource(&self, name: &str, target: &str, alias: Option<&str>) -> PathBuf {
        let p = self.root.join(format!("source-{name}.txt"));
        fs::write(&p, name).unwrap();
        let mut args = vec![
            "resource",
            "add",
            p.to_str().unwrap(),
            "--name",
            name,
            "--target-dir",
            target,
        ];
        if let Some(alias) = alias {
            args.extend(["--alias", alias]);
        }
        self.ok(&args);
        p
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn output_text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}
fn ok(o: Output, args: &[&str]) -> String {
    assert!(
        o.status.success(),
        "{args:?} failed ({}): {}",
        o.status,
        output_text(&o)
    );
    output_text(&o)
}
fn count(c: &Connection, table: &str) -> i64 {
    c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}
fn has_case(text: &str, word: &str) -> bool {
    text.to_ascii_lowercase()
        .contains(&word.to_ascii_lowercase())
}

#[test]
fn missing_database_never_initializes_on_ordinary_reads_or_writes() {
    let f = Fixture::new();
    for args in [
        vec!["harness", "list"],
        vec!["skill", "list"],
        vec!["resource", "list"],
        vec!["group", "list"],
        vec!["group", "create", "x"],
        vec!["skill", "add", "does-not-exist"],
        vec![
            "resource",
            "add",
            "does-not-exist",
            "--target-dir",
            "assets",
        ],
    ] {
        let text = f.fail(&args);
        assert!(
            has_case(&text, "db migrate"),
            "missing migration guidance: {text}"
        );
        assert!(
            !f.data.parent().unwrap().exists(),
            "ordinary command created data parent: {args:?}"
        );
    }
    assert!(f.ok(&["db", "path"]).contains("agent-linker.db"));
    assert!(!f.data.parent().unwrap().exists());
}

#[test]
fn config_is_removed_and_help_keeps_database_commands() {
    let f = Fixture::new();
    let help = f.ok(&["--help"]);
    assert!(
        !help.lines().any(|l| l.trim_start().starts_with("config ")),
        "{help}"
    );
    f.fail(&["config", "list"]);
    let help = f.ok(&["db", "--help"]);
    for command in ["path", "migrate", "check", "backup"] {
        assert!(help.contains(command), "{help}");
    }
    assert!(!f.data.parent().unwrap().exists());
}

#[test]
fn migrate_creates_v3_seed_once_and_queries_leave_bytes_unchanged() {
    let f = Fixture::new();
    f.migrate();
    {
        let c = f.conn();
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            3
        );
        assert_eq!(count(&c, "harnesses"), 1);
        assert_eq!(count(&c, "harness_mappings"), 2);
        let row: (String, i64, i64) = c
            .query_row("SELECT name,built_in,enabled FROM harnesses", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert_eq!(row, ("claude".into(), 1, 1));
        c.execute("UPDATE harnesses SET enabled=0,updated_at=123", [])
            .unwrap();
        c.execute("UPDATE harness_mappings SET required=0,updated_at=456", [])
            .unwrap();
    }
    let before = fs::read(f.db()).unwrap();
    for args in [
        vec!["harness", "list"],
        vec!["harness", "show", "claude"],
        vec!["harness", "mapping", "list"],
        vec!["group", "list"],
        vec!["skill", "list"],
        vec!["resource", "list"],
        vec!["db", "check"],
        vec!["db", "migrate"],
    ] {
        f.ok(&args);
    }
    assert_eq!(
        fs::read(f.db()).unwrap(),
        before,
        "query or repeated migrate rewrote the database"
    );
    let c = f.conn();
    assert_eq!(
        c.query_row("SELECT enabled,updated_at FROM harnesses", [], |r| Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?
        )))
        .unwrap(),
        (0, 123)
    );
    assert_eq!(
        c.query_row(
            "SELECT COUNT(*) FROM harness_mappings WHERE required=0 AND updated_at=456",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
}

#[test]
fn ordinary_commands_reject_older_and_newer_schema_without_mutation() {
    for version in [2, 4] {
        let f = Fixture::new();
        f.migrate();
        {
            f.conn()
                .pragma_update(None, "user_version", version)
                .unwrap();
        }
        let before = fs::read(f.db()).unwrap();
        let text = f.fail(&["harness", "list"]);
        if version == 2 {
            assert!(has_case(&text, "db migrate"), "{text}");
        } else {
            assert!(
                has_case(&text, "upgrade") || has_case(&text, "newer"),
                "{text}"
            );
        }
        let check = f.fail(&["db", "check"]);
        assert!(has_case(&check, "schema"), "{check}");
        assert_eq!(before, fs::read(f.db()).unwrap());
    }
}

#[test]
fn schema_has_exact_v3_columns_types_and_case_insensitive_constraints() {
    let f = Fixture::new();
    f.migrate();
    let c = f.conn();
    let expected = [
        (
            "harnesses",
            vec![
                "id",
                "name",
                "display_name",
                "built_in",
                "enabled",
                "created_at",
                "updated_at",
            ],
        ),
        (
            "harness_mappings",
            vec![
                "id",
                "harness_id",
                "source_path",
                "link_path",
                "link_kind",
                "required",
                "created_at",
                "updated_at",
            ],
        ),
        (
            "linkable_items",
            vec![
                "id",
                "kind",
                "name",
                "alias",
                "default_link_name",
                "source_path",
                "source_kind",
                "target_dir",
                "description",
                "created_at",
                "updated_at",
            ],
        ),
        (
            "groups",
            vec!["id", "name", "description", "created_at", "updated_at"],
        ),
        ("group_items", vec!["group_id", "item_id", "created_at"]),
    ];
    for (table, columns) in expected {
        let mut stmt = c.prepare(&format!("PRAGMA table_info({table})")).unwrap();
        let actual: Vec<(String, String, i64)> = stmt
            .query_map([], |r| Ok((r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let mut names: Vec<_> = actual.iter().map(|x| x.0.as_str()).collect();
        names.sort();
        let mut wanted = columns;
        wanted.sort();
        assert_eq!(names, wanted, "{table}");
        for (name, ty, not_null) in actual {
            if !["alias", "description", "target_dir"].contains(&name.as_str()) {
                assert_eq!(not_null, 1, "{table}.{name} must be NOT NULL");
            }
            if name.ends_with("_at") {
                assert_eq!(ty.to_ascii_uppercase(), "INTEGER");
            }
        }
    }
    for old in ["frameworks", "framework_mappings", "config"] {
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                [old],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }
    assert!(c
        .execute(
            "INSERT INTO harnesses VALUES('duplicate','CLAUDE','dup',1,1,0,0)",
            []
        )
        .is_err());
    assert!(c
        .execute(
            "INSERT INTO harnesses VALUES('bad','bad','bad',2,1,0,0)",
            []
        )
        .is_err());
    c.execute("INSERT INTO groups VALUES('g1','GroupName',NULL,0,0)", [])
        .unwrap();
    assert!(c
        .execute("INSERT INTO groups VALUES('g2','groupname',NULL,0,0)", [])
        .is_err());
    let hid: String = c
        .query_row("SELECT id FROM harnesses", [], |r| r.get(0))
        .unwrap();
    c.execute(
        "INSERT INTO harness_mappings VALUES('m1',?,'a','CASE/PATH','file',1,0,0)",
        [&hid],
    )
    .unwrap();
    assert!(c
        .execute(
            "INSERT INTO harness_mappings VALUES('m2',?,'a','case/path','file',1,0,0)",
            [&hid]
        )
        .is_err());
    assert!(c
        .execute(
            "INSERT INTO harness_mappings VALUES('m3',?,'a','different','invalid',1,0,0)",
            [&hid]
        )
        .is_err());
    assert!(c
        .execute("INSERT INTO group_items VALUES('missing','missing',0)", [])
        .is_err());
}

#[test]
fn item_defaults_normalization_ids_and_uniqueness_follow_contract() {
    let f = Fixture::new();
    f.migrate();
    let source = f.skill("Alpha");
    let r1 = f.resource("AssetOne", "./assets/./images", Some("shared.txt"));
    f.resource("AssetTwo", "other", Some("SHARED.TXT"));
    let c = f.conn();
    let (id, default): (String, String) = c
        .query_row(
            "SELECT id,default_link_name FROM linkable_items WHERE name='Alpha'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(
        id.starts_with("itm_") && id.len() == 20 && id[4..].bytes().all(|x| x.is_ascii_hexdigit()),
        "{id}"
    );
    assert_eq!(default, "Alpha");
    let target: String = c
        .query_row(
            "SELECT target_dir FROM linkable_items WHERE name='AssetOne'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(target.replace('\\', "/"), "assets/images");
    drop(c);
    let duplicate = f.root.join("duplicate.txt");
    fs::write(&duplicate, "duplicate").unwrap();
    f.fail(&[
        "resource",
        "add",
        duplicate.to_str().unwrap(),
        "--name",
        "alpha",
        "--target-dir",
        "new",
    ]);
    f.fail(&[
        "resource",
        "add",
        duplicate.to_str().unwrap(),
        "--name",
        "Different",
        "--target-dir",
        "assets/images",
        "--alias",
        "SHARED.TXT",
    ]);
    let s2 = f.root.join("s2");
    fs::create_dir(&s2).unwrap();
    fs::write(s2.join("SKILL.md"), "x").unwrap();
    f.fail(&[
        "skill",
        "add",
        s2.to_str().unwrap(),
        "--name",
        "Second",
        "--alias",
        "ALPHA",
    ]);
    let project = f.root.join("project");
    fs::create_dir_all(project.join(".agents")).unwrap();
    fs::write(project.join(".agents/links.toml"), "schema_version = 1\n").unwrap();
    let manifest = fs::read(project.join(".agents/links.toml")).unwrap();
    f.ok(&["skill", "rename", "Alpha", "Renamed"]);
    let c = f.conn();
    let after: (String, String) = c
        .query_row(
            "SELECT id,source_path FROM linkable_items WHERE name='Renamed'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(after.0, id);
    assert_eq!(
        fs::canonicalize(PathBuf::from(after.1)).unwrap(),
        fs::canonicalize(&source).unwrap()
    );
    assert_eq!(
        fs::read(project.join(".agents/links.toml")).unwrap(),
        manifest
    );
    assert_eq!(fs::read_to_string(r1).unwrap(), "AssetOne");
}

#[test]
fn resource_basename_and_skill_alias_are_persisted() {
    let f = Fixture::new();
    f.migrate();
    let file = f.resource("LogicalName", "assets", None);
    let p = f.root.join("alias-skill");
    fs::create_dir(&p).unwrap();
    fs::write(p.join("SKILL.md"), "x").unwrap();
    f.ok(&[
        "skill",
        "add",
        p.to_str().unwrap(),
        "--name",
        "LogicalSkill",
        "--alias",
        "linked-skill",
    ]);
    let c = f.conn();
    assert_eq!(
        c.query_row(
            "SELECT default_link_name FROM linkable_items WHERE name='LogicalName'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        file.file_name().unwrap().to_str().unwrap()
    );
    assert_eq!(
        c.query_row(
            "SELECT default_link_name FROM linkable_items WHERE name='LogicalSkill'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "linked-skill"
    );
}

#[test]
fn recreated_names_receive_new_random_identifiers() {
    let f = Fixture::new();
    f.migrate();
    let source = f.skill("recreated");
    f.ok(&["group", "create", "recreated-group"]);
    let c = f.conn();
    let old_item: String = c
        .query_row("SELECT id FROM linkable_items", [], |r| r.get(0))
        .unwrap();
    let old_group: String = c
        .query_row("SELECT id FROM groups", [], |r| r.get(0))
        .unwrap();
    drop(c);
    f.ok(&["skill", "remove", "recreated"]);
    f.ok(&["group", "delete", "recreated-group"]);
    f.ok(&[
        "skill",
        "add",
        source.to_str().unwrap(),
        "--name",
        "recreated",
    ]);
    f.ok(&["group", "create", "recreated-group"]);
    let c = f.conn();
    assert_ne!(
        old_item,
        c.query_row("SELECT id FROM linkable_items", [], |r| r
            .get::<_, String>(0))
            .unwrap()
    );
    assert_ne!(
        old_group,
        c.query_row("SELECT id FROM groups", [], |r| r.get::<_, String>(0))
            .unwrap()
    );
}

#[test]
fn harness_mapping_cli_round_trip_case_uniqueness_and_required_protection() {
    let f = Fixture::new();
    f.migrate();
    f.ok(&[
        "harness",
        "mapping",
        "add",
        "claude",
        "AGENTS.md",
        ".claude/extra.md",
        "--kind",
        "file",
    ]);
    assert!(f
        .ok(&["harness", "mapping", "list", "claude"])
        .contains("extra.md"));
    f.fail(&[
        "harness",
        "mapping",
        "add",
        "claude",
        "AGENTS.md",
        ".CLAUDE/EXTRA.MD",
        "--kind",
        "file",
    ]);
    f.ok(&["harness", "mapping", "remove", "claude", ".claude/extra.md"]);
    assert_eq!(count(&f.conn(), "harness_mappings"), 2);
    f.fail(&["harness", "mapping", "remove", "claude", "CLAUDE.md"]);
    assert_eq!(count(&f.conn(), "harness_mappings"), 2);
}

#[cfg(windows)]
#[test]
fn database_check_identifies_readonly_without_altering_database() {
    let f = Fixture::new();
    f.migrate();
    let before = fs::read(f.db()).unwrap();
    let mut permissions = fs::metadata(f.db()).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(f.db(), permissions).unwrap();
    let output = f.run(&["db", "check"]);
    let mut permissions = fs::metadata(f.db()).unwrap().permissions();
    permissions.set_readonly(false);
    fs::set_permissions(f.db(), permissions).unwrap();
    assert!(
        !output.status.success(),
        "read-only database classified healthy: {}",
        output_text(&output)
    );
    assert!(
        has_case(&output_text(&output), "ReadOnly")
            || has_case(&output_text(&output), "read-only")
            || has_case(&output_text(&output), "read_only")
            || has_case(&output_text(&output), "readonly"),
        "{}",
        output_text(&output)
    );
    assert_eq!(fs::read(f.db()).unwrap(), before);
}

#[test]
fn group_deduplication_skip_unknown_atomicity_and_cascade() {
    let f = Fixture::new();
    f.migrate();
    let source = f.skill("member");
    f.ok(&["group", "create", "team"]);
    let id: String = f
        .conn()
        .query_row(
            "SELECT id FROM linkable_items WHERE name='member'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    f.ok(&["group", "add", "team", "member", &id, "MEMBER"]);
    assert_eq!(count(&f.conn(), "group_items"), 1);
    let msg = f.ok(&["group", "add", "team", "member"]);
    assert!(
        has_case(&msg, "skip")
            || has_case(&msg, "already")
            || msg.contains("跳过")
            || msg.contains("已"),
        "skip invisible: {msg}"
    );
    f.fail(&["group", "add", "team", "member", "unknown"]);
    assert_eq!(count(&f.conn(), "group_items"), 1);
    f.fail(&["group", "remove", "team", "member", "unknown"]);
    assert_eq!(count(&f.conn(), "group_items"), 1);
    f.ok(&["skill", "remove", "member"]);
    assert_eq!(count(&f.conn(), "group_items"), 0);
    assert_eq!(
        fs::read_to_string(source.join("SKILL.md")).unwrap(),
        "# member\n"
    );
    f.skill("other");
    f.ok(&["group", "add", "team", "other"]);
    f.ok(&["group", "delete", "team"]);
    assert_eq!(count(&f.conn(), "group_items"), 0);
    assert_eq!(count(&f.conn(), "linkable_items"), 1);
}

#[test]
fn group_add_and_remove_roll_back_when_a_later_write_fails() {
    for removal in [false, true] {
        let f = Fixture::new();
        f.migrate();
        f.skill("first");
        f.skill("second");
        f.ok(&["group", "create", "team"]);
        if removal {
            f.ok(&["group", "add", "team", "first", "second"]);
        }
        let c = f.conn();
        let id: String = c
            .query_row(
                "SELECT id FROM linkable_items WHERE name='second'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let trigger = if removal {
            format!("CREATE TRIGGER fail_second BEFORE DELETE ON group_items WHEN OLD.item_id='{id}' BEGIN SELECT RAISE(ABORT,'blind injected second failure'); END;")
        } else {
            format!("CREATE TRIGGER fail_second BEFORE INSERT ON group_items WHEN NEW.item_id='{id}' BEGIN SELECT RAISE(ABORT,'blind injected second failure'); END;")
        };
        c.execute_batch(&trigger).unwrap();
        drop(c);
        let msg = f.fail(&[
            "group",
            if removal { "remove" } else { "add" },
            "team",
            "first",
            "second",
        ]);
        assert!(has_case(&msg, "blind injected second failure"), "{msg}");
        assert_eq!(count(&f.conn(), "group_items"), if removal { 2 } else { 0 });
    }
}

#[test]
fn item_rename_failure_rolls_back_trigger_side_effects_and_keeps_identity() {
    let f = Fixture::new();
    f.migrate();
    let source = f.skill("before");
    f.ok(&["group", "create", "team"]);
    f.ok(&["group", "add", "team", "before"]);
    let c = f.conn();
    let id: String = c
        .query_row(
            "SELECT id FROM linkable_items WHERE name='before'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    c.execute_batch("CREATE TRIGGER fail_rename AFTER UPDATE OF name ON linkable_items BEGIN DELETE FROM group_items; SELECT RAISE(ABORT,'blind rename failure'); END;").unwrap();
    drop(c);
    assert!(f
        .fail(&["skill", "rename", "before", "after"])
        .contains("blind rename failure"));
    let c = f.conn();
    assert_eq!(count(&c, "group_items"), 1);
    assert_eq!(
        c.query_row(
            "SELECT id FROM linkable_items WHERE name='before'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        id
    );
    assert!(source.join("SKILL.md").is_file());
}

#[test]
fn failed_result_queries_roll_back_successful_writes_and_trigger_side_effects() {
    // These triggers deliberately let the write succeed but remove the row its
    // result query must read. A transaction ending before that read loses data.
    let f = Fixture::new();
    f.migrate();
    f.skill("before");
    let c = f.conn();
    c.execute_batch("CREATE TABLE blind_audit(value TEXT); CREATE TRIGGER vanish_rename AFTER UPDATE OF name ON linkable_items BEGIN INSERT INTO blind_audit VALUES('rename'); DELETE FROM linkable_items WHERE id=NEW.id; END;").unwrap();
    drop(c);
    f.fail(&["skill", "rename", "before", "after"]);
    let c = f.conn();
    assert_eq!(count(&c, "blind_audit"), 0);
    assert_eq!(count(&c, "linkable_items"), 1);
    assert_eq!(
        c.query_row("SELECT name FROM linkable_items", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "before"
    );
    c.execute_batch("DROP TRIGGER vanish_rename; CREATE TRIGGER vanish_group AFTER INSERT ON groups BEGIN INSERT INTO blind_audit VALUES('group'); DELETE FROM groups WHERE id=NEW.id; END;").unwrap();
    drop(c);
    f.fail(&["group", "create", "vanishing"]);
    assert_eq!(count(&f.conn(), "blind_audit"), 0);
}

#[test]
fn group_list_is_sorted_summary_and_show_keeps_members() {
    let f = Fixture::new();
    f.migrate();
    f.skill("member-secret-marker");
    f.ok(&["group", "create", "z-last"]);
    f.ok(&["group", "create", "a-empty"]);
    f.ok(&["group", "add", "z-last", "member-secret-marker"]);
    let list = f.ok(&["group", "list"]);
    assert!(
        list.find("a-empty").unwrap() < list.find("z-last").unwrap(),
        "{list}"
    );
    assert!(
        !list.contains("member-secret-marker"),
        "list expanded members: {list}"
    );
    assert!(
        list.contains('0') && list.contains('1'),
        "counts absent: {list}"
    );
    assert!(f
        .ok(&["group", "show", "z-last"])
        .contains("member-secret-marker"));
    let c = f.conn();
    let id: String = c
        .query_row("SELECT id FROM groups WHERE name='a-empty'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(
        id.starts_with("grp_") && id.len() == 20 && id[4..].bytes().all(|b| b.is_ascii_hexdigit()),
        "{id}"
    );
}

#[test]
fn backup_captures_committed_wal_and_never_overwrites() {
    let f = Fixture::new();
    f.migrate();
    let c = f.conn();
    c.pragma_update(None, "journal_mode", "WAL").unwrap();
    c.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    c.execute(
        "INSERT INTO groups VALUES('grp_wal_fixture','wal-visible',NULL,1,1)",
        [],
    )
    .unwrap();
    let dest = f.root.join("snapshot.db");
    f.ok(&["db", "backup", dest.to_str().unwrap()]);
    let snap = Connection::open(&dest).unwrap();
    assert_eq!(
        snap.query_row(
            "SELECT name FROM groups WHERE id='grp_wal_fixture'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "wal-visible"
    );
    drop(snap);
    let before = fs::read(&dest).unwrap();
    f.fail(&["db", "backup", dest.to_str().unwrap()]);
    assert_eq!(fs::read(&dest).unwrap(), before);
}

#[test]
fn database_check_reports_missing_schema_corrupt_busy_and_healthy_without_seed() {
    let missing = Fixture::new();
    let text = missing.fail(&["db", "check"]);
    assert!(has_case(&text, "missing"), "{text}");
    assert!(!missing.data.parent().unwrap().exists());
    let f = Fixture::new();
    f.migrate();
    let c = f.conn();
    c.execute("DELETE FROM harnesses", []).unwrap();
    drop(c);
    let before = fs::read(f.db()).unwrap();
    let text = f.ok(&["db", "check"]);
    assert!(
        has_case(&text, "HealthyWritable") || has_case(&text, "healthy"),
        "{text}"
    );
    assert_eq!(fs::read(f.db()).unwrap(), before);
    assert_eq!(count(&f.conn(), "harnesses"), 0);
    let c = f.conn();
    c.execute("DROP TABLE group_items", []).unwrap();
    drop(c);
    let before = fs::read(f.db()).unwrap();
    let text = f.fail(&["db", "check"]);
    assert!(
        has_case(&text, "SchemaMismatch") || has_case(&text, "schema"),
        "{text}"
    );
    assert_eq!(fs::read(f.db()).unwrap(), before);
    let corrupt = Fixture::new();
    fs::create_dir_all(&corrupt.data).unwrap();
    fs::write(corrupt.db(), "not sqlite").unwrap();
    let before = fs::read(corrupt.db()).unwrap();
    corrupt.fail(&["db", "check"]);
    assert_eq!(fs::read(corrupt.db()).unwrap(), before);
    let busy = Fixture::new();
    busy.migrate();
    let c = busy.conn();
    c.execute_batch("PRAGMA journal_mode=DELETE; BEGIN EXCLUSIVE;")
        .unwrap();
    let text = busy.fail(&["db", "check"]);
    assert!(
        has_case(&text, "LockedBusy") || has_case(&text, "locked") || has_case(&text, "busy"),
        "{text}"
    );
    c.execute_batch("ROLLBACK;").unwrap();
}

#[test]
fn verbose_doctor_includes_database_schema_diagnostics() {
    let f = Fixture::new();
    f.migrate();
    fs::create_dir(f.root.join(".agents")).unwrap();
    fs::write(f.root.join(".agents/links.toml"), "schema_version = 1\n").unwrap();
    f.conn().execute("DROP TABLE group_items", []).unwrap();
    let text = f.fail(&["--verbose", "doctor"]);
    assert!(has_case(&text, "database"), "{text}");
    assert!(
        text.contains("group_items"),
        "missing database diagnostic: {text}"
    );
}

#[test]
fn init_missing_database_fallback_preserves_conflict_and_never_creates_store() {
    let f = Fixture::new();
    fs::write(f.root.join("CLAUDE.md"), "real user text").unwrap();
    f.fail(&["init"]);
    assert!(f.root.join("AGENTS.md").is_file());
    assert!(f.root.join(".agents/skills").is_dir());
    assert_eq!(
        fs::read_to_string(f.root.join("CLAUDE.md")).unwrap(),
        "real user text"
    );
    assert!(!f.data.parent().unwrap().exists());
}

#[test]
fn init_respects_disabled_harness_and_rejects_old_schema() {
    let f = Fixture::new();
    f.migrate();
    f.ok(&["harness", "disable", "claude"]);
    let before = fs::read(f.db()).unwrap();
    f.ok(&["init"]);
    assert!(!f.root.join("CLAUDE.md").exists());
    assert_eq!(fs::read(f.db()).unwrap(), before);
    let old = Fixture::new();
    old.migrate();
    old.conn().pragma_update(None, "user_version", 2).unwrap();
    let text = old.fail(&["init"]);
    assert!(has_case(&text, "db migrate"), "{text}");
    assert!(!old.root.join("CLAUDE.md").exists());
}

#[test]
fn verbose_registry_times_are_utc_text() {
    let f = Fixture::new();
    f.migrate();
    f.skill("clock-item");
    f.conn()
        .execute("UPDATE linkable_items SET created_at=0,updated_at=0", [])
        .unwrap();
    let text = f.ok(&["--verbose", "skill", "show", "clock-item"]);
    assert!(
        text.contains("1970")
            && (text.contains("UTC") || text.contains('Z') || text.contains("+00:00")),
        "timestamp is not readable UTC: {text}"
    );
    assert!(!text.contains("unix:"), "{text}");
}

fn legacy_fixture(f: &Fixture, version: u32) {
    fs::create_dir_all(&f.data).unwrap();
    let c = f.conn();
    c.execute_batch(if version == 1 {
        include_str!("fixtures/schema_v1.sql")
    } else {
        include_str!("fixtures/schema_v2.sql")
    })
    .unwrap();
    c.execute("INSERT INTO frameworks VALUES('claude','claude','Claude Code',1,1,0,'unix:100','unix:200')",[]).unwrap();
    c.execute("INSERT INTO frameworks VALUES('custom-enabled','custom-enabled','Custom Enabled',0,0,1,'unix:110','unix:210')",[]).unwrap();
    c.execute("INSERT INTO framework_mappings VALUES('old-map','claude','AGENTS.md','custom.md','file',0,'unix:101','unix:201')",[]).unwrap();
    c.execute("INSERT INTO framework_mappings VALUES('custom-dir-map','custom-enabled','.agents/skills','.custom/skills','directory',0,'unix:111','unix:211')",[]).unwrap();
    c.execute("INSERT INTO linkable_items(id,kind,name,alias,source_path,source_kind,target_dir,description,created_at,updated_at) VALUES('old-item','skill','Original',NULL,?,'directory',NULL,'kept','unix:102','unix:202')",[f.root.join("missing-old-source").to_str().unwrap()]).unwrap();
    c.execute("INSERT INTO linkable_items(id,kind,name,alias,source_path,source_kind,target_dir,description,created_at,updated_at) VALUES('old-resource','resource','LogicalResource',NULL,?,'file','./assets/./images',NULL,'unix:103','unix:203')",[f.root.join("leaf.txt").to_str().unwrap()]).unwrap();
    c.execute("INSERT INTO linkable_items(id,kind,name,alias,source_path,source_kind,target_dir,description,created_at,updated_at) VALUES('old-resource-dir','resource','DirectoryResource','preferred-dir',?,'directory','./vendor/./packages','dir description','unix:106','unix:206')",[f.root.join("old-directory-source").to_str().unwrap()]).unwrap();
    c.execute("INSERT INTO linkable_items(id,kind,name,alias,source_path,source_kind,target_dir,description,created_at,updated_at) VALUES('old-skill-alias','skill','AliasedSkill','preferred-skill',?,'directory',NULL,NULL,'unix:107','unix:207')",[f.root.join("old-aliased-skill-source").to_str().unwrap()]).unwrap();
    c.execute(
        "INSERT INTO groups VALUES('old-group','OldGroup','description','unix:104','unix:204')",
        [],
    )
    .unwrap();
    c.execute(
        "INSERT INTO group_items VALUES('old-group','old-item','unix:105')",
        [],
    )
    .unwrap();
    c.execute(
        "INSERT INTO group_items VALUES('old-group','old-resource','unix:108')",
        [],
    )
    .unwrap();
    c.execute(
        "INSERT INTO group_items VALUES('old-group','old-resource-dir','unix:109')",
        [],
    )
    .unwrap();
    c.execute(
        "INSERT INTO group_items VALUES('old-group','old-skill-alias','unix:112')",
        [],
    )
    .unwrap();
    fs::create_dir(f.root.join(".agents")).unwrap();
    fs::write(
        f.root.join(".agents/links.toml"),
        r#"schema_version = 1
[[links]]
id = "legacy-registry-reference"
scope = "project"
framework_name = "registry"
item_id = "old-item"
item_name = "Original"
source_path = "missing-old-source"
link_path = ".agents/skills/Original"
link_kind = "directory"
provider_backend = "mock"
created_by_command = "link"
created_at = "unix:102"
updated_at = "unix:202"
"#,
    )
    .unwrap();
}

#[test]
fn v1_and_v2_migration_preserve_identifiers_relations_values_and_unix_times() {
    for version in [1, 2] {
        let f = Fixture::new();
        legacy_fixture(&f, version);
        let old_manifest = fs::read(f.root.join(".agents/links.toml")).unwrap();
        f.migrate();
        let c = f.conn();
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            3
        );
        let h: (String, i64, i64, i64) = c
            .query_row(
                "SELECT id,enabled,created_at,updated_at FROM harnesses WHERE name='claude'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(h, ("claude".into(), 0, 100, 200));
        let custom:(String,i64,i64,i64)=c.query_row("SELECT id,enabled,created_at,updated_at FROM harnesses WHERE name='custom-enabled'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
        assert_eq!(custom, ("custom-enabled".into(), 1, 110, 210));
        let m:(String,String,i64,i64)=c.query_row("SELECT harness_id,link_path,created_at,updated_at FROM harness_mappings WHERE id='old-map'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
        assert_eq!(m, ("claude".into(), "custom.md".into(), 101, 201));
        let custom_map:(String,String,String,String,i64,i64)=c.query_row("SELECT harness_id,source_path,link_path,link_kind,created_at,updated_at FROM harness_mappings WHERE id='custom-dir-map'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).unwrap();
        assert_eq!(
            custom_map,
            (
                "custom-enabled".into(),
                ".agents/skills".into(),
                ".custom/skills".into(),
                "directory".into(),
                111,
                211
            )
        );
        let i:(String,String,i64,i64)=c.query_row("SELECT name,default_link_name,created_at,updated_at FROM linkable_items WHERE id='old-item'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
        assert_eq!(i, ("Original".into(), "Original".into(), 102, 202));
        let r: (String, String) = c
            .query_row(
                "SELECT target_dir,default_link_name FROM linkable_items WHERE id='old-resource'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(r.0.replace('\\', "/"), "assets/images");
        assert_eq!(r.1, "leaf.txt");
        let directory:(String,String,String,String,String,i64,i64)=c.query_row("SELECT kind,alias,default_link_name,source_path,target_dir,created_at,updated_at FROM linkable_items WHERE id='old-resource-dir'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).unwrap();
        assert_eq!(
            directory,
            (
                "resource".into(),
                "preferred-dir".into(),
                "preferred-dir".into(),
                f.root.join("old-directory-source").to_str().unwrap().into(),
                "vendor/packages".into(),
                106,
                206
            )
        );
        let skill_alias:(String,String,String,i64,i64)=c.query_row("SELECT kind,alias,default_link_name,created_at,updated_at FROM linkable_items WHERE id='old-skill-alias'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap();
        assert_eq!(
            skill_alias,
            (
                "skill".into(),
                "preferred-skill".into(),
                "preferred-skill".into(),
                107,
                207
            )
        );
        assert_eq!(count(&c, "group_items"), 4);
        assert_eq!(c.query_row("SELECT created_at FROM group_items WHERE group_id='old-group' AND item_id='old-item'",[],|r|r.get::<_,i64>(0)).unwrap(),105);
        assert_eq!(
            c.query_row(
                "SELECT created_at,updated_at FROM groups WHERE id='old-group'",
                [],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
            )
            .unwrap(),
            (104, 204)
        );
        assert_eq!(
            c.query_row("PRAGMA foreign_key_check", [], |r| r.get::<_, String>(0))
                .optional()
                .unwrap(),
            None
        );
        assert_eq!(
            fs::read(f.root.join(".agents/links.toml")).unwrap(),
            old_manifest
        );
        let manifest = agent_linker::core::manifest::load_manifest(&f.root).unwrap();
        assert_eq!(manifest.links[0].item_id, "old-item");
        assert_eq!(manifest.links[0].created_at, 102);
        assert_eq!(manifest.links[0].updated_at, 202);
        assert_eq!(
            c.query_row(
                "SELECT name FROM linkable_items WHERE id=?",
                [&manifest.links[0].item_id],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            manifest.links[0].item_name
        );
        drop(c);
        f.ok(&["unlink", "old-item"]);
        assert!(agent_linker::core::manifest::load_manifest(&f.root)
            .unwrap()
            .links
            .is_empty());
    }
}

#[test]
fn v3_constraint_failure_rolls_back_entire_legacy_migration() {
    for version in [1, 2] {
        let f = Fixture::new();
        legacy_fixture(&f, version);
        let c = f.conn();
        c.execute("INSERT INTO linkable_items(id,kind,name,source_path,source_kind,target_dir,created_at,updated_at) VALUES('collision','resource','original','C:/missing.txt','file','assets','unix:1','unix:2')",[]).unwrap();
        drop(c);
        let before = fs::read(f.db()).unwrap();
        f.fail(&["db", "migrate"]);
        assert_eq!(
            before,
            fs::read(f.db()).unwrap(),
            "migration failure changed original database bytes"
        );
        let c = f.conn();
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            i64::from(version)
        );
        assert_eq!(count(&c, "frameworks"), 2);
        assert_eq!(count(&c, "linkable_items"), 5);
        assert_eq!(count(&c, "group_items"), 4);
        assert_eq!(
            c.query_row(
                "SELECT name FROM linkable_items WHERE id='collision'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "original"
        );
    }
}

#[test]
fn empty_existing_database_migrates_explicitly() {
    let f = Fixture::new();
    fs::create_dir_all(&f.data).unwrap();
    drop(Connection::open(f.db()).unwrap());
    f.migrate();
    assert_eq!(count(&f.conn(), "harnesses"), 1);
    f.ok(&["db", "check"]);
}

#[test]
fn seed_failure_rolls_back_all_prior_seed_inserts() {
    let f = Fixture::new();
    f.migrate();
    let c = f.conn();
    c.execute_batch("DELETE FROM harness_mappings; CREATE TABLE blind_seed_audit(value TEXT); CREATE TRIGGER abort_second_seed BEFORE INSERT ON harness_mappings WHEN (SELECT COUNT(*) FROM harness_mappings)=1 BEGIN SELECT RAISE(ABORT,'blind second seed failure'); END; CREATE TRIGGER record_first_seed AFTER INSERT ON harness_mappings BEGIN INSERT INTO blind_seed_audit VALUES(NEW.id); END;").unwrap();
    drop(c);
    let before = fs::read(f.db()).unwrap();
    let error = f.fail(&["db", "migrate"]);
    assert!(error.contains("blind second seed failure"), "{error}");
    let c = f.conn();
    assert_eq!(count(&c, "harnesses"), 1);
    assert_eq!(count(&c, "harness_mappings"), 0);
    assert_eq!(count(&c, "blind_seed_audit"), 0);
    drop(c);
    assert_eq!(fs::read(f.db()).unwrap(), before);
}

#[test]
fn group_list_does_not_deserialize_corrupt_member_enums() {
    let f = Fixture::new();
    f.migrate();
    f.skill("invalid-enum-member");
    f.ok(&["group", "create", "summary-only"]);
    f.ok(&["group", "add", "summary-only", "invalid-enum-member"]);
    let c = f.conn();
    c.execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE linkable_items SET source_kind='blind-invalid-kind';").unwrap();
    drop(c);
    let list = f.ok(&["group", "list"]);
    assert!(list.contains("summary-only"), "{list}");
    assert!(list.contains('1'), "{list}");
    assert!(!list.contains("invalid-enum-member"), "{list}");
    f.fail(&["group", "show", "summary-only"]);
}

#[test]
fn item_and_group_exact_identifiers_win_over_id_shaped_names() {
    for primary_kind in ["skill", "resource"] {
        for secondary_kind in ["skill", "resource"] {
            let f = Fixture::new();
            f.migrate();
            if primary_kind == "skill" {
                f.skill("primary-marker");
            } else {
                f.resource("primary-marker", "assets", None);
            }
            let id: String = f
                .conn()
                .query_row(
                    "SELECT id FROM linkable_items WHERE name='primary-marker'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            if secondary_kind == "skill" {
                f.skill(&id);
            } else {
                f.resource(&id, "other-assets", None);
            }
            let output = f.ok(&[primary_kind, "show", &id]);
            assert!(
                output.contains("primary-marker"),
                "exact ID resolved to an ID-shaped name: {output}"
            );
            if primary_kind != secondary_kind {
                f.fail(&[secondary_kind, "show", &id]);
            }
            f.ok(&["group", "create", "primary-group-marker"]);
            let gid: String = f
                .conn()
                .query_row(
                    "SELECT id FROM groups WHERE name='primary-group-marker'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            f.ok(&["group", "create", &gid]);
            let output = f.ok(&["group", "show", &gid]);
            assert!(
                output.contains("primary-group-marker"),
                "exact group ID resolved to an ID-shaped name: {output}"
            );
            f.ok(&["group", "add", &gid, &id]);
            let member: (String, String) = f
                .conn()
                .query_row("SELECT group_id,item_id FROM group_items", [], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })
                .unwrap();
            assert_eq!(member, (gid, id));
        }
    }
}

#[test]
fn item_alias_does_not_participate_in_identifier_lookup() {
    let f = Fixture::new();
    f.migrate();
    let source = f.root.join("aliased-source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("SKILL.md"), "skill").unwrap();
    f.ok(&[
        "skill",
        "add",
        source.to_str().unwrap(),
        "--name",
        "lookup-name",
        "--alias",
        "alias-only-marker",
    ]);
    f.fail(&["skill", "show", "alias-only-marker"]);
    assert!(f
        .ok(&["skill", "show", "lookup-name"])
        .contains("lookup-name"));
}
