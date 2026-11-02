use super::{
    clean_command, command_from_cli, link_command, parse_error_exit_code, unlink_command,
    write_report, Cli, CliCleanArgs, CliLinkArgs, CliUnlinkArgs, OutputOptions,
};
use crate::{
    commands::{Command, CommandReport, LinkCommand, UnlinkTarget},
    core::{
        manifest::LinkRecord,
        project_links::{CleanMode, StatusEntry, StatusReport},
        symlink::{LinkKind, LinkStatus},
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
        let command = command_from_cli(Cli::try_parse_from(args).unwrap());
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
    let error = Cli::try_parse_from(["aglink", "link"]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
}

#[test]
fn link_group_rejects_item_only_flags() {
    let error =
        Cli::try_parse_from(["aglink", "link", "--group", "daily", "--as", "writer"]).unwrap_err();
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
    });

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
    let error =
        Cli::try_parse_from(["aglink", "unlink", "writer", "--group", "daily"]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
}

#[test]
fn unlink_all_uses_none_identifier() {
    let command = unlink_command(CliUnlinkArgs {
        name: None,
        group: None,
        all: true,
        dry_run: true,
    });

    assert!(matches!(command.target, UnlinkTarget::Item(None)));
    assert!(command.options.dry_run);
}

#[test]
fn clean_modes_are_exclusive() {
    let error =
        Cli::try_parse_from(["aglink", "clean", "--broken", "--missing-source"]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::ArgumentConflict);

    let command = clean_command(CliCleanArgs {
        broken: false,
        missing_source: true,
        dry_run: true,
    });
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
                    provider_backend: "mock".into(),
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

#[test]
fn both_group_command_spellings_normalize_to_identical_commands() {
    for (first, second) in [
        (
            vec!["aglink", "link", "--group", "daily", "--dry-run", "--force"],
            vec!["aglink", "group", "link", "daily", "--dry-run", "--force"],
        ),
        (
            vec!["aglink", "unlink", "--group", "daily", "--dry-run"],
            vec!["aglink", "group", "unlink", "daily", "--dry-run"],
        ),
    ] {
        let first = command_from_cli(Cli::try_parse_from(first).unwrap());
        let second = command_from_cli(Cli::try_parse_from(second).unwrap());
        assert_eq!(format!("{first:?}"), format!("{second:?}"));
    }
}
