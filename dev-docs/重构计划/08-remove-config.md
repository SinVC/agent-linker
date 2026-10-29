# 08 · 移除 config 命令及运行时代码

返回 [TODO](README.md)。前置：[07](07-transactions.md)。依据：第六章第 1 项、第八章阶段二第 4 项。条件文档：[04](../modules/04-global-store-registry.md)、[06](../modules/06-cli-command-surface.md)、[07](../modules/07-code-architecture-verification.md)。

## 本步结果与边界

在 v3 删除表之前移除全部 config 消费入口，避免下一步出现命令仍访问已删除表的中间状态。本步不更改 schema 版本，历史建表 SQL 由 09 的迁移处理。

## 代码入口

- [ConfigCommand](../../src/commands/mod.rs#L41)、[CommandReport 的 config 分支](../../src/commands/mod.rs#L182)、[run_config](../../src/commands/mod.rs#L267)。
- [CliConfigCommand](../../src/cli.rs#L77)、[command_from_cli](../../src/cli.rs#L358)、[report_lines](../../src/cli.rs#L582)。
- 原 [ConfigEntry/ConfigUnsetReport](../../src/core/db.rs#L103)、[config CRUD](../../src/core/db.rs#L269)、`validate_config_key`，06 后位于 Store。
- [混合 config/backup 测试](../../src/core/db.rs#L1066)、[只读测试](../../src/core/db.rs#L923)。

## 修改清单

1. 删除顶层 `aglink config` 和 path/list/get/set/unset 的解析、Command、执行、report、格式化分支及专属测试。
2. 删除 ConfigEntry、ConfigUnsetReport、config CRUD 与参数校验，不添加替代 KV 或新的全局偏好系统。
3. 从混合测试中仅去掉 config 内容，保留数据库备份不覆盖、只读查询不建库/不迁移的有效覆盖；这些场景改用仍存在的业务表和命令。
4. 保留 v1/v2 历史迁移路径，09 通过 v3 删除存量 config 表。不要为此单独推出 schema 版本。
5. 同步模块 04、06、07 与架构中的当前命令/数据职责描述；用户文档在实际含有 config 当前用法的位置删除相应入口。仅在实施本步时检查 [README](../../README.md)、[CLI_REFERENCE](../../CLI_REFERENCE.md)、[中文命令参考](../../CLI_REFERENCE_zh.md)等当前用法，不重写历史任务记录。

## 验收

- CLI help 不再显示 config，解析 config 返回无此命令；db path 仍可用。
- 编译后没有 config 运行时命令、报告和数据库访问残留，历史迁移 SQL 可以继续存在直到 v3 删除表。
- backup 与只读测试仍验证原来的行为，不能随 config 测试一起丢失。
- 运行 `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`、`cargo run --locked -- --help`。

## 执行记录

未执行。完成时填写删除范围、保留的通用测试及验证结果，再勾选 TODO 08。
