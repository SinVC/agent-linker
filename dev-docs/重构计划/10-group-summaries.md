# 10 · 分组列表聚合查询

返回 [TODO](README.md)。前置：[09](09-schema-v3.md)。依据：第五章第 2 节 list_groups、第八章阶段二第 3 项。条件文档：[04](../modules/04-global-store-registry.md)、[06](../modules/06-cli-command-surface.md)。

## 本步结果与边界

让 `group list` 只加载摘要，消除逐组加载全部成员的 N+1 查询。group show 和实际需要成员的 group link/unlink 保留完整成员查询。

## 代码入口

- [GroupSummary/Store::list_group_summaries](../../src/core/registry.rs)。
- [CommandReport::GroupList/run_group](../../src/commands/mod.rs)、[CLI group list 输出](../../src/cli.rs)。
- [空组、多组及成员变化黑盒验收](../../tests/phase2_blind.rs)。

## 修改清单

1. 在 Registry 的 Store API 提供轻量 `list_group_summaries`，摘要只包含本用例需要的 id、name、成员数量，不为 list 填充完整 `Group.items`。
2. 使用裁决给定的单条聚合查询：

   ```sql
   SELECT g.id, g.name, COUNT(gi.item_id)
   FROM groups g
   LEFT JOIN group_items gi ON gi.group_id = g.id
   GROUP BY g.id
   ORDER BY g.name;
   ```

3. group list 的 command report 与 CLI 输出改用摘要，保持名称、数量的用户展示；不再调用 group show 或每组成员查询。
4. 删除无消费者的旧列表重型加载路径，同步模块 04、06 的 list/show 数据边界，不引入分页、缓存或新的列表字段。

## 验收

- 临时库覆盖零分组、空分组、多分组、多个成员和成员移除，数量与排序正确，空组显示 0。
- list 路径执行单条聚合 SQL，不依赖成员反序列化；show/link/unlink 仍可取得完整成员。
- 运行 `cargo fmt --check`、`cargo test --locked`。

## 执行记录

进度、验证结果与未解决问题见[本步进度](../重构编排/steps/10-progress.md)。
