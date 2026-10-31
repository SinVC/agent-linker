# 07 · 写命令事务与分组输入幂等

返回 [TODO](README.md)。前置：[06](06-store.md)。依据：第四章 P0-5 的 SQLite 写事务、第六章第 4 项 group 输入处理、第八章阶段二第 1 项。条件文档：[03](../modules/03-harness-adapter.md)、[04](../modules/04-global-store-registry.md)、[06](../modules/06-cli-command-surface.md)。

## 本步结果与边界

以单次数据库写命令为事务边界，完成 group 输入去重与已在组内成员的幂等跳过。只处理全局 SQLite 写入，不处理文件系统批量链接，后者属于 16。

## 代码入口

- [Store::write/migrate](../../src/core/store.rs)：写用例、迁移和 Seed 的事务边界。
- [Item/Group CRUD 和成员增删](../../src/core/registry.rs)、[Harness 启停与 mapping 写入](../../src/core/harness.rs)。
- [GroupUpdateReport 消费](../../src/commands/mod.rs)、[跳过结果输出](../../src/cli.rs)、[故障注入黑盒测试](../../tests/phase2_blind.rs)。

## 修改清单

1. Store 上的每个数据库写命令使用 `rusqlite::Transaction` 包住该次用例的相关读取、全部写入与返回数据查询，成功后提交；失败整体回滚。包含 Item/Group CRUD、成员增删、Harness 启停和 mapping 增删；尚未下线的 config 写入口随之遵守同一边界。
2. group add/remove 对同一次输入解析出的 item_id 去重，重复输入不会中断整批操作。
3. group add 遇已在组内的成员时跳过并提示，继续处理其余成员；为现有报告补齐表达跳过结果所需的信息。输入中的不存在注册项仍按既有错误语义处理，不增加新的容错策略。
4. 显式迁移的单次迁移与 Seed 写入也有事务边界；v3 表重建的具体事务在 09 实现。`PRAGMA foreign_keys = ON` 继续由 open_rw 统一设置。
5. 同步模块 04、06 的事务与 group 行为契约；不改变 group link/unlink 的文件系统语义。

## 验收

- 临时库中对 group 批量增删和 Item rename 注入中途写失败，证明前序写入与更新时间均回滚。
- 重复输入只处理一次，group add 已在组内的项被跳过且有提示，其他新成员可正常加入。
- 删除 Item/Group 的外键级联符合定义，不修改来源文件和项目链接。
- migration/Seed 失败不留下该事务的部分数据。故障注入只用于临时数据库测试。
- 运行 `cargo fmt --check`、`cargo test --locked`。

## 执行记录

进度、验证结果与未解决问题见[本步进度](../重构编排/steps/07-progress.md)。
