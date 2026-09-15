# Release Checklist

Use this checklist before tagging an Agent Linker release.

## Version

- Current planned MVP version: `0.1.0`.
- Confirm `Cargo.toml` uses the intended version.
- Keep `publish = false` until crates.io publishing is explicitly prepared; the current Windows Broker client is a git dependency.

## Required Commands

Run these from the repository root:

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
cargo run --locked -- --help
```

## Platform Smoke Tests

Windows:

- Install and start the `WinSymlinksBroker` service.
- Run `aglink --verbose doctor` and verify the `symlink-backend` check reports `windows-broker`.
- In a temporary project with a temporary `AGLINK_DATA_DIR`, run `aglink db migrate`, `aglink init`, and `aglink status --json`.
- Verify `CLAUDE.md` and `.claude/skills` are real symlinks, not real files/directories or shortcuts.
- Verify `.agents/links.toml` and `status --json` record `provider_backend = "windows-broker"`.

Linux:

- Run the Rust test suite, or at minimum the Unix symlink tests, on Linux or WSL2:

```bash
cargo test --locked core::symlink::tests::std_provider_uses_unix_symlinks
cargo test --locked core::symlink::tests::default_provider_uses_std_backend_off_windows
```

macOS:

- Run the Linux/Unix symlink checks on macOS before claiming macOS as verified.
- Until that has happened, describe macOS support as sharing the Unix standard-library code path but not yet release-smoke-tested.

## Database Path Checks

Verify `aglink db path` for:

- Default platform path.
- `AGLINK_DATA_DIR` override.
- Portable mode by placing `agent-linker.db` next to the executable and leaving `AGLINK_DATA_DIR` unset.

The unit test `core::db::tests::database_path_uses_platform_defaults` covers Windows, macOS, Linux, and Linux fallback path calculation through injected contexts; still smoke-test real CLI output on any platform included in release notes.

## Temporary Project Self-Test

In a fresh temporary project:

```bash
aglink db migrate
aglink init
aglink status
aglink status --json
aglink --verbose doctor
```

Then register and link at least one temporary skill:

```bash
aglink skill add <path-to-temp-skill> --name stage8-smoke
aglink link stage8-smoke
aglink unlink stage8-smoke
```

Confirm real files are never overwritten and managed links are removed only through manifest records.
