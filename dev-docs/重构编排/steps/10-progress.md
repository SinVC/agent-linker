# 10 · 分组列表聚合查询

状态：完成，阶段二范围内当前平台验收通过（2026-10-31）；平台限制与前置差异见未解决问题。

[修改范围与契约](../../重构计划/10-group-summaries.md)

## 完成内容

- Store::list_group_summaries 只返回 id/name/item_count。
- list 使用裁决中的单条 LEFT JOIN + COUNT + GROUP BY + ORDER BY 聚合 SQL；删除旧逐组成员加载列表。
- show/link/unlink 保留完整 Group.items；不增加缓存、分页或额外摘要字段。

涉及：src/core/registry.rs、commands/mod.rs、cli.rs。

## 隔离验证

独立 GPT-6.1 Sol high 子代理，仅获得目标路径/签名、类型、业务验收标准，不读取业务实现。旧内联测试机械提取为独立 *_tests.rs，便于遵守隔离；无效旧 config 测试移除，其通用覆盖保留。

- 零分组、空组0、多分组排序、增删成员后的数量、show完整成员均通过；临时库非法成员枚举下 list 仍返回摘要、show 反序列化失败，证明 list 不加载成员。
- 单条聚合SQL由主代理实现核对；盲测黑盒验证输出和成员解析解耦，不声称通过公开trace观测SQL条数。

最终隔离验证全部退出 0：

- cargo fmt --check
- cargo check --locked
- cargo test --locked --quiet：80 单元测试通过、28 CLI 黑盒测试通过、2 项目发现测试通过；3 项既有真实 Broker 测试忽略；doc tests 0。
- cargo run --locked -- --help
- 文档相对链接目标存在性检查：0 缺失。

所有测试的编写、适配和执行均由独立 GPT-6.1 Sol high 子代理完成；未向其提供业务实现或允许其读取业务源码。

## 未解决问题

- 无需要用户决策的阻塞。保留已有修改，不改在役数据。
