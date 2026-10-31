use super::{
    clean_command, command_from_cli, link_command, parse_error_exit_code, unlink_command,
    write_report, Cli, CliCleanArgs, CliLinkArgs, CliUnlinkArgs, OutputOptions,
};
use crate::{
    commands::{Command, CommandReport, LinkCommand, UnlinkTarget},
    core::{
        manifest::LinkRecord,
        project_links::{CleanMode, StatusEntry, StatusReport},
        symlink::{LinkKind, LinkStatus, SymlinkBackend},
    },
};
use clap::{error::ErrorKind, CommandFactory, Parser};
use std::{path::PathBuf, process::ExitCode};

#[test]
fn help_exits_successfully() {
    let error = Cli::try_parse_from(["aglink", "--help"]).unwrap_err();
    assert_eq!(parse_error_exit_code(&error), ExitCode::SUCCESS);
}

#[test]
fn harness_command_replaces_framework_in_parser_and_help() {
    let subcommands: &[&[&str]] = &[
        &["list"],
        &["show", "claude"],
        &["enable", "claude"],
        &["disable", "claude"],
        &["mapping", "list"],
        &["mapping", "list", "claude"],
        &[
            "mapping",
            "add",
            "claude",
            "AGENTS.md",
            ".claude/extra.md",
            "--kind",
            "file",
        ],
        &["mapping", "remove", "claude", ".claude/extra.md"],
    ];
    for subcommand in subcommands {
        let args = ["aglink", "harness"]
            .into_iter()
            .chain(subcommand.iter().copied());
        let command = command_from_cli(Cli::try_parse_from(args).unwrap()).unwrap();
        assert!(matches!(command, Command::Harness(_)));
    }

    let old_command = Cli::try_parse_from(["aglink", "framework", "list"]).unwrap_err();
    assert_eq!(old_command.kind(), ErrorKind::InvalidSubcommand);
    let help = Cli::command().render_help().to_string();
    assert!(help.contains("harness"));
    assert!(!help.contains("framework"));
}

#[test]
fn link_requires_item_or_group() {
    let error = link_command(CliLinkArgs {
        name: None,
        group: None,
        as_name: None,
        target_dir: None,
        dry_run: false,
        force: false,
    })
    .unwrap_err();
    assert!(error.to_string().contains("usage"));
}

#[test]
fn link_group_rejects_item_only_flags() {
    let error = link_command(CliLinkArgs {
        name: None,
        group: Some("daily".to_string()),
        as_name: Some("writer".to_string()),
        target_dir: None,
        dry_run: false,
        force: false,
    })
    .unwrap_err();
    assert!(error.to_string().contains("--as"));
}

#[test]
fn link_item_preserves_dry_run_and_force() {
    let command = link_command(CliLinkArgs {
        name: Some("writer".to_string()),
        group: None,
        as_name: Some("helper".to_string()),
        target_dir: Some(PathBuf::from("docs")),
        dry_run: true,
        force: true,
    })
    .unwrap();

    assert!(matches!(
        command,
        LinkCommand::Item {
            request,
            options
        } if request.identifier == "writer"
            && request.link_name_override.as_deref() == Some("helper")
            && options.dry_run
            && options.force
    ));
}

#[test]
fn unlink_requires_exactly_one_target() {
    let error = unlink_command(CliUnlinkArgs {
        name: Some("writer".to_string()),
        group: Some("daily".to_string()),
        all: false,
        dry_run: false,
    })
    .unwrap_err();
    assert!(error.to_string().contains("usage"));
}

#[test]
fn unlink_all_uses_none_identifier() {
    let command = unlink_command(CliUnlinkArgs {
        name: None,
        group: None,
        all: true,
        dry_run: true,
    })
    .unwrap();

    assert!(matches!(command.target, UnlinkTarget::Item(None)));
    assert!(command.options.dry_run);
}

#[test]
fn clean_modes_are_exclusive() {
    let error = clean_command(CliCleanArgs {
        broken: true,
        missing_source: true,
        dry_run: false,
    })
    .unwrap_err();
    assert!(error.to_string().contains("usage"));

    let command = clean_command(CliCleanArgs {
        broken: false,
        missing_source: true,
        dry_run: true,
    })
    .unwrap();
    assert_eq!(command.options.mode, CleanMode::MissingSource);
    assert!(command.options.dry_run);
}

#[test]
fn status_json_uses_structured_escaping() {
    let report = CommandReport::Status {
        json: true,
        report: StatusReport {
            project_root: PathBuf::from("project\"root"),
            manifest_path: PathBuf::from(".agents").join("links.toml"),
            entries: vec![StatusEntry {
                record: LinkRecord {
                    id: "id\"1".to_string(),
                    origin: crate::core::manifest::LinkOrigin::Item,
                    harness_name: None,
                    item_id: "item\\1".to_string(),
                    item_name: "writer\nskill".to_string(),
                    source_path: PathBuf::from("source\"path"),
                    link_path: PathBuf::from(".agents").join("skills").join("writer"),
                    link_kind: LinkKind::Directory,
                    provider_backend: SymlinkBackend::Mock,
                    created_by_command: "link".to_string(),
                    created_at: 1,
                    updated_at: 1,
                },
                absolute_source_path: PathBuf::from("source"),
                absolute_link_path: PathBuf::from("link"),
                status: LinkStatus::Missing,
            }],
        },
    };

    let mut output = Vec::new();
    write_report(
        &mut output,
        &report,
        OutputOptions {
            quiet: true,
            verbose: false,
        },
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(value["project_root"], "project\"root");
    assert_eq!(value["links"][0]["id"], "id\"1");
    assert_eq!(value["links"][0]["item_name"], "writer\nskill");
    assert_eq!(value["links"][0]["status"], "missing");
}
