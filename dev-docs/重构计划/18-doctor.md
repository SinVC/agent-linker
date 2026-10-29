# 18 · 拆出 Doctor 模块

返回 [TODO](README.md)。前置：[11](11-db-check.md)、[17](17-project-context.md)。依据：第七章 Doctor 职责、第八章阶段三第 2 项。条件文档：[01](../modules/01-symlink-core.md)、[06](../modules/06-cli-command-surface.md)、[07](../modules/07-code-architecture-verification.md)。

## 本步结果与边界

将独立诊断编排从 project_links 迁到 `core/doctor.rs`。仅移动和接线现有诊断能力，不增加诊断项、修复操作或新的探测后端。

## 代码入口

- [DoctorReport/DoctorCheck](../../src/core/project_links.rs#L133)、[doctor_current_project/doctor_project](../../src/core/project_links.rs#L586)、[check_broker_probe](../../src/core/project_links.rs#L717)。
- [Doctor 回归测试](../../src/core/project_links.rs#L1842)、[core 模块列表](../../src/core/mod.rs#L1)。
- [Doctor command/report](../../src/commands/mod.rs#L177)、[run](../../src/commands/mod.rs#L240)、[CLI report_lines](../../src/cli.rs#L582)。

## 修改清单

1. 新建 `core/doctor.rs`，迁入 Doctor 报告、检查项、数据库/Manifest/Harness/backend 诊断与 Windows Broker 探测；对应测试随职责迁移。
2. project_links 仅保留链接、状态、解绑、清理与 LinkPlan，不保留 Doctor 转发函数或重复导出。
3. commands 通过独立 Doctor core 入口编排；路径使用 01 的项目根定位，数据库诊断使用 Store::check 的结构化状态，不重新实现 check。
4. 保留原有 verbose 诊断、Broker 可用性和系统/Broker 错误码传递；平台条件编译在迁移中保持正确，21 再统一清理平台桩代码。
5. 更新 core 模块列表、commands/cli imports、模块 06/07 和架构导航中的 Doctor 位置；新文件创建后更新本计划代码链接。

## 验收

- Doctor 原有数据库、Manifest、Harness、Provider/Broker 场景在新模块通过；不健康报告依旧驱动非零退出码。
- mock/临时库和临时项目测试不依赖本机在役数据库；真实 Windows Broker 探测结果单独记录。
- project_links 不再包含诊断编排。运行 `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`。

## 执行记录

未执行。完成时填写移动符号、新文件位置和验证结果，再勾选 TODO 18。
