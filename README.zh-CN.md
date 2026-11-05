<div align="center">

# Agent Linker (`aglink`)

**基于安全受控的符号链接（Symlink），在多工程间统一管理 AI Agent 的指令、技能与上下文资源。**

[![Release](https://img.shields.io/github/v/release/SinVC/agent-linker?color=blue)](https://github.com/SinVC/agent-linker/releases)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green.svg)](LICENSE-MIT)
[![Platform](https://img.shields.io/badge/platform-windows%20%7C%20linux%20%7C%20macos-lightgrey.svg)]()

[English](README.md) | [简体中文](README.zh-CN.md)

</div>

---

### 为什么需要 Agent Linker？

现代 AI 编程助手（如 Claude Code、Cursor、Antigravity、Windsurf 等）高度依赖工程专用指令（`AGENTS.md`）、领域技能（`SKILL.md`）和共享规范文档。但在跨多个代码库协作时，传统的“复制粘贴”极易导致上下文漂移、版本碎片化与沉重的维护负担。

**Agent Linker** (`aglink`) 将 AI 技能与资源视作系统级包进行管理：集中注册、受控挂载、工程清单声明化追踪：

- ⚡ **零磁盘冗余，一处更新处处生效**：在全局中心注册技能与资源文件，通过操作系统符号链接瞬间挂载到任意工程；更新源文件，所有关联项目实时同步，无磁盘冗余。
- 🛡️ **坚如磐石的文件安全底线**：严格区分真实文件与符号链接。`aglink` 绝不修改、覆盖或删除任何真实文件与目录；即便启用 `--force`，也仅限于替换指向错误目标的符号链接。
- 🪟 **丝滑无感的 Windows 体验**：告别开发者模式配置烦恼与高频弹出的 UAC 管理员提权窗口。Windows 端配备专属后台服务（`WinSymlinksBroker`），免提权静默创建符号链接。
- 🔌 **全生态 Harness 适配兼容**：以开放标准规范（`AGENTS.md`、`.agents/skills/`）为真实源，自动向宿主 Agent（如 Claude Code 的 `.claude/`）投射兼容映射，避免工具格式锁定。
- 📋 **状态声明化与健康自愈诊断**：每个工程的挂载状态通过轻量清单（`.agents/links.toml`）严格纳管，操作幂等，并通过 `aglink doctor` 一键完成全局数据库与工程级自检。

---

## 30 秒架构全景

```text
[ 全局 SQLite 注册表 ]
  ├── Skill: aglink-cli  ──┐
  └── Resource: rules.md ──┼──> aglink link ──> [ 工作区工程: .agents/links.toml ]
                           │                        ├── .agents/skills/aglink-cli -> ...
                           │                        ├── AGENTS.md (标准真实源)
                           │                        └── .claude/config.json -> ... (Harness 兼容映射)
```

---

## 🚀 快速上手

### 1. 安装

**Linux / macOS:**
```bash
curl -fsSL https://github.com/SinVC/agent-linker/releases/latest/download/aglink-installer.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://github.com/SinVC/agent-linker/releases/latest/download/aglink-installer.ps1 | iex
```

*或使用 Cargo 从源码编译安装：*
```bash
cargo install --path .
aglink db migrate
```

### 2. 核心工作流

```bash
# 1. 初始化当前工程（生成 .agents/、AGENTS.md 并配置 .gitignore）
aglink init

# 2. 在全局注册可复用的 Skill 或通用 Resource
aglink skill add ~/my-skills/code-reviewer --name reviewer
aglink resource add ~/docs/guidelines.md --target-dir docs

# 3. 挂载到当前工程中
aglink link reviewer
aglink link guidelines

# 4. 查看当前链接状态与健康检查
aglink status
aglink doctor
```

---

## 📚 深度文档索引

项目已沉淀完备的架构与设计文档，详情请直接查阅对应指南：

| 主题 | 文档链接 | 核心内容 |
| :--- | :--- | :--- |
| **CLI 命令指南** | **[CLI 命令行手册](CLI_REFERENCE_zh.md)** ([English](CLI_REFERENCE.md)) | 完整命令字典（`init`、`link`、`unlink`、`skill`、`resource`、`group`、`harness`、`db`、`doctor`）、参数规则与退出码口径。 |
| **系统架构** | **[架构导航与总览](dev-docs/architecture.md)** | 核心设计原则、模块索引、依赖拓扑与产品边界定义（项目唯一事实来源）。 |
| **链接核心** | **[符号链接核心 (Symlink Core)](dev-docs/modules/01-symlink-core.md)** | 跨平台 Symlink 抽象机制、Windows Broker 架构设计、错误分类与冲突判定。 |
| **工程清单** | **[工程初始化与清单 (Init & Manifest)](dev-docs/modules/02-project-init-manifest.md)** | `.agents/links.toml` 清单规范、幂等初始化拓扑与 `.gitignore` 托管。 |
| **宿主适配** | **[宿主适配器 (Harness Adapter)](dev-docs/modules/03-harness-adapter.md)** | 标准规范到第三方 Agent 约定（如 Claude Code）的映射与兼容层契约。 |
| **全局存储** | **[全局存储与注册表 (Global Store & Registry)](dev-docs/modules/04-global-store-registry.md)** | SQLite Schema v4 结构、事务边界、数据库路径解析逻辑与迁移策略。 |
| **链接条目** | **[链接条目模型 (Linkable Items)](dev-docs/modules/05-linkable-item.md)** | Skill 与 Resource 的抽象模型、来源归属约束与默认目标路径解析。 |
| **命令界面** | **[CLI 命令界面 (Command Surface)](dev-docs/modules/06-cli-command-surface.md)** | 用户可见交互边界、输出模式规范（`--json`、`--quiet`）及诊断逻辑。 |
| **代码规范** | **[代码架构与验证策略](dev-docs/modules/07-code-architecture-verification.md)** | 内部模块依赖防腐规则、不变量守则与测试验证策略。 |
| **发布与变更** | **[发布检查清单](RELEASE_CHECKLIST.md)** 与 **[更新日志](CHANGELOG.md)** | 发版前的完备性检查清单与历史版本变更记录。 |

---

## 🛠️ 参与贡献

欢迎提交 Issue 与 Pull Request！提交代码前，请确保遵循 [架构导航](dev-docs/architecture.md) 中的设计规范，并运行自动化测试：

```bash
cargo test --locked
```

---

## 📄 开源许可

本项目采用双重开源许可证授权：
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))
