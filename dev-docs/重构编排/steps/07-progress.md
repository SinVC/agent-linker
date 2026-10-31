# 07 · 写命令事务与分组输入幂等

状态：完成，阶段二范围内当前平台验收通过（2026-10-31）；平台限制与前置差异见未解决问题。

[修改范围与契约](../../重构计划/07-transactions.md)

## 完成内容

- Store 写用例统一使用 rusqlite Transaction；相关读取、全部写入、返回实体查询及 commit 在同一事务内。
- Item/Group CRUD、成员增删、Harness 启停/mapping 增删及迁移/Seed 具备事务边界。
- group add/remove 按解析后 item_id 去重；add 已有成员跳过并在报告中提示，未知项仍失败；只读数据不额外校验。

涉及：src/core/store.rs、registry.rs、harness.rs、commands/mod.rs、cli.rs。

## 隔离验证

独立 GPT-6.1 Sol high 子代理，仅获得目标路径/签名、类型、业务验收标准，不读取业务实现。旧内联测试机械提取为独立 *_tests.rs，便于遵守隔离；无效旧 config 测试移除，其通用覆盖保留。

- 触发器中途注入 group 批量 add/remove 失败，成员和更新时间回滚；Item rename 及 Group create 的结果查询失败回滚。
- 重复输入幂等、已有成员跳过、其他新成员加入及未知项失败行为通过。
- Item/Group 删除级联与来源文件保留通过；迁移唯一冲突保留完整旧库；Seed 第二次插入触发故障时，前次补插和 audit 副作用全部回滚。

最终隔离验证全部退出 0：

- cargo fmt --check
- cargo check --locked
- cargo test --locked --quiet：80 单元测试通过、28 CLI 黑盒测试通过、2 项目发现测试通过；3 项既有真实 Broker 测试忽略；doc tests 0。
- cargo run --locked -- --help
- 文档相对链接目标存在性检查：0 缺失。

所有测试的编写、适配和执行均由独立 GPT-6.1 Sol high 子代理完成；未向其提供业务实现或允许其读取业务源码。

## 未解决问题

- 覆盖界限：故障注入验证代表性的批量成员写入、Item rename、返回实体查询和 Seed；未对每个其他写入口逐一注入触发器。其余入口通过同一事务能力和正常读写用例验证。
- 无需要用户决策的阻塞。保留已有修改，不改在役数据。
