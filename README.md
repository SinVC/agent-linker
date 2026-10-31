# Agent Linker (aglink)

**Agent Linker** (`aglink`) 是一个基于 Rust 开发的 CLI 工具，专为 AI Agent 的指令、技能 (Skills) 和资源 (Resources) 提供基于受控 Symlink (符号链接) 的管理方案。

通过构建中心化的 Agent 资源仓库，并将它们安全、高效地链接到各个独立的工程目录中，Agent Linker 帮助开发者轻松实现跨项目的上下文同步与 Harness 兼容。

## 🌟 Advantages (优势)

- **Safety First (安全第一)**: 严格区分真实文件与符号链接。程序永远不会覆盖或删除用户的真实文件与目录。即便在使用 `--force` 时，也仅限于替换错误的符号链接。
- **Cross-platform Consistency (跨平台一致性)**: 在 Linux / macOS 上使用原生 Symlink；在 Windows 上提供专属 Broker 服务机制，彻底告别频繁的 UAC 提权弹窗。
- **Centralized Management (中心化管理)**: 全局使用 SQLite 存储 Registry、Config、Harness 和 Group 数据，支持按分组管理技能与资源，一处修改，处处生效。
- **Harness Adapter (兼容适配)**: 管理 Claude Code 等 Harness 的路径约定；当前 `init` 创建内置 Claude Code 兼容链接，自定义 mapping 与启停标记仅保存在全局数据库。
- **High Performance (高性能)**: 基于 Rust 构建，操作幂等，执行极速。

## 🎯 Significance (意义)

随着 AI Agent 技术的普及，开发者通常需要在多个项目之间共享相同的提示词 (Prompts)、脚本技能或背景知识文档。传统做法往往依赖复制粘贴，导致版本碎片化，更新极其痛苦。

**Agent Linker** 的诞生正是为了解决这一痛点：
1. **Definition (定义) 统一**: 统一提供全局的 Skill 与 Resource 抽象标准，构建专属的 AI 知识库。
2. **零冗余**: 利用操作系统的符号链接特性，实现多工程复用而不在磁盘上产生副本。
3. **消除 Windows 权限痛点**: 长期以来，Windows 下创建符号链接需要管理员权限，破坏了自动化的流畅体验。本项目结合专用的本地服务进程，为 Windows 开发者带来了丝滑的链接体验。

## 📦 Installation & Configuration (安装配置)

当前 MVP 版本：`0.1.0`。发布前检查清单见 [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md)，变更记录见 [CHANGELOG.md](CHANGELOG.md)。

### 1. Release 安装 (推荐)

Linux:

```bash
curl -fsSL https://github.com/winddxr/agent-linker/releases/latest/download/aglink-installer.sh | sh
```

Windows:

```powershell
irm https://github.com/winddxr/agent-linker/releases/latest/download/aglink-installer.ps1 | iex
```

Windows 单用户安装默认使用 `C:\Program Files\aglink`（支持 `-InstallDir`）：`bin/` 保存程序，`data/` 保存数据库及 SQLite 日志，`backups/` 保存默认备份。安装器只对保护目录写入、数据目录赋权及必要的 Broker 安装提权；用户 PATH、旧库快照导入和数据库初始化由外层用户进程执行。已有旧默认库会在新目标尚不存在时导入并保留原库；升级保留数据与备份。portable zip 携带已初始化的同目录数据库。

卸载:

```powershell
irm https://github.com/winddxr/agent-linker/releases/latest/download/aglink-uninstall.ps1 | iex
```

默认卸载保留数据库、备份及共享 Broker 文件与维护脚本；`-Purge` 清理本安装的已知数据库文件与默认备份，`-RemoveBroker` 单独控制服务卸载。自定义数据覆盖目录只清理已知 SQLite 文件，保留未知内容。

### 2. 从源码安装

确保你的系统已安装 [Rust 与 Cargo](https://rustup.rs/) 环境。在本项目根目录执行：

```bash
cargo install --path .
aglink db migrate
```
*(注：后续发布到 crates.io 后可通过 `cargo install aglink` 安装)*

### 3. 各平台环境配置

#### 🐧 Linux
Linux 使用 Rust 标准库创建原生符号链接；阶段 8 已在本机 Ubuntu WSL2 环境验证该路径。

#### 🍎 macOS
macOS 与 Linux 共用 Unix 标准库符号链接代码路径，但当前暂未取得 macOS 实机验证结果。发布说明中应保持这一限制，不把 macOS 标为已验证平台。

#### 🪟 Windows (特别说明)
在 Windows 系统下，创建符号链接默认需要管理员权限。为了保证安全与自动化流程的顺畅，**Windows 用户必须安装配套的 Broker 服务**。

Windows Release 包会携带固定版本的 `WinSymlinksBroker` 运行时。推荐使用 `aglink-installer.ps1` 一次完成 `aglink` 安装、Broker 检测与必要的服务安装。Agent Linker 默认通过安全的本地 IPC 与该 Broker 服务通信。阶段 8 已验证 Windows 默认 backend 为 `windows-broker`，不会先直接创建再自动 fallback。

## 🚀 Quick Start (快速开始)

源码安装或手工解压后，先运行 `aglink db migrate`；安装脚本和 portable 包已完成初始化。然后初始化项目并查看 Harness 定义：

```bash
aglink init
aglink harness list
aglink harness show claude
```

> **注意**: 完整命令使用方法与用途分类，请查阅详细的 [CLI Command Reference (命令行指南)](CLI_REFERENCE.md)。同时，也可通过 `aglink --help` 随时获取内建帮助。

日常使用普通用户身份。Agent 的持久化写操作默认经宿主授权在沙箱外执行；已初始化数据库的只读查询和链接 dry-run 可在沙箱内执行。权限不足时 CLI 明确失败，不自行解除沙箱。正常 Windows 安装从有效 receipt 和 exe 布局定位数据库；显式设置不同的 `AGLINK_DATA_DIR` 会主动切换数据配置。其他安装形态跨入口使用时应显式统一该目录。`aglink db path` 可检查实际目标；普通读写均不会自动建库或迁移；显式 `aglink db migrate` 升级到 Schema v3，保留旧 ID 和引用。`db check` 验证五表和 SQLite 完整性，不健康返回非零。

## 📖 Architecture & Contract (架构与契约)

项目的设计理念与模块拆分严格遵循了以下原则。对于开发与贡献者，核心的 Acceptance Criteria (验收标准) 请参阅 `dev-docs/architecture.md`。

### Key Terms (关键术语)
- **Symlink**: 操作系统级符号链接，非快捷方式，非硬链接。
- **Agent Skill**: 按用途命名的目录，包含 AI Agent 可使用的提示词、脚本或文档等资源。
- **Harness**: 运行 Agent 的软件系统，如 Claude Code；`aglink harness` 管理兼容路径与适配设置，不启动或停止该工具。旧 `framework` 命令已移除。
- **Linkable Item**: Skill 与 Resource 的统一抽象，是工具管理的基本链接单元。

### 核心模块
- **Symlink Core**: 封装操作系统差异，保障创建与销毁的幂等性。
- **Global Store & Registry**: 全局 SQLite 状态存储。
- **CLI Command Surface**: 用户交互边界，输出结构化诊断信息。
