# 09 · Schema v3 迁移与数据模型同步

返回 [TODO](README.md)。前置：[05](05-item-identity.md)、[06](06-store.md)、[07](07-transactions.md)、[08](08-remove-config.md)。依据：第四章 P0-2 的唯一性落地、第五章第 1 节与唯一约束查询整改、第六章第 1、2 项、第八章阶段二第 2 项、第九章第 2 项。条件文档：[03](../modules/03-harness-adapter.md)、[04](../modules/04-global-store-registry.md)、[05](../modules/05-linkable-item.md)、[07](../modules/07-code-architecture-verification.md)。

## 本步结果与边界

通过 v2 → v3 事务性表重建落地裁决给出的完整数据契约，同步其必须一并修改的 SQL、领域模型和输出消费者。本步不重做 group list、db check 或 init 编排；仅调整它们编译和读取新结构所必需的字段/表名。

## 代码入口

- 原 [LATEST_SCHEMA_VERSION](../../src/core/db.rs#L19)、[migrate_connection](../../src/core/db.rs#L468)、[Seed](../../src/core/db.rs#L563)，06 后位于 Store。
- [LinkableItem 与单值枚举](../../src/core/linkable.rs#L35)、[默认链接名计算](../../src/core/linkable.rs#L79)、[解析函数](../../src/core/linkable.rs#L248)。
- [Item 写入](../../src/core/registry.rs#L239)、[查询](../../src/core/registry.rs#L292)、[rename](../../src/core/registry.rs#L123)、[行映射](../../src/core/registry.rs#L588)、[唯一性预查](../../src/core/registry.rs#L641)、[ID 生成](../../src/core/registry.rs#L739)。
- [Harness 模型](../../src/core/harness.rs#L19)、[存储模型](../../src/core/harness.rs#L90)、[mapping SQL](../../src/core/harness.rs#L330)、[bool 转换](../../src/core/harness.rs#L395)。
- [timestamp/bool 工具](../../src/core/util.rs#L1)、[Manifest 时间字段](../../src/core/manifest.rs#L27)、[CLI 详情输出](../../src/cli.rs#L918)。

## 目标 Schema（按裁决原样落实）

```sql
CREATE TABLE harnesses (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name TEXT NOT NULL,
    built_in INTEGER NOT NULL CHECK (built_in IN (0, 1)),
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE harness_mappings (
    id TEXT PRIMARY KEY NOT NULL,
    harness_id TEXT NOT NULL REFERENCES harnesses(id) ON DELETE CASCADE,
    source_path TEXT NOT NULL,
    link_path TEXT NOT NULL COLLATE NOCASE,
    link_kind TEXT NOT NULL CHECK (link_kind IN ('file', 'directory')),
    required INTEGER NOT NULL CHECK (required IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (harness_id, link_path)
);
CREATE TABLE linkable_items (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('skill', 'resource')),
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    alias TEXT,
    default_link_name TEXT NOT NULL,
    source_path TEXT NOT NULL,
    source_kind TEXT NOT NULL CHECK (source_kind IN ('file', 'directory')),
    target_dir TEXT,
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX idx_skill_default_link_name
    ON linkable_items(default_link_name COLLATE NOCASE)
    WHERE kind = 'skill';
CREATE UNIQUE INDEX idx_resource_target_link_name
    ON linkable_items(target_dir COLLATE NOCASE, default_link_name COLLATE NOCASE)
    WHERE kind = 'resource';
CREATE TABLE groups (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE group_items (
    group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL REFERENCES linkable_items(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (group_id, item_id)
);
```

## 修改清单

1. 保留 v1/v2 升级链，在 v2 → v3 的同一事务内执行建新表、`INSERT SELECT`、旧表删除与重命名、索引创建和版本提交。表重建顺序必须保留子表关系；不通过清库、重置为 v1 或重生成存量 ID 完成迁移。
2. 数据映射如下：

   | 旧数据 | v3 数据 |
   | --- | --- |
   | `frameworks` | `harnesses`，去掉 `enabled_by_default` |
   | `framework_mappings.framework_id` | `harness_mappings.harness_id` |
   | 全部存量主键、成员引用 | 原值保留，已有 Manifest 的 item_id 仍有效 |
   | `unix:<seconds>` 时间字符串 | `i64` Unix 秒，created/updated/member 时间值保持 |
   | Item alias 非空时 | default_link_name 采用 alias |
   | 无 alias 的 Skill | default_link_name 采用 name |
   | 无 alias 的 Resource | default_link_name 采用 source 文件/目录原名，沿用现有回退规则 |
   | Resource target_dir | 归一化项目相对目录后持久化，用于目录与链接名联合判重 |
   | `config` 表 | 删除 |
   | Item 的 `source_type/source_ownership/version/commit_hash/repo_url/repo_commit` | 删除 |

3. 新 Item/Group 改用无业务语义的固定格式随机 ID，例如 `itm_<16hex>`、`grp_<16hex>`；保留所有旧 ID 和内置 Harness/mapping ID，不从名称生成新 Item/Group ID。
4. 同步全部 CRUD SQL、字段列表、行反序列化及报告。领域模型删除 `SourceType`、`SourceOwnership`、repo 预留字段和对应解析，Harness 删除无消费者的 `enabled_by_default` 字段；内置默认启用状态通过保留的 enabled 数据表达。
5. `default_link_name` 成为持久化字段并用于默认路径计算。add/refresh/rename 同步维护它：Skill 无 alias 的 rename 更新未来默认名，有 alias 时仍用 alias；Resource 默认名继续来自 source 或 alias。rename 不触碰已有项目路径、Manifest 或 source。
6. name 跨 Skill/Resource 全局大小写无关唯一。Skill 默认链接名唯一；Resource 按归一化后的 `(target_dir, default_link_name)` 唯一，不拦截不同目标目录的同名文件。
7. 删除 `ensure_name_available`、`ensure_link_name_available` 和 `ensure_group_name_available` 的冗余预查，依靠唯一约束，将 SQLite `CONSTRAINT_UNIQUE` 映射成清晰用户提示。迁移遇不满足目标约束的数据时事务失败并保留原库，不自动改名或丢弃记录。
8. bool 使用 rusqlite 原生读写，删除 `bool_to_i64/int_to_bool`。时间戳统一为 i64 Unix 秒，调整共享时间工具与消费者；既有 Manifest 的 `unix:` 字段保持可读。CLI verbose 将时间格式化为可读 UTC，不新增时间格式选项。
9. 同步模块 03、04、05、07 的最终表名、字段、唯一性与迁移契约；保留单 crate 和既有分层。

## 验收

- 临时新库可从空状态迁移至 v3；临时 v1/v2 库可顺序升级；重复 migrate 不改变用户状态和时间。
- v2 fixture 包含启用/禁用 Harness、自定义 mapping、Skill、文件/目录 Resource、alias、Group/成员及旧 Manifest 引用，升级后逐项核对值、ID、引用与时间；结构符合上述 SQL。
- 迁移中途失败或唯一约束失败后仍为完整旧库；成功重建不因外键级联丢成员或 mapping。
- 跨类型同名/大小写同名失败；同 Skill 默认名失败；不同 Resource 目录同名成功、相同目录同名失败；mapping 联合唯一生效。
- 新 ID 符合选定固定随机格式、rename 后 ID 不变；rename 不反向修改项目且后续解绑可按 item_id 匹配。
- CLI verbose UTC 时间、bool 读写及旧 Manifest 读取通过。运行 `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`。

## 执行记录

未执行。完成时填写迁移映射、fixture 覆盖、修改文件和验证结果，再勾选 TODO 09。
