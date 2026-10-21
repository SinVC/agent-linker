# Changelog

## Unreleased

- Renamed the `framework` command to `harness` with no compatibility alias; updated the Rust module, reports, diagnostics, and current documentation to use Harness.
- Clarified that `claude` identifies Claude Code; harness settings control compatibility definitions, not tool processes.
- Preserved existing SQLite tables, manifest schema, IDs, enabled flags, and custom mappings across the terminology change.

## 0.1.0 - MVP release candidate

- Added the `aglink` Rust CLI with `init`, registry, group, link/unlink, status, clean, doctor, config, database, and framework commands.
- Added safe symlink semantics: correct links are idempotent, wrong symlinks require `--force`, and real files/directories are never overwritten or deleted by link operations.
- Added Windows Broker as the default Windows symlink backend, with standard-library symlink creation available only as an explicit fallback in code.
- Added Unix standard-library symlink support for Linux and macOS code paths.
- Added global SQLite storage for config, frameworks, mappings, linkable items, and groups.
- Added project-local `.agents/links.toml` as the source of truth for managed project links.
- Added structured `status --json` output and verbose `doctor` diagnostics.

### Verification Status

- Windows: smoke-tested on Windows with `WinSymlinksBroker`; temporary `init` created real symbolic links and recorded `provider_backend = "windows-broker"`.
- Linux: smoke-tested on local Ubuntu WSL2 for the Unix standard-library symlink provider and non-Windows default backend.
- macOS: not yet smoke-tested; it uses the Unix standard-library code path, but release notes should not describe macOS as verified until a macOS run is completed.
- Database paths: unit tests cover Windows, macOS, Linux, Linux fallback, portable mode, and `AGLINK_DATA_DIR` override through injected path contexts.
