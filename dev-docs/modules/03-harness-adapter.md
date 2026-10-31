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
- `harness enable/disable` 和 mapping 增删只修改全局数据库。`init` 只读加载已启用 Harness 的 mappings；缺库时从同一个内置定义回退，不隐式建库；旧/新 schema 或其他存储错误正常返回。不支持 `init --harness`。
- `harness.rs::builtin_harnesses()` 是内置定义与 mapping 的唯一来源；仅显式迁移使用 `INSERT OR IGNORE` Seed，不覆盖用户 enabled、mapping 或时间。
- Schema v3 使用 `harnesses`、`harness_mappings`、`harness_id`；v1/v2 事务迁移保留 ID、启用状态和映射。时间为 i64 Unix 秒，bool 使用 SQLite 原生转换。
- Harness name 大小写无关唯一，mapping 的 `(harness_id, link_path)` 大小写无关唯一；所有写操作由同一 Store 事务完成。
- 后续 Harness 沿用相同 mapping 模型。
- mapping 创建 symlink 时必须走 Symlink Core。
- 新 mapping 的 link_path 去除 `./`、归一分隔符后存储；同 Harness 按受控路径键与旧记录判重。删除优先原始字面匹配，其次路径键匹配，按 ID 删除；历史等价重复不自动合并，可逐条删除。source_path 不额外规范化。

## 验收口径

- `init` 使用已启用映射；无启用 Harness 时不恢复 Claude，缺库时使用默认 Claude 映射且不创建全局目录。
- Harness 链接状态能写入并回读 manifest。
- 禁用或启用 Harness 只改变全局标记，不修改真实源或已有项目链接。
- 用户自定义 mapping 可查询、增删并由新 init 消费，复用受控项目路径与 Symlink Core；不自动创建任意 source。
