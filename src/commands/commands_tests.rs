use super::{default_item_name, CommandReport};
use std::path::PathBuf;

#[test]
fn default_item_name_uses_path_stem() {
    assert_eq!(
        default_item_name(&PathBuf::from("notes.md")).unwrap(),
        "notes"
    );
    assert_eq!(
        default_item_name(&PathBuf::from("skill-dir")).unwrap(),
        "skill-dir"
    );
}

#[test]
fn doctor_report_controls_success_status() {
    let report = CommandReport::Doctor(crate::core::doctor::DoctorReport {
        checks: Vec::new(),
        ok: false,
    });
    assert!(!report.is_success());
}
