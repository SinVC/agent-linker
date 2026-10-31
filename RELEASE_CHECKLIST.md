# Release Checklist

Use this checklist before tagging an Agent Linker release.

## Version

- Current planned MVP version: `0.1.0`.
- Confirm `Cargo.toml` uses the intended version.
- Keep `publish = false` until crates.io publishing is explicitly prepared; the current Windows Broker client is a git dependency.
- Confirm `win-symlinks-client` is pinned to the intended commit in `Cargo.toml`.
- Configure release repository variables:
  - `WIN_SYMLINKS_BROKER_URL`
  - `WIN_SYMLINKS_BROKER_SHA256`

## Required Commands

Run these from the repository root:

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
cargo run --locked -- --help
```

## Release Packaging

- Push a version tag such as `v0.1.0`; GitHub Actions builds and publishes release assets.
- Confirm Windows release archives include `broker/win-symlinks-broker.exe`.
- Confirm Windows release archives include `install-receipt.json` and do not include `VERSION.json`.
- Confirm Windows release archives do not include `LICENSES/` or `THIRD_PARTY_NOTICES.md`.
- Confirm `aglink-v<version>-x86_64-pc-windows-msvc-portable.zip` contains an initialized `agent-linker.db` next to `aglink.exe`, and queries work without a first-run migration.
- Confirm release assets include:
  - `aglink-installer.ps1`
  - `aglink-uninstall.ps1`
  - `aglink-installer.sh`
  - `aglink-uninstall.sh`
  - `SHA256SUMS`

## Platform Smoke Tests

Windows:

- Run `powershell -NoProfile -File packaging/windows/test-install.ps1` after building the CLI; add `-ProgramFiles` for the isolated protected-directory ACL fixture. These use offline packages and never install/remove the real Broker. `-CrossEntryGate <path>` exposes a temporary rendezvous record for separate Agent verification before continuing the same fixture.
- Install with `aglink-installer.ps1`; confirm it installs and starts the `WinSymlinksBroker` service when absent.
- Run `aglink --verbose doctor` and verify the `symlink-backend` check reports `windows-broker`.
- In a temporary project with a temporary `AGLINK_DATA_DIR`, run `aglink db migrate`, `aglink init`, and `aglink status --json`.
- Verify `CLAUDE.md` and `.claude/skills` are real symlinks, not real files/directories or shortcuts.
- Verify `.agents/links.toml` and `status --json` record `provider_backend = "windows-broker"`.
- Run `aglink-uninstall.ps1` and verify the binary and PATH entry are removed while data is retained.
- Run `aglink-uninstall.ps1 -Purge` in a disposable environment and verify `agent-linker.db` is removed without deleting unrelated directories.
- Use an isolated Program Files fixture: verify the initiating ordinary user can write `data/` and `backups/` but cannot change `bin/`. Also test `-NoBroker`, a writable custom root without UAC, and nonzero initialization failure status.
- Without `AGLINK_DATA_DIR`, create unique groups from an ordinary terminal and an Agent outside-sandbox command; immediately read/delete each from the other entry point and compare `db path`. Change `LOCALAPPDATA` and verify the normal installation target stays fixed.
- Verify sandbox queries and link dry-runs on an initialized read-only database; genuine access denials must fail without alternative databases.
- Verify upgrade and default uninstall preserve records, backups, and retained Broker files/scripts; verify purge cleans the new layout. Import/backup tests must preserve source schema and committed WAL data, refuse existing targets, and never create missing sources.

Linux:

- Run `sh packaging/unix/test-install.sh <absolute-path-to-linux-aglink>` for offline installer first-run and failure-exit verification.
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
- Normal Windows receipt layout, custom installation root, damaged receipt, and Cargo's `bin` fallback.
- `AGLINK_DATA_DIR` override.
- Portable mode by placing `agent-linker.db` next to the executable and leaving `AGLINK_DATA_DIR` unset.

The database unit tests cover platform defaults, receipt precedence, overrides, portable and Cargo layouts, missing/old/new schemas, read-only access, and WAL snapshots. Still smoke-test real CLI output on any platform included in release notes. Portable staging and normal install/upgrade must explicitly migrate to schema v3 before the first query. Nonhealthy db check must stop installation success. Unix installers must initialize successfully and propagate initialization failure; source/manual quick starts must explicitly run `db migrate`. MSIX package identity has not been verified by these entry-point checks.

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
