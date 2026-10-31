# Agent Linker 架构导航

本文档体系取代 `docs/architecture_decisions.md`，作为后续设计与实现的唯一事实来源。旧源文件只保留为历史材料，不再作为开发依据。

## 全局原则

- 根文档只维护全局视图、模块索引、依赖关系和实施路径。
- 模块文档只记录设计目标、职责边界、架构约束、接口契约、关键决策和验收口径。
- 不在文档中记录具体实现代码、开发常识或可由上下文直接推导的信息。

## 核心术语

- Symlink：操作系统级符号链接，非快捷方式，非硬链接。
- Agent Skill：按用途命名的目录，包含 AI Agent 可使用的提示词、脚本或文档等资源。
- Harness：运行 Agent 的软件系统，如 Claude Code；本项目管理其兼容路径映射和适配设置，不启停该工具。
- Linkable Item：Skill 与 Resource 的统一抽象，是工具管理的基本链接单元。

## 产品边界

- 工程名：`agent_linker`。
- CLI 命令名：`aglink`。
- `AGENTS.md` 与 `.agents/` 是真实源。
- Harness 特定文件和目录只作为兼容性 symlink 投射。
- MVP 采用单 crate 和内部模块，不采用 workspace 或运行时插件系统。

## 模块索引

| 模块 | 文档 | 主要职责 | 优先级 |
| --- | --- | --- | --- |
| Symlink Core | [01-symlink-core.md](modules/01-symlink-core.md) | 跨平台 symlink 抽象、状态、冲突、错误、Windows Broker 集成和后端策略 | P0 |
| Project Init & Manifest | [02-project-init-manifest.md](modules/02-project-init-manifest.md) | `init` 目标结构、幂等规则、`.gitignore`、项目链接状态清单 | P0 |
| Harness Adapter | [03-harness-adapter.md](modules/03-harness-adapter.md) | 真实源到 Agent Harness 约定路径的映射 | P0 |
| Global Store & Registry | [04-global-store-registry.md](modules/04-global-store-registry.md) | Store 三入口、Schema v3、Registry、Group 与数据库诊断 | P1 |
| Linkable Item | [05-linkable-item.md](modules/05-linkable-item.md) | Skill / Resource 统一模型、来源管理、路径规则 | P1 |
| CLI Command Surface | [06-cli-command-surface.md](modules/06-cli-command-surface.md) | 顶层命令、命令职责和用户可见行为边界 | P1 |
| Code Architecture & Verification | [07-code-architecture-verification.md](modules/07-code-architecture-verification.md) | 代码层职责、依赖规则、扩展策略和测试边界 | P1 |

## 依赖关系

```text
CLI Command Surface
  -> Project Init & Manifest
  -> Harness Adapter
  -> Global Store & Registry
  -> Linkable Item
  -> Symlink Core

Project Init & Manifest
  -> Harness Adapter
  -> Symlink Core

Harness Adapter
  -> Global Store & Registry
  -> Symlink Core

Linkable Item
  -> Global Store & Registry
  -> Symlink Core

Global Store & Registry
  -> Code Architecture & Verification
```

## 实施路径

已定稿重构的分步执行入口见[重构 TODO](重构计划/README.md)，每步的修改范围、代码位置和验收要求保存在对应下钻文档中。

1. 建立 Symlink Core：统一链接创建、读取、删除、状态检查、错误模型和 Windows Broker 默认后端。
2. 建立 Project Init & Manifest：完成 `aglink init` 的真实源结构、Harness 兼容链接、manifest 写入和 `.gitignore` 管理。
3. 建立 Harness Adapter：以 builtin_harnesses 统一内置定义；init 只读消费全局启用标记与自定义 mapping，缺库使用内置默认映射。
4. 建立 Global Store & Registry：完成 Store 三入口、Schema v3 事务迁移、数据库诊断和 Registry 持久化边界。
5. 建立 Linkable Item：实现 Skill / Resource 注册、刷新、路径规则和链接目标计算。
6. 建立 CLI Command Surface：开放 `db`、`harness`、`skill`、`resource`、`group`、`link`、`unlink`、`status`、`clean`、`doctor`。
7. 完成验证闭环：以 mock symlink、临时数据库、临时 manifest 和平台专项测试覆盖核心流程。

## 发布分发

Release、安装、卸载、Windows Broker 打包和 portable 形态的约束见
[release-distribution.md](release-distribution.md)。

Windows 单用户安装目录调整的实施范围与验收要求见
[handoff-windows-install-data-layout.md](handoff-windows-install-data-layout.md)。
