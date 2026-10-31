# 11 · 数据库检查状态与退出码

状态：完成，阶段二范围内当前平台验收通过（2026-10-31）；平台限制与前置差异见未解决问题。

[修改范围与契约](../../重构计划/11-db-check.md)

## 完成内容

- check 独立于普通 open_ro 诊断，观察 Missing/SchemaMismatch/ReadOnly/LockedBusy/HealthyWritable；Invalid 承接结构/完整性损坏。
- 当前schema检查五张核心表及quick_check；行数不决定健康。检查不建库、不迁移、不Seed。
- CommandReport::is_success 依健康状态决定，CLI非健康返回非零；普通/verbose/Doctor直接消费状态与细节，Doctor不再要求Harness非空。

涉及：src/core/store.rs、commands/mod.rs、cli.rs、project_links.rs。

## 隔离验证

独立 GPT-6.1 Sol high 子代理，仅获得目标路径/签名、类型、业务验收标准，不读取业务实现。旧内联测试机械提取为独立 *_tests.rs，便于遵守隔离；无效旧 config 测试移除，其通用覆盖保留。

- Missing、旧/新SchemaMismatch、Windows只读文件属性ReadOnly、临时写锁LockedBusy、HealthyWritable及实际进程退出码通过。
- 缺group_items、损坏数据库/quick_check、空但结构完整Harness表通过；缺库不建目录，旧库不改版本。

最终隔离验证全部退出 0：

- cargo fmt --check
- cargo check --locked
- cargo test --locked --quiet：80 单元测试通过、28 CLI 黑盒测试通过、2 项目发现测试通过；3 项既有真实 Broker 测试忽略；doc tests 0。
- cargo run --locked -- --help
- 文档相对链接目标存在性检查：0 缺失。

所有测试的编写、适配和执行均由独立 GPT-6.1 Sol high 子代理完成；未向其提供业务实现或允许其读取业务源码。

## 未解决问题

- 无需要用户决策的阻塞。保留已有修改，不改在役数据。
