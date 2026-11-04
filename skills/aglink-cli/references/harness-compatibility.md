# Harness Compatibility & Project Scaffolding

Manages compatibility symlink path mappings in `agent-linker.db` for host AI tools (such as Claude Code) without launching or stopping the host tool (replaces the removed `framework` command).

## Commands

```text
aglink harness list
aglink harness show <harness>
aglink harness enable <harness>
aglink harness disable <harness>
aglink harness mapping list [harness]
aglink harness mapping add <harness> <source> <link> --kind <file|dir>
aglink harness mapping remove <harness> <link>
```

## 1. Global Harness Configuration

- **`<harness>`**: Accepts an existing harness name or ID. Only `claude` is built-in; custom harnesses cannot be created.
- **`enable` / `disable`**: Toggles the global enabled flag controlling whether `aglink init` creates compatibility symlinks for that harness.
- **`mapping list [harness]`**: Lists mappings for `<harness>`, or across all harnesses when `[harness]` is omitted.
- **`mapping add <harness> <source> <link> --kind <file|dir>`**:
  - `<source>` and `<link>` must be project-relative paths (absolute paths and `..` segments are rejected).
  - `--kind` accepts `file`, `dir`, or `directory`.
  - Does **not** verify whether `<source>` exists when adding the rule.
  - Duplicate `<link>` paths within the same harness are rejected.
- **`mapping remove <harness> <link>`**: Removes a mapping by `<link>` path. Built-in `required` mappings cannot be removed.

## 2. How Settings Apply to Projects

- **New `aglink init` runs**: `init` reads enabled harness mappings from `agent-linker.db` (read-only), creates the symlinks, records them in `.agents/links.toml`, and updates the `.gitignore` managed block (`.claude/`, `.agents/`).
  - No enabled harness -> no compatibility symlinks are created.
  - Missing database file -> `init` falls back to built-in Claude defaults without creating the database; outdated/newer schema or other store errors still fail explicitly.
  - `init` skips existing valid symlinks and fails on mismatched source types or target conflicts.
- **Existing projects (never auto-updated)**: Changing global harness settings never modifies existing repositories.
  - **Apply newly added/enabled mappings to an existing project**: Run `aglink init` in that project root (idempotent; scaffolds missing compatibility links while preserving existing files and valid symlinks).
  - **Remove existing harness links from a project**: `aglink unlink --all` and `aglink unlink --group` retain harness links. Run `aglink status` to locate the harness entry and remove it explicitly via `aglink unlink <link-path-or-manifest-id>`.
