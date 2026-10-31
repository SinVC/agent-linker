# Linkable Item

## 设计目标

将 Skill 和 Resource 统一建模为 Linkable Item，使注册、分组、链接、状态检查和清理共享一致的数据模型。

## 职责边界

- 定义 Linkable Item 领域模型。
- 处理 Skill 与 Resource 的业务约束。
- 检测 source 的 file / directory kind。
- 计算默认链接路径和链接名。
- 管理来源记录，不接管源内容。

## 架构约束

- Registry 存储在全局 SQLite。
- MVP 采用分散管理模式。
- `source_path` 永远记录绝对路径，可位于项目外。注册时 canonicalize 后经 `core::paths` 统一规范化 Windows 普通短磁盘路径，存储、展示和比较使用同一形式。
- 项目 Manifest 内的相对 source 经受控项目路径入口解析；不允许通过 symlink 父目录逃逸。
- 项目 manifest 中的 `link_path` 使用非空项目相对路径，拒绝绝对根、盘符前缀及 `..` 组件；解析与祖先检查统一通过 `core::paths`，已有父目录不可为 symlink。
- MVP 阶段 symlink 目标使用绝对 source path。

## 接口契约

Linkable Item 至少包含：

- `id`
- `name`
- `alias / link_name`
- `type`
- `kind`
- `source_path`
- `default_link_name`
- `default_target_dir`
- `description`
- `created_at`
- `updated_at`

Skill 约束：

- 类型为 `skill`。
- source 必须是目录。
- source 必须包含非空 `SKILL.md`。
- 默认链接到 `.agents/skills/<link_name>`。

Resource 约束：

- 类型为 `resource`。
- source 可以是文件或目录。
- 目标目录由注册或链接时指定。
- 默认链接名保持源文件或源目录原名称。

## 关键决策

- name 跨 Skill/Resource 全局大小写无关唯一。alias 只决定默认链接名，不参与注册项 ID/name 查询；精确 ID 优先于名称，不因新随机 ID 格式限制合法名称。
- Skill default_link_name 大小写无关唯一；Resource 按归一 `(target_dir, default_link_name)` 大小写无关唯一，允许不同目标目录的同名文件。
- default_link_name 持久化：alias 优先，Skill 否则使用 name，Resource 否则使用 source 原名并沿用 name 回退。add/refresh/rename 同步维护。
- 新 ID 使用 itm_<16hex> 随机格式；rename 保留 ID，不反向修改项目路径、Manifest 或 source；时间为 i64 Unix 秒，旧 Manifest unix 字段仍可读。
- 注册时自动检测 kind，用户不手动指定。
- 链接时实际 source kind 与 Registry 记录不一致时必须报错。
- 来源始终外部管理，不持久化单值 source_type/source_ownership 或未消费的 repo/version/commit 预留字段。
- 删除 Registry 项不自动删除已链接项目中的 symlink。
- 注册项 Manifest 使用 `origin = "item"`，旧 `framework_name = "registry"` 兼容推断为 item。`unlink --all` 仅解绑 item；分组解绑按当前成员 item_id 选择其全部 item 链接，保留 Harness 与分组外记录。

## 验收口径

- Skill 注册会校验目录和非空 `SKILL.md`。
- Resource 注册能识别文件和目录。
- Linkable Item 能稳定计算项目相对 link path。
- 已链接项目的清理由 `unlink` 或 `clean` 基于 manifest 完成。
- `aglink` 不复制、不移动、不删除 source 内容。

