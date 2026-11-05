# Project Workspace: Init, Mount & Unmount

## Commands

```text
aglink init
aglink link <item> [--as <link-name>] [--target-dir <dir>] [--force] [--dry-run]
aglink link --group <group> [--force] [--dry-run]        # alias: aglink group link <group> ...
aglink unlink <selector> [--dry-run]
aglink unlink --group <group> [--dry-run]                # alias: aglink group unlink <group> ...
aglink unlink --all [--dry-run]
aglink status [--json]
```

## 1. Initialize Project (`init`)

- Initialization runs in order: real project scaffolding, enabled Harness compatibility links, then configured default Skill/Resource items. Each link phase preflights its batch; a later phase failure preserves completed earlier phases.
- Manage the global default list with `aglink init defaults list`, `aglink init defaults add <item>`, and `aglink init defaults remove <item>` (registered name or ID). Add/remove are idempotent for known items. Renames preserve membership; unregistering removes it. Changing defaults does not unlink existing projects; rerunning `init` adds current defaults and retains old links. Missing sources and target conflicts fail explicitly.

- Run `aglink init` in the project root before `link` if `.agents/links.toml` does not exist (`init` always targets CWD; no `--dry-run` or `--force`). A malformed manifest returns an error.
- Idempotently creates missing `AGENTS.md` (empty file), `.agents/`, and `.agents/skills/` while preserving existing content.
- Updates the `aglink` managed block in `.gitignore` (`.claude/`, `.agents/`).
- Reads enabled harness mappings from the initialized global database and records created symlinks in `.agents/links.toml` (if the database file is missing, falls back to built-in Claude mappings without creating the database; skips existing valid symlinks; fails on mismatched source types or target conflicts).

## 2. Mount Skills, Resources, or Groups (`link`)

- **Lookup keys**: `<item>` accepts a registered name (case-insensitive) or Registry ID (`itm_<16hex>`). `<group>` accepts a group name or ID (`grp_<16hex>`). Aliases (`--alias`) cannot be used as `<item>` lookup keys. If unknown, run `aglink skill list`, `aglink resource list`, or `aglink group list` / `show` first.
- **Validation & execution**: Validates source existence, type, and non-blank UTF-8 `SKILL.md` (for skills). Creates missing real parent directories (fails if any parent segment is a symlink) without scaffolding the rest of the `init` layout. Re-linking a symlink that already points to the expected target succeeds and records it in `.agents/links.toml`.

| Granularity | Default Project Link Path | Path Overrides |
| --- | --- | --- |
| **Single Skill** (`link <skill>`) | `.agents/skills/<link-name>` (`<link-name>` = `--alias` if set, else registered name) | `--as <link-name>` overrides `<link-name>` verbatim. `--target-dir` is **not** supported. |
| **Single Resource** (`link <resource>`) | `<target-dir>/<link-name>` (`<link-name>` = `--alias` if set, else full final segment of canonical source path, e.g., `guide.md`) | `--target-dir <dir>` overrides target directory for this operation only. `--as <link-name>` overrides filename verbatim (**without** auto-appending an extension). |
| **Group** (`link --group <group>` / `group link`) | Each member uses its own default path sequentially | `--as` and `--target-dir` are **not** supported. |

- A single item can be linked to multiple paths in the same project.
- `--force` replaces **only** an existing symlink pointing to the wrong target; it never overwrites real files/directories or claims a manifest path owned by another entry.

## 3. List Active Project Links (`status`)

- There is no `link list` subcommand—use `aglink status [--json]` to inspect tracked `.agents/links.toml` entries (`status` does not scan for untracked symlinks or repair links).
- `--json` outputs `project_root`, `manifest_path`, and `links[]` (`id`, `item_id`, `item_name`, `link_path`, `source_path`, `link_kind` [`file`/`directory`], `provider_backend`, `status`).
- Healthy entries report `correct_symlink`. If any other status appears, see [diagnostics-and-maintenance.md](diagnostics-and-maintenance.md).

## 4. Unmount Links (`unlink`)

- Pass **exactly one** of `<selector>`, `--group`, or `--all`.
- Pre-validates all selected targets before mutating: if **any** selected target is a real file, real directory, or unsupported file type, the entire command aborts with zero deletions. Otherwise removes symlinks and manifest records (or just the record if the path is already absent).
- **`<selector>`**: Queries **only `.agents/links.toml`** (never accesses `agent-linker.db`). Accepts manifest `id`, `item_id`, recorded `item_name`, project-relative `link_path`, or link basename. Must match **exactly one** entry—if an item is linked to multiple paths in the project, run `aglink status` and disambiguate with the manifest `id` or full relative `link_path`.
- **`--group <group>`** (`group unlink`): Selects project links matching the group's **current** database member IDs and retains harness links. *Caveat*: Items removed from the group after linking are not selected—remove them individually with `aglink unlink <selector>`.
- **`--all`**: Removes all item links while retaining harness links (without accessing `agent-linker.db`).
