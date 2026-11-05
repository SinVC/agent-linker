# Release Distribution Plan

This document records the release and installation contract for Agent Linker.

## Goals

- GitHub Release is the versioned artifact source of truth.
- Release builds are produced by GitHub Actions, not by local manual upload.
- Windows releases include the required `WinSymlinksBroker` runtime component.
- Installer scripts are maintained in the repository and published as release assets.
- Normal installs keep user data outside the executable directory.
- Portable installs keep `agent-linker.db` next to `aglink.exe`.

## Release Assets

Each tagged release should publish:

- `aglink-<version>-x86_64-pc-windows-msvc.zip`
- `aglink-<version>-x86_64-pc-windows-msvc-portable.zip`
- `aglink-<version>-x86_64-unknown-linux-gnu.tar.gz`
- `aglink-installer.ps1`
- `aglink-uninstall.ps1`
- `aglink-installer.sh`
- `aglink-uninstall.sh`
- `SHA256SUMS`

Windows archive contents must not include `LICENSES/` or
`THIRD_PARTY_NOTICES.md`. Version metadata belongs in `install-receipt.json`;
do not add a separate `VERSION.json`.

Windows and Linux archives include the complete repository `skills/` tree.
Normal installers copy it to the resolved data directory's `skills/`, then register
`aglink-cli` and add it to `init defaults` through CLI commands. Rust initialization
and migrations contain no bundled-skill name or source-path defaults.
The data marker `.aglink-cli-configured` is written only after successful first
configuration. Upgrades refresh the same registered source without changing its ID
or restoring a user-removed default. A same-name registration from another source
fails explicitly and is preserved. Default uninstall retains skills and the marker;
purge removes the marker with the database, retaining skill files and unknown content.

## Windows Broker Runtime

`WinSymlinksBroker` is a required Windows runtime dependency. Windows packages
must include a fixed, checksum-verified broker build under `broker/`.

The release workflow takes the broker from a pinned release asset or archive
configured by repository variables:

- `WIN_SYMLINKS_BROKER_URL`
- `WIN_SYMLINKS_BROKER_SHA256`

The release should record the broker source URL and SHA256 in
`install-receipt.json`.

Do not fetch a floating upstream `latest` broker during user installation.
Do not ask users to build the broker from source as part of normal install.

## Install Modes

### Normal Install

Normal Windows installs use a single-user installation root (default
`ProgramW6432/aglink`, falling back to `ProgramFiles/aglink`; `-InstallDir` overrides it).
`bin/` contains the CLI, `data/` contains SQLite and its sidecars, and `backups/`
contains default backups. Existing `broker/`, `scripts/`, and the receipt remain.
Other platforms keep their data paths:

- Windows normal install: `<installRoot>\data\agent-linker.db`; unrecognized layouts retain the `%LOCALAPPDATA%\agent-linker` fallback.
- Linux: `$XDG_DATA_HOME/agent-linker/agent-linker.db`
- Linux fallback: `~/.local/share/agent-linker/agent-linker.db`
- macOS: `~/Library/Application Support/agent-linker/agent-linker.db`

The ordinary Windows outer process captures the initiating SID and old default
database, downloads and verifies the archive, and launches a disk-backed helper.
Only protected installation updates, receipt, ACLs, and necessary Broker setup
are elevated. Writable custom roots with an available Broker (or `-NoBroker`)
need no UAC. Grant that SID inherited Modify on `data/` and `backups/` only.
The outer process imports a consistent snapshot of an existing old default database
only when the target is absent, preserves the source, then runs `db migrate` and
`db check` with checked exit codes. It temporarily sets `AGLINK_DATA_DIR` to the
explicit source/target and restores it in `finally`; user overrides are not changed.
Existing targets are validated and never overwritten. `doctor` is diagnostic only.
For local installation verification, `-PackagePath <zip> -Version <exact-version>`
uses that archive and its adjacent `.sha256` instead of downloading release assets.
`-SkipDatabaseSnapshot` skips only the temporary validation snapshot of an existing
target database; explicit migration and post-migration `db check` still run. Legacy
database import, when needed, remains a consistent snapshot into the target.
An already-elevated outer process is not evidence of ordinary-user ACL correctness.
Reuse an installed Broker without replacing files that its service may be using.

### Portable Install

Portable Windows archives include an initialized `agent-linker.db` next to `aglink.exe`.
This enables the existing portable database path resolution.
Packaging explicitly sets the staging data directory for `db migrate`, producing a schema v4 database usable by the first ordinary query. Normal install/upgrade runs explicit migrate and check; a nonhealthy check returns nonzero and stops installation success. Unix
installers initialize and check the database; source/manual installs must run
`aglink db migrate` before queries.

Portable mode controls the Agent Linker database location only. Windows
symlink operations still require the machine-level `WinSymlinksBroker` service.

After extracting a portable package to its final location, run
`powershell -ExecutionPolicy Bypass -File scripts/setup-portable.ps1` to register
the bundled skills and configure init defaults. Packaging leaves the Registry empty
so no build-machine absolute paths enter the archive. Registration uses absolute
paths; moving the extracted directory later requires unregistering the old source
and registering/configuring the new location explicitly.

## Uninstall Modes

Uninstall scripts support two levels:

- Default uninstall removes the CLI and user PATH entry, retaining data, backups,
  shared Broker files, receipt, and maintenance scripts for subsequent removal.
  Protected deletion uses the helper; never recursively delete the installation root.
- Purge additionally removes known SQLite files (including `-journal`, `-wal`,
  `-shm`) from this installation's `data/` and generated default backup files
  from `backups/`. Unknown content is retained.

Broker removal is separate because the broker is a machine-level service and
may be shared by other tools:

- Default uninstall does not remove `WinSymlinksBroker`.
- `-RemoveBroker` / `--remove-broker` removes it with elevation.

When `AGLINK_DATA_DIR` is set, purge removes only known SQLite files, never
recursively deletes that directory, and does not assume an ownership marker exists.

## Script Source And Distribution

Script source lives under `packaging/`. The user-facing one-line install
commands should download scripts from release assets:

```powershell
irm https://github.com/winddxr/agent-linker/releases/latest/download/aglink-installer.ps1 | iex
```

```bash
curl -fsSL https://github.com/winddxr/agent-linker/releases/latest/download/aglink-installer.sh | sh
```

Do not point user installation commands at the `main` branch.
