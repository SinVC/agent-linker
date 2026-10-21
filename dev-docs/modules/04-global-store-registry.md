# Global Store & Registry

## 设计目标

使用全局 SQLite 保存配置、Registry、Harness Adapter、Group 和机器级元数据，并与项目 manifest 明确分工。

## 职责边界

- 解析全局数据库路径。
- 管理 SQLite 连接和 schema migration。
- 保存 Harness Adapter、Linkable Item Registry、Group 和用户级偏好。
- 提供数据库路径、迁移和可写性诊断。
- 向 command 层暴露结构化 API，不暴露 SQL。

## 架构约束

- 全局 SQLite 不记录每个项目的实际链接状态。
- 项目 manifest 不记录全局 Registry 或用户偏好。
- MVP 不引入项目级配置文件。
- SQLite schema 必须版本化 migration。
- Harness 改名仅调整命令和领域术语；schema 保留 `frameworks`、`framework_mappings`、`framework_id`，已有数据、ID、启用标记和自定义映射继续可用。

## 接口契约

默认数据库路径：

- Windows：`%LOCALAPPDATA%\agent-linker\agent-linker.db`
- macOS：`~/Library/Application Support/agent-linker/agent-linker.db`
- Linux：`$XDG_DATA_HOME/agent-linker/agent-linker.db`
- Linux fallback：`~/.local/share/agent-linker/agent-linker.db`

路径覆盖规则：

- `AGLINK_DATA_DIR` 可显式指定 Agent Linker 数据目录，数据库固定为该目录下的 `agent-linker.db`。
- portable release 可将 `agent-linker.db` 放在可执行文件同目录；当该文件存在且未设置 `AGLINK_DATA_DIR` 时，使用该 portable 数据库。
- 普通安装模式不默认写入可执行文件同目录。
- `aglink db path` 必须能展示实际路径和解析原因。

SQLite 保存：

- Harness Adapter 与映射规则。
- Skill / Resource / Linkable Item Registry。
- Group 定义。
- 来源路径、来源类型、版本、commit、描述等元数据。
- 用户级偏好设置。

## 关键决策

- Windows 默认使用 `%LOCALAPPDATA%`，因为数据库包含本机路径和本机状态，不适合作为 roaming 数据。
- 普通安装不默认写入 exe 同目录，因为该位置可能不可写；portable release 通过随包放置 `agent-linker.db` 显式启用同目录数据库。
- command 层不直接写 SQL。
- migration 在数据库初始化或显式 `aglink db migrate` 时运行。
- 当前项目链接状态只保存在 `.agents/links.toml`。

## 验收口径

- 数据库路径解析在三类主流平台和 portable 模式下可诊断。
- migration 状态可检查、可显式执行。
- Registry 与 Harness Adapter 可通过结构化 API 访问。
- manifest 与 SQLite 不互相替代、不重复承担对方职责。
