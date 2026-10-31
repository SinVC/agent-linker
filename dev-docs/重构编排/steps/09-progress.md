# 09 · Schema v3 迁移与数据模型同步

状态：完成，阶段二范围内当前平台验收通过（2026-10-31）；平台限制与前置差异见未解决问题。

[修改范围与契约](../../重构计划/09-schema-v3.md)

## 完成内容

- v1/v2 升级链保留；v2→v3 在同一事务完成建表、INSERT SELECT、旧表删除/重命名、索引及版本更新。
- frameworks/framework_mappings 映射为 harnesses/harness_mappings；删除 config、enabled_by_default、单值来源及未消费 repo/version/commit 字段。
- 保留旧 ID、成员和 mapping 引用；unix 字符串时间转 i64 秒。Resource 目标目录归一；持久化 default_link_name 并由 add/refresh/rename 维护。
- 新 Item/Group ID 为 itm_<16hex>/grp_<16hex>，采用 SQLite randomblob，不依赖名称；查询保持精确 ID 优先于名称，避免新 ID 格式与合法名称重合时误选；唯一性由目标 Schema 约束负责，无冗余预查。
- bool 原生读写；Manifest 接受旧 unix 字段，新写整数；time crate 仅用于 UTC 时间显示。

涉及：src/core/store.rs、linkable.rs、registry.rs、harness.rs、util.rs、manifest.rs、cli.rs、Cargo.toml/Cargo.lock。

## 隔离验证

独立 GPT-6.1 Sol high 子代理，仅获得目标路径/签名、类型、业务验收标准，不读取业务实现。旧内联测试机械提取为独立 *_tests.rs，便于遵守隔离；无效旧 config 测试移除，其通用覆盖保留。

- 空库、v1、v2 临时 fixture 升级到3；fixture 同时包含启用自定义 Harness 与禁用 Claude、自定义 file/directory mapping、普通/alias Skill、文件/alias目录 Resource、四成员 Group 及旧 Manifest；逐项核对 ID/引用/时间。
- 跨类型/大小写name、Skill默认名、Resource归一目录+默认名、mapping联合唯一及Group名称唯一通过。
- 唯一冲突迁移整体回滚，原表、版本和数据保留；成功迁移不丢子表。
- 随机ID格式/重新创建、rename保留ID/source/项目、bool、UTC verbose和旧Manifest读取通过。

最终隔离验证全部退出 0：

- cargo fmt --check
- cargo check --locked
- cargo test --locked --quiet：80 单元测试通过、28 CLI 黑盒测试通过、2 项目发现测试通过；3 项既有真实 Broker 测试忽略；doc tests 0。
- cargo run --locked -- --help
- 文档相对链接目标存在性检查：0 缺失。

所有测试的编写、适配和执行均由独立 GPT-6.1 Sol high 子代理完成；未向其提供业务实现或允许其读取业务源码。

## 失败与修复记录

- 额外盲测曾发现：以现有 Skill ID 命名另一 Resource 后，resource show <Skill ID> 错误命中 Resource 名称。已统一为全局精确 ID 优先解析，typed 命令再检查类型，共用一次查询，不限制合法名称；完整复验通过，同类型/跨类型 Item、group add 与 Group 的 ID 优先用例均通过。

## 未解决问题

- 覆盖界限：迁移约束冲突采用代表性 fixture，未穷举每种约束冲突组合；Resource source 无 basename 时回退 name 的既有分支未独立实跑。
- 无需要用户决策的阻塞。保留已有修改，不改在役数据。
