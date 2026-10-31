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
    let report = CommandReport::Doctor(crate::core::project_links::DoctorReport {
        checks: Vec::new(),
        ok: false,
    });
    assert!(!report.is_success());
}

#[test]
fn command_layer_does_not_use_forbidden_low_level_apis() {
    let source = include_str!("mod.rs");
    for forbidden in [
        concat!("rusq", "lite"),
        concat!("std::", "fs"),
        concat!("std::", "os::"),
        concat!(".agents/", "links.toml"),
        concat!("load_", "manifest"),
        concat!("save_", "manifest"),
        concat!("ensure_", "symlink"),
        concat!("symlink_", "file"),
        concat!("symlink_", "dir"),
        concat!("read_", "link("),
    ] {
        assert!(
            !source.contains(forbidden),
            "commands layer must not contain `{forbidden}`"
        );
    }
}
