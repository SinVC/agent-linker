use super::{backup_database, check_database, latest_schema_version, Store};
use crate::core::{
    paths::{DbPathReason, DbPathResolution},
    test_support::TestDir,
};
use std::fs;

#[test]
fn windows_install_backup_default_stays_under_the_install_root() {
    let temp = TestDir::new("installed-backup-default");
    let resolution = DbPathResolution {
        path: temp.path().join("data/agent-linker.db"),
        reason: DbPathReason::WindowsInstall,
    };
    assert_eq!(
        super::default_backup_path(&resolution).parent().unwrap(),
        temp.path().join("backups")
    );
}
#[test]
fn readonly_queries_do_not_initialize_or_migrate() {
    let temp = TestDir::new("readonly-schema");
    let resolution = DbPathResolution {
        path: temp.path().join("missing/agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    assert!(Store::open_ro(&resolution)
        .err()
        .unwrap()
        .to_string()
        .contains("db migrate"));
    assert!(!check_database(&resolution).unwrap().exists);
    assert!(!resolution.path.parent().unwrap().exists());
    fs::create_dir_all(resolution.path.parent().unwrap()).unwrap();
    let c = rusqlite::Connection::open(&resolution.path).unwrap();
    c.execute_batch("PRAGMA user_version=1;").unwrap();
    assert!(Store::open_ro(&resolution)
        .err()
        .unwrap()
        .to_string()
        .contains("db migrate"));
    assert_eq!(super::schema_version(&c).unwrap(), 1);
    c.execute_batch("PRAGMA user_version=999;").unwrap();
    assert!(Store::open_ro(&resolution)
        .err()
        .unwrap()
        .to_string()
        .contains("upgrade aglink"));
    assert_eq!(super::schema_version(&c).unwrap(), 999);
}
#[test]
fn queries_are_readonly_even_while_another_writer_holds_the_database() {
    let temp = TestDir::new("readonly-registry");
    let resolution = DbPathResolution {
        path: temp.path().join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    Store::migrate(&resolution).unwrap();
    let c = rusqlite::Connection::open(&resolution.path).unwrap();
    c.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let reader = Store::open_ro(&resolution).unwrap();
    assert!(reader
        .connection()
        .execute("DELETE FROM groups", [])
        .unwrap_err()
        .to_string()
        .contains("readonly"));
    assert!(reader.list_group_summaries().unwrap().is_empty());
    assert_eq!(reader.list_harnesses().unwrap().len(), 1);
    assert_eq!(reader.list_all_mappings().unwrap().len(), 2);
    let harnesses = reader.list_harnesses().unwrap();
    assert!(harnesses[0].enabled);
    assert_eq!(harnesses[0].mappings.len(), 2);
    assert!(reader
        .list_items(crate::core::linkable::LinkableItemType::Skill)
        .unwrap()
        .is_empty());
    c.execute_batch("ROLLBACK;").unwrap();
}
#[test]
fn backup_snapshots_committed_wal_without_changing_source_schema() {
    let temp = TestDir::new("wal-snapshot");
    let resolution = DbPathResolution {
        path: temp.path().join("source.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    assert!(backup_database(&resolution, Some(&temp.path().join("missing.db"))).is_err());
    assert!(!resolution.path.exists());
    let c = rusqlite::Connection::open(&resolution.path).unwrap();
    c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE legacy(value TEXT); PRAGMA user_version=1; INSERT INTO legacy VALUES ('committed-wal');").unwrap();
    let dest = temp.path().join("snapshot.db");
    backup_database(&resolution, Some(&dest)).unwrap();
    let snap = rusqlite::Connection::open(&dest).unwrap();
    assert_eq!(super::schema_version(&snap).unwrap(), 1);
    assert_eq!(
        snap.query_row("SELECT value FROM legacy", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "committed-wal"
    );
    assert_eq!(super::schema_version(&c).unwrap(), 1);
    assert!(c.prepare("SELECT * FROM config").is_err());
    assert!(backup_database(&resolution, Some(&dest)).is_err());
}
#[test]
fn migration_seeds_claude_harness_and_mappings() {
    let temp = TestDir::new("db-migrate");
    let resolution = DbPathResolution {
        path: temp.path().join("store/agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    let report = Store::migrate(&resolution).unwrap();
    assert_eq!(report.previous_version, 0);
    assert_eq!(report.current_version, latest_schema_version());
    assert!(check_database(&resolution).unwrap().is_ok());
    let store = Store::open_ro(&resolution).unwrap();
    let hs = store.list_harnesses().unwrap();
    assert_eq!(hs.len(), 1);
    assert_eq!(hs[0].id, "claude");
    assert!(hs[0].enabled);
    assert_eq!(hs[0].mappings.len(), 2);
}
#[test]
fn migration_seed_preserves_enabled_state() {
    let temp = TestDir::new("db-seed-preserves-enabled");
    let resolution = DbPathResolution {
        path: temp.path().join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    Store::migrate(&resolution).unwrap();
    Store::open_rw(&resolution)
        .unwrap()
        .disable_harness("claude")
        .unwrap();
    Store::migrate(&resolution).unwrap();
    assert!(
        !Store::open_ro(&resolution)
            .unwrap()
            .list_harnesses()
            .unwrap()[0]
            .enabled
    );
    Store::open_rw(&resolution)
        .unwrap()
        .enable_harness("claude")
        .unwrap();
    assert!(
        Store::open_ro(&resolution)
            .unwrap()
            .list_harnesses()
            .unwrap()[0]
            .enabled
    );
}
#[test]
fn config_is_absent_and_database_backup_does_not_overwrite() {
    let temp = TestDir::new("db-config-backup");
    let resolution = DbPathResolution {
        path: temp.path().join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    Store::migrate(&resolution).unwrap();
    let store = Store::open_ro(&resolution).unwrap();
    assert!(store.connection().prepare("SELECT * FROM config").is_err());
    let dest = temp.path().join("backup.db");
    let report = backup_database(&resolution, Some(&dest)).unwrap();
    assert_eq!(report.backup_path, dest);
    assert!(report.bytes > 0);
    assert!(report.backup_path.is_file());
    assert!(backup_database(&resolution, Some(&report.backup_path))
        .unwrap_err()
        .to_string()
        .contains("already exists"));
}

#[test]
fn store_connections_enforce_readonly_and_foreign_key_pragmas() {
    let temp = TestDir::new("connection-pragmas");
    let resolution = DbPathResolution {
        path: temp.path().join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    Store::migrate(&resolution).unwrap();
    let ro = Store::open_ro(&resolution).unwrap();
    assert_eq!(
        ro.connection()
            .query_row("PRAGMA query_only", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(ro
        .connection()
        .execute("INSERT INTO groups VALUES('bad','bad',NULL,0,0)", [])
        .is_err());
    drop(ro);
    let rw = Store::open_rw(&resolution).unwrap();
    assert_eq!(
        rw.connection()
            .query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(rw
        .connection()
        .execute(
            "INSERT INTO group_items VALUES('unknown-group','unknown-item',0)",
            []
        )
        .is_err());
}
