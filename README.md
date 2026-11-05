<div align="center">

# Agent Linker (`aglink`)

**Manage AI Agent instructions, skills, and resources across projects with safe, controlled symbolic links.**

[![Release](https://img.shields.io/github/v/release/winddxr/agent-linker?color=blue)](https://github.com/winddxr/agent-linker/releases)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green.svg)](LICENSE-MIT)
[![Platform](https://img.shields.io/badge/platform-windows%20%7C%20linux%20%7C%20macos-lightgrey.svg)]()

[English](README.md) | [简体中文](README.zh-CN.md)

</div>

---

### Why Agent Linker?

Modern AI coding agents (Claude Code, Cursor, Antigravity, Windsurf) rely heavily on tailored instructions (`AGENTS.md`), specialized workflows (`SKILL.md`), and shared reference documentation. As you work across dozens of repositories, copying and pasting these assets creates fragmented knowledge, version drift, and maintenance headaches.

**Agent Linker** (`aglink`) treats AI skills and resources like system packages—centrally registered, safely mounted via OS symlinks, and declaratively tracked per project:

- ⚡ **Zero Duplication, Instant Updates**: Register skills and resources once in a central registry. Link them into any workspace without disk bloat—updates to source skills apply across all linked projects immediately.
- 🛡️ **Ironclad File Safety**: Strict safety boundaries. `aglink` never overwrites, moves, or deletes real files or directories. Even with `--force`, it only replaces symlinks pointing to wrong targets.
- 🪟 **Frictionless Windows Experience**: No developer mode hacks or repetitive UAC administrator prompts. `aglink` integrates with a dedicated background broker (`WinSymlinksBroker`) for seamless, unprivileged symlink creation on Windows.
- 🔌 **Universal Harness Compatibility**: Standardize on canonical workspace files (`AGENTS.md`, `.agents/skills/`) while automatically projecting compatibility symlinks for host agents (e.g. Claude Code's `.claude/`).
- 📋 **Auditable & Declarative**: Every linked item is recorded in a lightweight project manifest (`.agents/links.toml`), with runtime health verified via `aglink doctor`.

---

## 30-Second Mental Model

```text
[ Global SQLite Registry ]
  ├── Skill: aglink-cli  ──┐
  └── Resource: rules.md ──┼──> aglink link ──> [ Workspace: .agents/links.toml ]
                           │                        ├── .agents/skills/aglink-cli -> ...
                           │                        ├── AGENTS.md (canonical source)
                           │                        └── .claude/config.json -> ... (Harness link)
```

---

## 🚀 Quick Start

### 1. Installation

**Linux / macOS:**
```bash
curl -fsSL https://github.com/winddxr/agent-linker/releases/latest/download/aglink-installer.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://github.com/winddxr/agent-linker/releases/latest/download/aglink-installer.ps1 | iex
```

*Or install from source via Cargo:*
```bash
cargo install --path .
aglink db migrate
```

### 2. Common Workflow

```bash
# 1. Initialize project structure (.agents/, AGENTS.md, .gitignore)
aglink init

# 2. Register reusable skills & resources globally
aglink skill add ~/my-skills/code-reviewer --name reviewer
aglink resource add ~/docs/guidelines.md --target-dir docs

# 3. Mount them into the current project
aglink link reviewer
aglink link guidelines

# 4. Inspect status & verify health
aglink status
aglink doctor
```

---

## 📚 Documentation & Reference

Agent Linker maintains comprehensive design and operational documentation. Direct links to relevant resources:

| Topic | Documentation | Highlights |
| :--- | :--- | :--- |
| **CLI Usage** | **[CLI Reference](CLI_REFERENCE.md)** ([中文版](CLI_REFERENCE_zh.md)) | Full command surface (`init`, `link`, `unlink`, `skill`, `resource`, `group`, `harness`, `db`, `doctor`), resolution rules, and flags. |
| **Architecture** | **[Architecture Navigation](dev-docs/architecture.md)** | Design boundaries, module responsibilities, dependency rules, and single source of truth. |
| **Symlink Engine** | **[Symlink Core](dev-docs/modules/01-symlink-core.md)** | Cross-platform abstraction, Windows Broker IPC, conflict handling, and error models. |
| **Project Manifest** | **[Init & Manifest](dev-docs/modules/02-project-init-manifest.md)** | `.agents/links.toml` format, idempotent scaffolding, and `.gitignore` managed block. |
| **Harness Adapters** | **[Harness Adapter](dev-docs/modules/03-harness-adapter.md)** | Mapping canonical `.agents/` structures to Claude Code and future harnesses. |
| **Global Storage** | **[Global Store & Registry](dev-docs/modules/04-global-store-registry.md)** | SQLite Schema v4, transaction boundaries, migrations, and database resolution order. |
| **Linkable Items** | **[Linkable Items](dev-docs/modules/05-linkable-item.md)** | Skills, Resources, source validation (`SKILL.md`), and default target paths. |
| **Command Design** | **[CLI Command Surface](dev-docs/modules/06-cli-command-surface.md)** | User-facing behaviors, output modes (`--json`, `--quiet`), and exit codes. |
| **Code Structure** | **[Architecture & Verification](dev-docs/modules/07-code-architecture-verification.md)** | Internal crate boundaries, invariants, and testing strategy. |
| **Releases** | **[Release Checklist](RELEASE_CHECKLIST.md)** & **[Changelog](CHANGELOG.md)** | Verification steps, distribution packaging, and version history. |

---

## 🛠️ Contributing

Contributions are welcome! Please make sure changes conform to the architectural guidelines in [Architecture Navigation](dev-docs/architecture.md).

Run test suites before submitting PRs:
```bash
cargo test --locked
```

---

## 📄 License

Dual-licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
