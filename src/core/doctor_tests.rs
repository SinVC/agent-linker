use crate::core::{
    doctor::{
        create_exclusive_probe_dir_in, doctor_project, doctor_project_with_probe, DoctorCheck,
    },
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

fn mock_ok_broker(_: bool) -> DoctorCheck {
    DoctorCheck {
        name: "windows-broker".to_string(),
        ok: true,
        summary: "available".to_string(),
        details: Vec::new(),
    }
}

fn mock_failed_broker(_: bool) -> DoctorCheck {
    DoctorCheck {
        name: "windows-broker".to_string(),
        ok: false,
        summary: "broker_unavailable".to_string(),
        details: vec!["injected probe failure".to_string()],
    }
}

#[test]
fn doctor_public_entry_runs_with_default_probe() {
    let temp_dir = TestDir::new("doctor-public-entry");
    fs::create_dir(temp_dir.path().join(".agents")).unwrap();
    save_empty_manifest(temp_dir.path());
    let resolution = database(&temp_dir);

    let report = doctor_project(temp_dir.path(), &resolution, true);
    assert!(report
        .checks
        .iter()
        .any(|check| check.name == "database" && check.ok));
    assert!(report
        .checks
        .iter()
        .any(|check| check.name == "manifest" && check.ok));
    assert!(report
        .checks
        .iter()
        .any(|check| check.name == "harness-adapter" && check.ok));
    assert!(report
        .checks
        .iter()
        .any(|check| check.name == "symlink-backend" && check.ok));
}

#[test]
fn doctor_reports_database_manifest_harness_and_backend() {
    let temp_dir = TestDir::new("doctor");
    fs::create_dir(temp_dir.path().join(".agents")).unwrap();
    save_empty_manifest(temp_dir.path());
    let resolution = database(&temp_dir);

    let report = doctor_project_with_probe(temp_dir.path(), &resolution, true, mock_ok_broker);

    assert!(report.ok, "doctor report must be healthy: {report:#?}");
    assert!(report
        .checks
        .iter()
        .any(|check| check.name == "database" && check.ok));
    assert!(report
        .checks
        .iter()
        .any(|check| check.name == "manifest" && check.ok));
    assert!(report
        .checks
        .iter()
        .any(|check| check.name == "harness-adapter" && check.ok));
    assert!(report
        .checks
        .iter()
        .any(|check| check.name == "symlink-backend" && check.ok));
    assert!(report
        .checks
        .iter()
        .any(|check| check.name == "windows-broker" && check.ok));
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
    let report = doctor_project_with_probe(project.path(), &resolution, true, mock_ok_broker);
    assert!(
        report.ok,
        "valid empty harness registry must remain healthy: {report:#?}"
    );
    let harness_check = report
        .checks
        .iter()
        .find(|check| check.name == "harness-adapter")
        .expect("harness-adapter check must be present");
    assert!(harness_check.ok);
    assert!(harness_check.summary.contains("0 harnesses"));
}

#[test]
fn doctor_report_aggregates_check_status_correctly() {
    let project = TestDir::new("doctor-aggregate");
    fs::create_dir(project.path().join(".agents")).unwrap();
    save_empty_manifest(project.path());
    let resolution = database(&project);

    let failed_report =
        doctor_project_with_probe(project.path(), &resolution, true, mock_failed_broker);
    assert!(
        !failed_report.ok,
        "report.ok must be false when broker probe fails: {failed_report:#?}"
    );
    assert!(
        failed_report
            .checks
            .iter()
            .any(|check| check.name == "windows-broker" && !check.ok),
        "failed broker check must be preserved in checks"
    );

    let ok_report = doctor_project_with_probe(project.path(), &resolution, true, mock_ok_broker);
    assert!(
        ok_report.ok,
        "report.ok must be true when all checks succeed: {ok_report:#?}"
    );
}

#[test]
fn probe_dir_lifecycle_skips_occupied_and_preserves_sentinel_file() {
    let test_dir = TestDir::new("probe-lifecycle-occupied");
    let occupied = test_dir.path().join("occupied-dir");
    fs::create_dir(&occupied).unwrap();
    let sentinel = occupied.join("sentinel.txt");
    fs::write(&sentinel, "preserve-me").unwrap();

    let mut names = vec!["occupied-dir".to_string(), "fresh-dir".to_string()].into_iter();
    let guard = create_exclusive_probe_dir_in(
        test_dir.path(),
        || names.next().unwrap_or_else(|| "fallback".to_string()),
        5,
    )
    .expect("must succeed on second attempt after collision");

    assert_eq!(guard.path(), test_dir.path().join("fresh-dir"));
    assert!(guard.path().is_dir());
    assert_eq!(fs::read_to_string(&sentinel).unwrap(), "preserve-me");

    drop(guard);
    assert!(!test_dir.path().join("fresh-dir").exists());
    assert!(sentinel.is_file());
    assert_eq!(fs::read_to_string(&sentinel).unwrap(), "preserve-me");
}

#[test]
fn probe_dir_lifecycle_exhausts_retries_without_deleting_occupied_sentinel() {
    let test_dir = TestDir::new("probe-lifecycle-exhaust");
    let occupied = test_dir.path().join("always-occupied");
    fs::create_dir(&occupied).unwrap();
    let sentinel = occupied.join("sentinel.txt");
    fs::write(&sentinel, "never-delete-me").unwrap();

    let result =
        create_exclusive_probe_dir_in(test_dir.path(), || "always-occupied".to_string(), 3);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert!(sentinel.is_file());
    assert_eq!(fs::read_to_string(&sentinel).unwrap(), "never-delete-me");
}

#[test]
fn probe_dir_guard_cleans_up_on_early_failure() {
    let test_dir = TestDir::new("probe-lifecycle-failure");
    let created_path;
    {
        let guard =
            create_exclusive_probe_dir_in(test_dir.path(), || "fail-test-dir".to_string(), 1)
                .expect("should create probe dir");
        created_path = guard.path().to_path_buf();
        assert!(created_path.is_dir());

        let res: Result<(), &str> = Err("simulated probe failure");
        assert!(res.is_err());
    }

    assert!(
        !created_path.exists(),
        "probe dir must be cleaned up when guard is dropped after failure"
    );
}
