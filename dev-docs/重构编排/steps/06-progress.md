# 06 · Store 三入口与唯一内置 Seed 来源

状态：完成，阶段二范围内当前平台验收通过（2026-10-31）；平台限制与前置差异见未解决问题。

[修改范围与契约](../../重构计划/06-store.md)

## 完成内容

- Store::open_ro/open_rw 只打开最新库；open_ro READ_ONLY/query_only，open_rw 无 CREATE/foreign_keys。缺库和旧 schema 提示显式 migrate，新 schema 提示升级。
- Store::migrate 是唯一建库、迁移、Seed 入口；Registry/Harness 共用 Store 连接，内置定义仅来自 builtin_harnesses，Seed INSERT OR IGNORE。
- backup 保留只读 WAL 快照和目标不覆盖语义；计划中的旧 db.rs 代码链接已指向 store.rs。

涉及：src/core/store.rs（取代 db.rs）、registry.rs、harness.rs、project_links.rs、commands/mod.rs、core/mod.rs。

## 隔离验证

独立 GPT-6.1 Sol high 子代理，仅获得目标路径/签名、类型、业务验收标准，不读取业务实现。旧内联测试机械提取为独立 *_tests.rs，便于遵守隔离；无效旧 config 测试移除，其通用覆盖保留。

- 缺库普通读写（含无效 source add）不生成目录/库；旧/新 schema 不迁移。
- PRAGMA query_only 与 foreign_keys 直接观测通过；只读连接写入失败。
- 读查询、普通写入及重复 migrate 不更改内置更新时间；禁用和自定义 mapping 保留；WAL 快照不改原库且不覆盖目标。

最终隔离验证全部退出 0：

- cargo fmt --check
- cargo check --locked
- cargo test --locked --quiet：80 单元测试通过、28 CLI 黑盒测试通过、2 项目发现测试通过；3 项既有真实 Broker 测试忽略；doc tests 0。
- cargo run --locked -- --help
- 文档相对链接目标存在性检查：0 缺失。

所有测试的编写、适配和执行均由独立 GPT-6.1 Sol high 子代理完成；未向其提供业务实现或允许其读取业务源码。

## 未解决问题

- 无需要用户决策的阻塞。保留已有修改，不改在役数据。
