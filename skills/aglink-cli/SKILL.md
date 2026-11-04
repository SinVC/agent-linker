---
name: aglink-cli
description: Operate the aglink (Agent Linker) CLI to register skill directories and resource files in agent-linker.db, curate reusable groups, initialize .agents/links.toml, mount or unmount managed project symlinks, configure host harness mappings (e.g., Claude Code), and diagnose broken symlinks, database migrations/backups, or WinSymlinksBroker. Do NOT use when invoking an existing skill to perform a task, authoring SKILL.md prompt content, or running general git, package-manager, or application-database commands.
---

# `aglink` CLI Skill

`aglink` manages reusable Agent skills and resources via controlled symlinks across two strictly decoupled layers:
- **Global SQLite (`agent-linker.db`)**: `skill`, `resource`, `group`, `harness`, and `db` commands.
- **Project Manifest (`.agents/links.toml`)**: `init` (uses current working directory) and `link`, `unlink`, `status`, `clean` (search upward from CWD for the nearest `.agents/links.toml`, stopping at a filesystem root or symlink directory). `doctor` inspects both layers.

## Core Invariants

1. **Strict State Decoupling**: Mutating the global database (renaming, unregistering, editing groups, or changing harness settings) **never** alters existing project symlinks or `.agents/links.toml`. Raw paths cannot be passed to `aglink link`—register them in the global database first (`skill add` / `resource add`), then `link`.
2. **Real-File Safety**: `aglink` never copies, moves, or deletes source files, and never overwrites or deletes real files or real directories.
   - `--force` (on `link`) replaces **only** an existing symlink pointing to the wrong target; it fails on real files/directories or manifest paths owned by another entry.
   - `unlink` and `clean` abort with an error if any selected target is a real file, real directory, or unsupported file type.
3. **Atomicity & Previews**: SQLite writes commit once per command. Filesystem operations are **not** transactional and stop on the first error without rolling back prior changes—use `--dry-run` before batch or uncertain `link`, `unlink`, or `clean` operations.
4. **Sandbox, Permissions & Windows Backend**:
   - Initialized-database read queries and `--dry-run` previews can run inside a sandbox.
   - Persistent writes (`db migrate`, global registry/group/harness edits, and real `init`/`link`/`unlink`/`clean` runs) require host authorization outside the sandbox. Permission errors fail explicitly and never switch databases or disable the sandbox. Use an ordinary user account daily.
   - Windows symlink creation requires a running `WinSymlinksBroker` service and fails immediately without fallback if unavailable.
5. **Naming & Path Rules**:
   - Item names, `--alias`, group names, and `--as` must be non-empty single path segments without `/\<>:"|?*` or standalone `.` / `..`.
   - Project `--target-dir` and harness paths must be relative to the project root (absolute paths and `..` segments are rejected).
6. **Output & Exit Codes**:
   - `-q`/`--quiet` suppresses normal output; `--verbose` adds diagnostics with readable UTC timestamps (accepted before or after subcommands; `--quiet` wins if both are set; `status --json` is unaffected). Errors go to `stderr`.
   - Exits `0` on success or `--help`; exits `1` on invalid arguments, runtime errors, or failed `doctor` / `db check` checks (**note**: `status` exits `0` even when link anomalies are present).

## Scenario Routing

Load only the reference file matching the current task:

| Scenario | Reference |
| --- | --- |
| Initialize a repo (`init`), mount/unmount individual skills, resources, or groups (`link`, `unlink`, `group link/unlink`), override target paths, or list active project links (`status`) | [references/project-workspace.md](references/project-workspace.md) |
| Register, inspect, rename, refresh, relocate, or remove global `skill` / `resource` entries, or curate `group` definitions and memberships | [references/library-curation.md](references/library-curation.md) |
| Fix broken or conflicting project links (`status` anomalies, `clean`, `link --force`), locate/initialize/migrate/backup `agent-linker.db` (`db *`), or troubleshoot backend/store failures (`doctor`) | [references/diagnostics-and-maintenance.md](references/diagnostics-and-maintenance.md) |
| Configure host tool compatibility path mappings (`harness *`, e.g., Claude Code) or sync harness links in new/existing projects | [references/harness-compatibility.md](references/harness-compatibility.md) |

### Composite Workflows
- **Mount an unregistered local path into the current project**: Register via [references/library-curation.md](references/library-curation.md) (`skill/resource add`) -> mount via [references/project-workspace.md](references/project-workspace.md) (`init` if needed, then `link`).
- **Missing or outdated database error**: Initialize or upgrade first via `aglink db migrate` in [references/diagnostics-and-maintenance.md](references/diagnostics-and-maintenance.md) (ordinary commands never auto-create or auto-migrate the database).
