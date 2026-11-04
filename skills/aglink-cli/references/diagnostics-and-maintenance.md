# Diagnostics, Link Repair & Database Maintenance

## Commands

```text
aglink status [--json]
aglink clean [--broken | --missing-source] [--dry-run]
aglink doctor [--verbose]
aglink db path
aglink db check
aglink db migrate
aglink db backup [file-path]
```

## 1. Quick Triage Order

1. **Project symlink missing, broken, or conflicting** -> Run `aglink status [--json]` (note: `status` exits `0` even when anomalies exist).
2. **Symlink creation failure, store/schema error, or Windows backend issue** -> Run `aglink doctor --verbose`, `aglink db path`, and `aglink db check`.

## 2. Project Link Status & Repair (`status`, `clean`, `link --force`)

| `links[].status` | Meaning | Safe Action |
| --- | --- | --- |
| `correct_symlink` | Points to recorded source; source exists with expected type | None needed. |
| `missing` | No file or symlink at link path | `aglink link <item>` to restore symlink, or `aglink clean` / `aglink unlink <selector>` to drop record. |
| `broken_symlink` | Points to recorded source, but source no longer exists | `aglink clean --broken` (or `clean`) to remove broken symlink + record; if source moved on disk, re-register via [library-curation.md](library-curation.md) and re-link. |
| `wrong_symlink_target` | Symlink points to a different target (existing or not) | `aglink link <item> --force` to repoint (fails if path is owned by another manifest entry), or `aglink unlink <selector>` to remove; if recorded source is also gone, `aglink clean --missing-source` removes it. |
| `existing_real_file` / `existing_real_directory` | Real file or directory occupies link path | **`--force`, `unlink`, and `clean` all abort.** Do not delete real user content automatically; ask the user to move/rename it or link with `--as` / `--target-dir`. |
| `unsupported_file_type` | Special FS node at link path, or source type no longer matches manifest | **`unlink` and `clean` abort** if link target is unsupported; inspect filesystem manually and check source via `skill refresh` / `resource refresh`. |

### `clean` Filter Semantics (Mutually Exclusive)
- `aglink clean` (default): Removes `missing` and `broken_symlink` entries.
- `aglink clean --broken`: Removes only `broken_symlink` entries.
- `aglink clean --missing-source`: Removes entries whose recorded source no longer exists, even if the symlink currently points elsewhere.

## 3. Database Lifecycle & Path Resolution (`db`)

- **Explicit migration required**: Ordinary reads and writes never create, migrate, or seed `agent-linker.db`. Source/manual installs and older schemas require `aglink db migrate` before querying; newer schemas require upgrading `aglink`.
- **`db path`** (resolves path and selection reason without opening DB; filename is always `agent-linker.db`):
  1. Non-empty `AGLINK_DATA_DIR` (relative paths resolved against CWD).
  2. Windows `bin/aglink.exe` with a valid installation receipt in `<installRoot>`: `<installRoot>/data/` (receipt read/validation errors fail explicitly; a missing DB file does not fall back; independent of `LOCALAPPDATA`).
  3. An **existing** `agent-linker.db` next to the `aglink` executable (portable mode).
  4. Platform default: Windows `%LOCALAPPDATA%/agent-linker/`, macOS `~/Library/Application Support/agent-linker/`, Linux `$XDG_DATA_HOME/agent-linker/` (or `~/.local/share/agent-linker/`).
  *(Unify `AGLINK_DATA_DIR` explicitly across entry points when using custom layouts.)*
- **`db check`**: Verifies file existence, schema v3, all five core tables, SQLite `quick_check`, and writability without `CREATE` (empty harness tables can be healthy). Exits `0` on `healthy_writable`; exits `1` reporting `missing`, `schema_mismatch`, `read_only`, `locked_busy`, or `invalid`. Never initializes, migrates, or seeds.
- **`db migrate`**: Initializes or upgrades to schema v3 in a single transaction (requires host execution outside sandbox), preserving existing IDs, relations, user state, and timestamps (`INSERT OR IGNORE` for built-in seed).
- **`db backup [file-path]`**: Opens existing DB read-only and writes a consistent SQLite snapshot including committed WAL data without changing the source schema. Defaults to `agent-linker-backup-<unix-nanos>.db` in `<installRoot>/backups/` (normal Windows installs) or in the DB directory (other modes). An explicit path creates missing parent directories and **fails if the destination already exists**. Run before schema migrations or bulk destructive registry edits.

## 4. Full-Stack System Check (`doctor`)

- `aglink doctor [--verbose]` verifies the global database, schema, built-in harness records, current project manifest, and symlink backend—probing `WinSymlinksBroker` on Windows via a temporary symlink.
- Exits `1` if any check fails; never modifies or repairs project links.
