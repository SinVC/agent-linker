# 08 · 移除 config 命令及运行时代码

状态：完成，阶段二范围内当前平台验收通过（2026-10-31）；平台限制与前置差异见未解决问题。

[修改范围与契约](../../重构计划/08-remove-config.md)

## 完成内容

- 删除 config CLI、Command、报告、CRUD、校验及专属测试；不增加替代 KV。
- 历史 v1/v2 建表 SQL 保留，由 v3 删除 config 表；通用 backup/只读测试保留并改用现有业务。

涉及：src/cli.rs、commands/mod.rs、core/store.rs、CLI_REFERENCE.md、CLI_REFERENCE_zh.md及模块契约。

## 隔离验证

独立 GPT-6.1 Sol high 子代理，仅获得目标路径/签名、类型、业务验收标准，不读取业务实现。旧内联测试机械提取为独立 *_tests.rs，便于遵守隔离；无效旧 config 测试移除，其通用覆盖保留。

- 真实 CLI help 不显示 config，config 解析失败；db path 成功且不建库。
- 编译通过，运行时代码无 config 消费入口；WAL备份、只读不建库/不迁移和备份不覆盖覆盖仍通过。

最终隔离验证全部退出 0：

- cargo fmt --check
- cargo check --locked
- cargo test --locked --quiet：80 单元测试通过、28 CLI 黑盒测试通过、2 项目发现测试通过；3 项既有真实 Broker 测试忽略；doc tests 0。
- cargo run --locked -- --help
- 文档相对链接目标存在性检查：0 缺失。

所有测试的编写、适配和执行均由独立 GPT-6.1 Sol high 子代理完成；未向其提供业务实现或允许其读取业务源码。

## 未解决问题

- 无需要用户决策的阻塞。保留已有修改，不改在役数据。
