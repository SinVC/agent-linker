use crate::core::{
    doctor::doctor_project,
    manifest::{save_manifest, Manifest},
    paths::{DbPathReason, DbPathResolution},
    store::Store,
    test_support::TestDir,
};
use std::{fs, path::Path};
fn database(temp_dir: &TestDir) -> DbPathResolution {
    let resolution = DbPathResolution {
        path: temp_dir.path().join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    };
    Store::migrate(&resolution).unwrap();
    resolution
}
fn save_empty_manifest(root: &Path) {
    save_manifest(root, &Manifest::empty()).unwrap();
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
