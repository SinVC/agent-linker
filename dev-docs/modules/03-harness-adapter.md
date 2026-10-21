# Harness Adapter

## 设计目标

Harness 指运行 Agent 的软件系统，如 Claude Code。通过“真实源到 Harness 约定路径”的映射模型复用同一套内容源；适配启停不等于启动／停止 Harness 工具。

## 职责边界

- 管理 Harness 定义与 mapping。
- 决定哪些 Harness 默认启用。
- 为 `init` 和链接状态管理提供映射数据。
- 支持内置 Harness 和用户自定义 Harness。

## 架构约束

- `AGENTS.md` 与 `.agents/` 是唯一真实内容源。
- Harness 特定文件和目录不作为真实内容源。
- Harness 适配数据存储在全局 SQLite。
- 新 Harness 通过数据扩展，不通过运行时插件扩展。

## 接口契约

Harness 定义至少包含：

- `id`
- `name`
- `display_name`
- `built_in`
- `enabled_by_default`
- `enabled`
- `created_at`
- `updated_at`

Harness mapping 至少包含：

- `id`
- `harness_id`
- `source_path`
- `link_path`
- `link_kind`
- `required`

内置 Claude Code mapping（ID 为 `claude`）：

- `AGENTS.md` -> `CLAUDE.md`
- `.agents/skills/` -> `.claude/skills/`

## 关键决策

- 主命令为 `aglink harness`，旧 `aglink framework` 已移除，不提供兼容别名。
- MVP 内置 `claude`（显示名 Claude Code）并默认启用；当前无新增 Harness 的 CLI。
- `harness enable/disable` 和 mapping 增删只修改全局数据库。当前 `init` 固定使用内置 Claude Code 两条映射，不读取启用标记或自定义 mapping；不支持 `init --harness`。
- Rust 模块及类型使用 `harness` / `Harness`；SQLite 保留 `frameworks`、`framework_mappings` 和 `framework_id`，无需重建数据库。manifest 的兼容规则见 Project Init & Manifest。
- 后续 Harness 沿用相同 mapping 模型。
- mapping 创建 symlink 时必须走 Symlink Core。

## 验收口径

- `init` 使用内置映射生成兼容链接；后续接入全局 Harness 设置需另行实现与验证。
- Harness 链接状态能写入并回读 manifest。
- 禁用或启用 Harness 只改变全局标记，不修改真实源或已有项目链接。
- 用户自定义 mapping 可查询、增删，但当前不被 `init` 应用。
