# Library Curation: Registry, Groups & Lifecycle

## Commands

```text
aglink skill add <path> [--name <name>] [--alias <link-name>]
aglink resource add <path> --target-dir <dir> [--name <name>] [--alias <link-name>]
aglink <skill|resource> list
aglink <skill|resource> show <item>
aglink <skill|resource> rename <item> <new-name>
aglink <skill|resource> refresh <item>
aglink <skill|resource> remove <item>

aglink group create <name>
aglink group list
aglink group show <group>
aglink group rename <group> <new-name>
aglink group delete <group>
aglink group add <group> <item>...
aglink group remove <group> <item>...
```

*(Note: `group link` and `group unlink` operate on a project workspace; see [project-workspace.md](project-workspace.md).)*

## 1. Register & Inspect Items (`add`, `list`, `show`)

- `add` resolves `<path>` relative to CWD, stores the canonicalized absolute path in `agent-linker.db` (assigning `itm_<16hex>`), and never creates project links or overwrites existing registry records.
- `--alias` sets the default link name only; it cannot be used to look up `<item>`.

| Rule | `skill` | `resource` |
| --- | --- | --- |
| **Source Requirement** | Directory containing a readable, non-blank UTF-8 `SKILL.md` | Any file or directory |
| **Required Add Flag** | None | `--target-dir <dir>` (project-relative, no `..`) |
| **Default Name** (`--name` omitted) | Final path segment without its last extension | Same as `skill` (e.g., `guide.md` -> `guide`) |
| **Default Link Name** | `--alias` if set; else registered name | `--alias` if set; else full final segment of canonical source path (e.g., `guide.md`) |
| **Uniqueness** | Registered name is globally case-insensitive unique across Skill and Resource. Default link name is case-insensitive unique across Skills. | Registered name shares global case-insensitive uniqueness with Skills. Default destination is unique by normalized `target-dir` + link name (same filename allowed in different target directories). |

## 2. Manage Groups (`group`)

- **`create`**: Creates an empty group (`grp_<16hex>`); fails if the group name already exists.
- **`list` / `show`**: `list` aggregates group names and member counts; `show` loads full members.
- **`add` / `remove`**: Updates members in one SQLite transaction, deduplicating inputs by `item_id`. `add` skips already-present members with a message. Unknown items or `remove` of non-members fail and roll back the entire command.
- **`rename` / `delete`**: Renames or deletes the group; `delete` never unregisters member items. Modifying or deleting groups never alters existing project links.

## 3. Rename, Refresh, Relocate & Retire (`rename`, `refresh`, `remove`)

- **`rename <item> <new-name>`**: Updates the registered name while keeping its `item_id`, source path, `--alias`, groups, and existing project links. *Side effect*: Renaming an unaliased Skill changes its default link name for future `link` calls.
- **`refresh <item>`**: Re-validates the original source path and updates its canonical path and timestamp. Fails if the source is missing or changed file/directory type (**does not track moved files**).
- **`remove <item>`**: Unregisters the item from `agent-linker.db` and **removes it from all groups** (existing project symlinks and `.agents/links.toml` entries are unchanged).

### Changing Source Path, `--alias`, or Default `--target-dir`
There is no in-place `update` command for these attributes. Execute in order:
1. Check current group memberships (`aglink <type> show <item>` or `aglink group list` / `show`) so you can restore them.
2. In affected projects, run `aglink unlink <selector>` first.
3. Run `aglink <type> remove <item>`.
4. Run `aglink <type> add <new-path> ...` with the updated flags (assigns a new `itm_<16hex>` ID).
5. Re-add the item to its groups (`aglink group add <group> <item>`) and re-link in projects (`aglink link <item>`).
