# Global Store & Registry

## 设计目标

使用全局 SQLite 保存 Registry、Harness Adapter、Group 和机器级元数据，并与项目 manifest 明确分工。

## 职责边界

- 通过 `core::paths` 取得全局数据库路径与解析原因；本模块负责数据库存储。
- 管理 SQLite 连接和 schema migration。
- 保存 Harness Adapter、Linkable Item Registry 和 Group。
- 提供数据库路径、迁移和可写性诊断。
- 向 command 层暴露结构化 API，不暴露 SQL。

## 架构约束

- 全局 SQLite 不记录每个项目的实际链接状态。
- 项目 manifest 不记录全局 Registry 或 Harness 设置。
- MVP 不引入项目级配置文件。
- SQLite schema 必须版本化 migration。
- Schema v4 使用 `harnesses`、`harness_mappings`、`linkable_items`、`groups`、`group_items`、`init_items` 六张表；不使用通用 config 表。升级保留全部 ID、成员引用、启用标记、自定义映射与时间。
- v3→v4 只新增 `init_items(item_id)`，外键关联 Registry，注册项删除时级联清理。迁移不注入默认技能；安装脚本负责注册随包技能及首次设置默认项。

## 接口契约

默认数据库路径：

- Windows 正常安装：`<installRoot>\data\agent-linker.db`，默认根为 `C:\Program Files\aglink`；未识别的安装形态回退到 `%LOCALAPPDATA%\agent-linker\agent-linker.db`。
- macOS：`~/Library/Application Support/agent-linker/agent-linker.db`
- Linux：`$XDG_DATA_HOME/agent-linker/agent-linker.db`
- Linux fallback：`~/.local/share/agent-linker/agent-linker.db`

路径覆盖规则：

- `AGLINK_DATA_DIR` 可显式指定 Agent Linker 数据目录，数据库固定为该目录下的 `agent-linker.db`。
- Windows `bin/aglink.exe` 的上一级有有效的本产品普通安装 receipt 时，优先于 portable 使用安装根下 `data/`；不依赖 `LOCALAPPDATA` 或数据库是否存在。Receipt 最小身份为 `schema_version == 1`、`product == "agent-linker"`、`portable == false`，不依赖 `command` 或 `installed_by` 的值；损坏或无法读取的 receipt 明确报错。
- portable release 可将 `agent-linker.db` 放在可执行文件同目录；当该文件存在且未设置 `AGLINK_DATA_DIR` 时，使用该 portable 数据库。
- 普通安装模式不默认写入可执行文件同目录。
- `aglink db path` 必须能展示实际路径和解析原因。
- 路径类型、环境解析、平台默认值及 Receipt 识别统一归 `core::paths`；普通且未达到 Windows 长路径阈值的磁盘 canonical 路径去除 `\\?\` 前缀用于存储、展示和比较，超长路径及其他前缀保留。

SQLite 保存：

- Harness Adapter 与映射规则。
- Skill / Resource / Linkable Item Registry。
- Group 定义。
- 来源路径、来源 file/directory kind、持久化 default_link_name、目标目录及描述。

## 关键决策

- Windows 单用户正常安装通过目录 ACL 让安装发起用户可修改 `data/`、`backups/`，程序目录仍受保护；权限不足时不切换到其他库。
- 普通安装不默认写入 exe 同目录，因为该位置可能不可写；portable release 通过随包放置 `agent-linker.db` 显式启用同目录数据库。
- command 层不直接写 SQL。
- `store.rs::Store::open_ro` 使用 READ_ONLY 与 query_only；`open_rw` 使用无 CREATE 的 READ_WRITE 与 foreign_keys；两者只接受最新 schema，不迁移或 Seed。仅 `Store::migrate` 创建目录/库、执行迁移与 Seed，CLI 入口是显式 `aglink db migrate`。
- Registry/Harness 方法共用传入 Store 的连接，SQL 分别位于 registry.rs/harness.rs；每次写命令的读取、写入和返回查询在一个 rusqlite Transaction 中提交，失败回滚。迁移链和 Seed 同样整体提交。
- v2→v3 同事务表重建、INSERT SELECT、旧表删除/重命名、索引和版本更新；unix 时间字符串转 i64 秒，所有旧 ID 不变，冲突失败完整保留旧库，不自动改名或丢记录。
- name 跨 Skill/Resource 大小写无关唯一；Skill default_link_name 唯一，Resource 按归一 `(target_dir, default_link_name)` 唯一；Group name 唯一。依赖 SQLite 约束，不做重复唯一性预查。
- 新 Item/Group 使用 itm_<16hex>/grp_<16hex> 随机 ID。group add/remove 按 item_id 去重；add 已有成员跳过并提示，未知项仍失败，整批写入原子提交。
- group list 只用单条 LEFT JOIN/COUNT/GROUP BY 查询 id、name、成员数；show/link/unlink 加载完整成员。
- 查询和项目链接流程对全局库只读，不创建、迁移或 seed；缺库/旧 schema 提示 `db migrate`，新 schema 提示升级程序。`db check` 独立观察 schema，检查六表及 quick_check，再用不带 CREATE 的连接探测可写性；不建库。状态区分 Missing、SchemaMismatch、ReadOnly、LockedBusy、HealthyWritable 与结构/完整性错误 Invalid，只有 HealthyWritable 返回成功。行数不决定健康。
- 备份只读打开已有源库并生成 SQLite 一致快照，包含已提交 WAL 数据，保留源 schema，不覆盖已有目标；仅正常 Windows 安装默认写入安装根的 `backups/`，其余形态保留数据库同目录默认值。
- 当前项目链接状态只保存在 `.agents/links.toml`。

## 验收口径

- 数据库路径解析在三类主流平台和 portable 模式下可诊断。
- migration 状态可检查、可显式执行。
- Registry 与 Harness Adapter 可通过结构化 API 访问。
- manifest 与 SQLite 不互相替代、不重复承担对方职责。
