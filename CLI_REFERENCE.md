# Agent Linker CLI Reference

`aglink` manages global state (the item registry, groups, and harness definitions) in a global SQLite database, and tracks project-scoped managed symlinks in `.agents/links.toml` (the *project manifest*).

## General Conventions

- **Scope and Decoupling**:
  - **`init` uses the current working directory; project commands search upward for the nearest `.agents/links.toml`**, stopping at a filesystem root or symlink directory. `init`, `link`, `unlink`, `status`, and `clean` operate on the current project; `skill`, `resource`, `group`, `db`, and `harness` operate on the global database; `doctor` inspects both.
  - **Global and project states are decoupled**: mutating the global database (renaming, unregistering, modifying groups, or changing harness settings) never alters existing project symlinks or manifest records.
- **Paths and Naming**:
  - Registered source paths are resolved relative to the current working directory and stored as canonicalized absolute paths.
  - Project target directories and harness mapping paths must be relative to the project root (absolute paths and `..` segments are rejected).
  - Item names, aliases (`--alias`), group names, and link overrides (`--as`) must be non-empty single path segments without `/\<>:"|?*` or standalone `.` / `..`.
- **Manifest and Non-transactional Execution**: Project commands require an initialized project containing `.agents/links.toml`; run `aglink init` if none is found. A malformed manifest returns an error. `link` creates missing real parent directories and updates the manifest (failing if any parent segment is a symlink), without scaffolding the rest of the `init` layout. `--dry-run` previews operations without modifying project files or the manifest. Filesystem operations are not transactional: execution stops on the first error without rolling back prior changes.
- **File Safety**:
  - `aglink` never copies, moves, or deletes source files, and never overwrites or deletes real files or directories.
  - Re-linking an existing symlink that already points to the expected target succeeds and records it in the manifest. `--force` **only replaces symlinks pointing to the wrong target**—it never overwrites real files/directories or claims a manifest path owned by another entry.
  - `unlink` and `clean` only remove symlinks and manifest records (or just the record if the path is already absent). If any selected target is a real file, real directory, or unsupported file type, the operation aborts with an error.
- **Windows Backend**: Symlink creation on Windows requires a running `WinSymlinksBroker` service and fails immediately without fallback if unavailable (diagnose with `aglink doctor --verbose`).
- **Output and Exit Codes**: `-q`/`--quiet` suppresses normal output and `--verbose` adds diagnostic detail (accepted before or after subcommands; `--quiet` wins if both are set; `status --json` is unaffected). Errors go to `stderr`. Commands exit with `0` on success or `--help`, and `1` on invalid arguments, runtime errors, or failed `doctor` / `db check` checks (`status` anomalies exit with `0`).

## Item Resolution and Default Paths

`<item>` accepts a registered name or Registry ID. `<group>` accepts a group name or ID.

- **Uniqueness**: Registered names are globally case-insensitive unique across Skill and Resource. Skill default link names are case-insensitive unique; Resource default destinations are unique by normalized target directory and link name, allowing the same filename in different directories. Aliases determine default link names and do not participate in ID/name lookups.

| Rule | Skill | Resource |
| --- | --- | --- |
| **Source Requirement** | Directory containing a readable, non-blank UTF-8 `SKILL.md` | Any file or directory |
| **Default Name** (`--name` omitted) | Final path segment without its last extension | Same as Skill (e.g., `guide.md` → `guide`) |
| **Default Link Name** | `--alias` if set; otherwise the registered name | `--alias` if set; otherwise the full final segment of the canonical source path (e.g., `guide.md`) |
| **Default Project Link Path** | `.agents/skills/<link-name>` | `<target-dir>/<link-name>` |

`link --as <link-name>` overrides the default link name verbatim (without auto-appending an extension), and Resource `link --target-dir <dir>` overrides the target directory for that operation only. A single item may be linked to multiple paths in the same project.

## Initialization, Status, and Cleanup

```text
aglink init
aglink status [--json]
aglink clean [--broken | --missing-source] [--dry-run]
aglink doctor
```

- **`init`**: Idempotently scaffolds the current project (no `--dry-run` or `--force`). Creates missing `AGENTS.md` (empty file), `.agents/`, and `.agents/skills/` (preserving existing content); reads enabled harness mappings from the initialized global database and records created links in the manifest (when the database is missing, falls back to the same built-in Claude mappings without initializing it) (skipping existing valid symlinks, failing on mismatched source types or target conflicts); updates the `aglink` managed block in `.gitignore` (`.claude/`, `.agents/`).
- **`status [--json]`**: Inspects only manifest entries without scanning for untracked symlinks or repairing links. `--json` outputs `project_root`, `manifest_path`, and `links` (each containing manifest `id`, `item_id`, `item_name`, `link_path`, `source_path`, `link_kind` [`file`/`directory`], `provider_backend`, and `status`).

| `links[].status` | Description |
| --- | --- |
| `missing` | No file or symlink exists at the link path |
| `correct_symlink` | Symlink points to the recorded source path, and the source exists with the expected type |
| `wrong_symlink_target` | Symlink points to a different target (whether or not that target exists) |
| `broken_symlink` | Symlink points to the recorded source path, but the source no longer exists |
| `existing_real_file` / `existing_real_directory` | A real file or real directory occupies the link path |
| `unsupported_file_type` | An unsupported file type occupies the link path, or the source type no longer matches the manifest |

- **`clean`**: Removes stale manifest entries using mutually exclusive filters: defaults to `missing` and `broken_symlink`; `--broken` selects only `broken_symlink`; `--missing-source` selects entries whose recorded source no longer exists (even if the symlink points elsewhere).
- **`doctor`**: Verifies the global database, schema, built-in harness records, project manifest, and symlink backend (probing `WinSymlinksBroker` on Windows via a temporary symlink). Does not repair project links.

## Linking and Unlinking

```text
aglink link <item> [--as <link-name>] [--target-dir <dir>] [--force] [--dry-run]
aglink link --group <group> [--force] [--dry-run]
aglink unlink <selector> [--dry-run]
aglink unlink --group <group> [--dry-run]
aglink unlink --all [--dry-run]
```

- **`link`**: Validates source existence, type, and `SKILL.md` content. `--target-dir` applies only to Resources. `--group` links each member sequentially using its default path (`--as` and `--target-dir` are not supported).
- **`unlink`**: Requires exactly one of `<selector>`, `--group`, or `--all`. Pre-validates all selected targets before making changes; if any target is a real file/directory or unsupported type, the entire command aborts without partial deletions.
  - `<selector>` **queries only the project manifest (without accessing the database)**: accepts a manifest ID, `item_id`, recorded `item_name`, project-relative `link_path`, or link basename. Must match exactly one entry; disambiguate multiple matches with the manifest ID or full relative path.
  - `--group <group>`: Selects item links matching the group's **current member IDs** (items removed from the group after linking are not selected); harness links are retained.
  - `--all`: Selects all item links while retaining harness links (without accessing the database).

## Skill and Resource Registry

```text
aglink skill add <path> [--name <name>] [--alias <link-name>]
aglink resource add <path> --target-dir <dir> [--name <name>] [--alias <link-name>]
aglink <type> list
aglink <type> show <item>
aglink <type> rename <item> <new-name>
aglink <type> remove <item>
aglink <type> refresh <item>
```

Where `<type>` is `skill` or `resource`:

- **`add`**: Registers a local source in the global database without creating project links. Fails on a globally conflicting registered name or default destination (using the uniqueness rules above; does not overwrite).
- **`list` / `show`**: Lists all items of that type or shows details for a single item.
- **`rename`**: Updates the registered name while keeping its ID, alias, and groups (renaming an unaliased Skill changes its default link name for future `link` calls).
- **`remove`**: Unregisters the item and removes it from all groups.
- **`refresh`**: Re-validates the original source path and updates its canonical path and timestamp; fails if the source is missing or changed type (does not track moved files).
- Changing a source path, alias, or default target directory requires `remove` followed by `add` (which assigns a new ID; `unlink` existing project links first).

## Groups

```text
aglink group create <name>
aglink group list
aglink group show <group>
aglink group rename <group> <new-name>
aglink group delete <group>
aglink group add <group> <item>...
aglink group remove <group> <item>...
aglink group link <group> [--force] [--dry-run]
aglink group unlink <group> [--dry-run]
```

- **`create`**: Creates an empty group (fails if the name exists). **`list`**: Aggregates group names and member counts; **`show`** loads full members. **`rename` / `delete`**: Renames or deletes a group (deleting a group does not unregister its members).
- **`add` / `remove`**: Adds or removes one or more members in one transaction. Inputs deduplicate by item_id; add skips existing members with a message. Unknown items or remove of non-members fail and roll back the command.
- **`group link` / `group unlink`**: Aliases for `link --group` and `unlink --group`.

## Database

```text
aglink db path
aglink db migrate
aglink db backup [file-path]
aglink db check
```

- **`db path`**: Prints the resolved database path and selection reason without opening the database. Queries use read-only connections without creation, migration, or seeding. Missing/old databases require `aglink db migrate`; newer schemas require upgrading aglink.
- **`db migrate`**: Explicitly initializes or upgrades to schema v3 in one transaction, preserving existing IDs and relations. Built-in seed uses INSERT OR IGNORE and preserves user state and timestamps. Ordinary reads and writes never initialize or migrate.
- **`db backup [file-path]`**: Opens an existing source read-only and writes a consistent SQLite snapshot, including committed WAL data, without changing the source schema. Defaults to `agent-linker-backup-<unix-nanos>.db` in `<installRoot>/backups/` for normal Windows installs, or in the database directory for other modes. An explicit relative or absolute path creates missing parent directories and fails if the destination exists.
- **`db check`**: Checks existence, schema version, all five core tables, and SQLite quick_check, then probes writability without CREATE. Empty harness tables can be healthy. Reports missing/schema_mismatch/read_only/locked_busy/invalid with nonzero exit status; healthy_writable exits 0. Does not initialize, migrate, or seed.

**Database Path Resolution Order** (filename is always `agent-linker.db`):
1. Directory in non-empty `AGLINK_DATA_DIR` (relative paths resolved against the working directory).
2. Windows `bin/aglink.exe` with a valid normal Agent Linker installation receipt in its installation root: `<installRoot>/data/`. Receipt read/validation errors fail explicitly; a missing database does not change the target.
3. An **existing** `agent-linker.db` next to the `aglink` executable (portable mode).
4. Platform default directory: Windows `%LOCALAPPDATA%/agent-linker/`; macOS `~/Library/Application Support/agent-linker/`; Linux `$XDG_DATA_HOME/agent-linker/` (or `~/.local/share/agent-linker/`).

Source/manual installs must run `aglink db migrate` before their first query. Use an ordinary user account daily. Agent persistent writes run outside the sandbox with host authorization; initialized-database queries and link dry-runs can run inside it. Permission failures are explicit and never switch databases or disable the sandbox. Normal Windows installations resolve independently of `LOCALAPPDATA`; explicit `AGLINK_DATA_DIR` overrides intentionally select another configuration. Other layouts should explicitly unify that directory across entry points.

## Harness Compatibility

Manages compatibility path mappings for host AI agent tools (such as Claude Code) without launching or stopping the host tool (replaces the removed `framework` command).

```text
aglink harness list
aglink harness show <harness>
aglink harness enable <harness>
aglink harness disable <harness>
aglink harness mapping list [harness]
aglink harness mapping add <harness> <source> <link> --kind <file|dir>
aglink harness mapping remove <harness> <link>
```

- `<harness>` accepts an existing name or ID (only `claude` is built in; custom harnesses cannot be added). `enable`/`disable` toggles the global enabled flag.
- **`mapping`**: `<source>` and `<link>` are project-relative paths (`add` does not check whether `<source>` exists; `--kind` accepts `file`, `dir`, or `directory`). Omitting `[harness]` in `mapping list` lists all harnesses. Duplicate `<link>` paths per harness are rejected, and `required` built-in mappings cannot be removed.
- Settings apply to new `init` calls, which read enabled harness mappings without writing the database. No enabled harness means no compatibility links. A missing database uses the built-in Claude defaults without creating it; old/new schema and other store errors are reported. Existing projects are not changed by setting updates.

## End-to-End Example

Run from the project root (assuming `../skills/writer/SKILL.md` is non-empty and `../guide.md` exists):

```bash
aglink db migrate
aglink init
aglink skill add ../skills/writer --name writer --alias writing # Default link name: writing
aglink link writer --as draft                                   # Override: creates .agents/skills/draft -> ../skills/writer
aglink unlink .agents/skills/draft                              # Removes symlink & manifest record; keeps source & registry item

aglink resource add ../guide.md --target-dir docs               # Inferred name: guide; default path: docs/guide.md
aglink link guide --target-dir references --as handbook.md      # Override: creates references/handbook.md -> ../guide.md
aglink unlink references/handbook.md
```

SQLite writes commit once per command. group add/remove deduplicate by item_id; add skips existing members with a message. group list aggregates names and member counts; show loads full members. New Item/Group IDs use itm_<16hex>/grp_<16hex>; rename preserves IDs, sources, and existing projects. Verbose timestamps use readable UTC.
