# 06 · Store 三入口与唯一内置 Seed 来源

返回 [TODO](README.md)。前置：[01](01-paths.md)、[05](05-item-identity.md)。依据：第四章 P0-5 整改第 1 项、第七章 Store 与 Registry/Harness 职责、第八章阶段二第 1、4 项。条件文档：[03](../modules/03-harness-adapter.md)、[04](../modules/04-global-store-registry.md)、[07](../modules/07-code-architecture-verification.md)。

## 本步结果与边界

将当前 `db.rs` 收敛为 `core/store.rs`，用统一 Store 分离日常读、日常写、显式初始化与迁移。当前 Schema 仍为 v2，事务边界在 07、v3 在 09、init 接入动态 Harness 在 12；本步不提前修改这些业务行为。

## 代码入口

- [db.rs 连接与迁移](../../src/core/db.rs#L196)、[open_migrated_connection](../../src/core/db.rs#L397)、[open_readonly_connection](../../src/core/db.rs#L438)、[seed_builtin_harnesses](../../src/core/db.rs#L563)。
- [RegistryStore](../../src/core/registry.rs#L42)及所有调用 `open_migrated_connection` 的 CRUD 函数。
- [Harness 读写](../../src/core/harness.rs#L122)、[built_in_claude/default_init_mappings](../../src/core/harness.rs#L50)。
- [commands 的数据库和 Registry 编排](../../src/commands/mod.rs#L240)、[core 模块导出](../../src/core/mod.rs#L1)。
- [项目连接入口](../../src/core/project_links.rs#L190)、[只读测试](../../src/core/db.rs#L923)、[Seed 测试](../../src/core/db.rs#L1025)。

## 修改清单

1. 将连接、迁移、backup、check 及报告类型移入 `core/store.rs`；路径类型已归 01 的 `paths.rs`。更新 core 模块、commands、registry、harness、project_links、cli 的 imports，不保留只转发的旧 db 模块。
2. 实现三个 Store 入口：

   | 入口 | 必须完成 | 不能隐式发生 |
   | --- | --- | --- |
   | `Store::open_ro` | READ_ONLY、`PRAGMA query_only = ON`、schema 等于当前 latest | 创建目录/数据库、DDL、迁移、Seed |
   | `Store::open_rw` | READ_WRITE 且无 CREATE、`PRAGMA foreign_keys = ON`、schema 等于当前 latest | 创建目录/数据库、DDL、迁移、Seed |
   | `Store::migrate` | 创建必要数据目录和数据库，执行版本迁移及初始化 Seed | 被普通查询或普通写操作间接调用 |

3. 缺库时普通读写均提示显式 `aglink db migrate`，旧 schema 同样要求 migrate，新于程序的 schema 提示升级程序。删除 `open_migrated_connection` 及默认连接套壳，不为普通写命令保留自动建库路径。
4. Registry 与 Harness 的业务访问落在 Store 上；SQL 分别留在 `registry.rs`、`harness.rs`，共用一个连接，去掉独立持有连接的重复 `RegistryStore` 包装。commands 按用途打开读或写 Store；已有项目 Provider 重载暂保留到 17。
5. `harness.rs` 提供 `builtin_harnesses()` 作为内置定义和 mapping 的唯一数据源，迁移时遍历并 `INSERT OR IGNORE`。删除存储模块重复硬编码的 Claude 数据值和覆盖式 Seed；重复 migrate 不覆盖用户 enabled、mapping 或 updated_at。
6. `db migrate` 调用唯一迁移入口。测试在使用临时库前显式 migrate，不靠普通 add/open 的副作用初始化。backup 保持已有只读快照与不覆盖目标语义，check 的诊断重构留给 11。
7. 同步模块 03、04、07 和架构模块导航中 Store、连接与内置定义来源的条目。

## 验收

- 临时缺库路径执行普通读写均失败且父目录、数据库不出现，包括无效 source 的 add；显式 migrate 后可以读写。
- open_ro 无法写；open_rw 外键开关确实开启；旧版/新版 schema 的普通连接不迁移、不 Seed。
- 连续读查询、普通写操作、重复 migrate 不刷新内置 Harness 的 updated_at；用户禁用状态和自定义 mapping 保留。
- 一个用例内部 Registry/Harness 查询复用传入 Store，不再打开第二个连接。
- 运行 `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`、`cargo run --locked -- --help`。检查新文件存在后更新本计划所有 `db.rs` 链接到相应新位置。

## 执行记录

未执行。完成时填写移动后的符号位置、修改文件和验证结果，再勾选 TODO 06。
