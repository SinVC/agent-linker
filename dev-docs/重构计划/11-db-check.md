# 11 · 数据库检查状态与退出码

返回 [TODO](README.md)。前置：[09](09-schema-v3.md)、[10](10-group-summaries.md)。依据：第五章第 2 节 db check、第八章阶段二第 3 项。条件文档：[04](../modules/04-global-store-registry.md)、[06](../modules/06-cli-command-surface.md)。

## 本步结果与边界

让数据库健康检查依据实际结构和 SQLite 完整性，并可靠向调用脚本返回非零退出码。只修改 check 及其报告消费者，不修改迁移策略或引入修复命令。

## 代码入口

- [DbCheckStatus/DbCheckReport/check_database](../../src/core/store.rs)。
- [CommandReport::is_success/run_db](../../src/commands/mod.rs)、[CLI 退出码与报告](../../src/cli.rs)、[Doctor 消费](../../src/core/project_links.rs)。
- [诊断和退出码黑盒测试](../../tests/phase2_blind.rs)、[Doctor 空 Harness 测试](../../src/core/project_links_tests.rs)。

## 修改清单

1. check 观察缺库、旧/新 schema 和当前 schema，不以普通 open_ro 的版本拒绝替代诊断；检查过程不创建数据库、不迁移、不 Seed。
2. 当前 schema 检查 `sqlite_master` 中五张核心表 `harnesses`、`harness_mappings`、`linkable_items`、`groups`、`group_items` 齐全，且 `PRAGMA quick_check` 通过。
3. 删除以 Harness/mapping 行数大于零作为健康条件的做法；行数不决定结构健康。
4. 用明确枚举区分 `Missing`、`SchemaMismatch`、`ReadOnly`、`LockedBusy`、`HealthyWritable`，不再用 `.is_ok()` 将权限、锁竞争等所有异常压成 `writable = false`。结构不完整或 quick_check 失败不得报告健康。
5. `CommandReport::DbCheck(report)` 的 `is_success` 根据健康结果返回，CLI 不健康时返回非 0；同步普通、verbose 和 Doctor 的状态呈现，不依赖字符串解析分类。
6. 同步模块 04、06 的检查标准和退出码契约；安装脚本调用该结果的核对留给 13。

## 验收

- 临时库分别覆盖 Missing、SchemaMismatch、ReadOnly、LockedBusy、HealthyWritable；受当前平台权限影响的项明确记录实际覆盖方式。
- 删除 `group_items` 后必须不健康；空但结构完整的 Harness 表不因条目数量为零被误判；quick_check 不通过时失败。
- 不健康 DbCheck report 的 `is_success()` 为 false，CLI 实际进程退出码非 0；健康返回 0。
- 检查缺库不会创建目录或数据库，检查旧库不修改版本与数据。
- 运行 `cargo fmt --check`、`cargo test --locked`，实际 CLI 退出码验证只使用临时库环境。

## 执行记录

进度、验证结果与未解决问题见[本步进度](../重构编排/steps/11-progress.md)。
