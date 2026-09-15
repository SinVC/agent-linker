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

Normal installs place the executable under the user's application install
directory and keep data in the platform data directory:

- Windows: `%LOCALAPPDATA%\agent-linker\agent-linker.db`
- Linux: `$XDG_DATA_HOME/agent-linker/agent-linker.db`
- Linux fallback: `~/.local/share/agent-linker/agent-linker.db`
- macOS: `~/Library/Application Support/agent-linker/agent-linker.db`

On Windows, the installer may elevate only the broker service installation
step. The whole installer should not require administrator rights.

### Portable Install

Portable Windows archives include `agent-linker.db` next to `aglink.exe`.
This enables the existing portable database path resolution.

Portable mode controls the Agent Linker database location only. Windows
symlink operations still require the machine-level `WinSymlinksBroker` service.

## Uninstall Modes

Uninstall scripts support two levels:

- Default uninstall removes installed binaries, installer-owned files, PATH
  entries, and install receipt data. User databases and registry data are kept.
- Purge uninstall additionally removes Agent Linker data directories.

Broker removal is separate because the broker is a machine-level service and
may be shared by other tools:

- Default uninstall does not remove `WinSymlinksBroker`.
- `-RemoveBroker` / `--remove-broker` removes it with elevation.

When `AGLINK_DATA_DIR` is set, purge must delete it only when an ownership
marker and install receipt prove the directory is managed by this installer.
Otherwise it may remove only known Agent Linker files or leave the directory
for manual review.

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
