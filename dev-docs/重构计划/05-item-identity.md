# 05 · 注册项标识符与 alias 查询空间隔离

返回 [TODO](README.md)。前置：[02](02-manifest-origin.md)、[04](04-symlink-replace.md)。依据：第四章 P0-2、第八章阶段一第 4 项、第九章第 2 项。条件文档：[04](../modules/04-global-store-registry.md)、[05](../modules/05-linkable-item.md)、[06](../modules/06-cli-command-surface.md)。

## 本步结果与边界

先修复查询误选注册项：alias 不再是查找标识符。跨类型全局 name 唯一与默认目标唯一约束在 09 随 Schema v3 落地，本步不添加临时索引、额外扫描或新的命名规则。

## 代码入口

- [find_item_with_connection](../../src/core/registry.rs#L313)、[find_any_item_with_connection](../../src/core/registry.rs#L334)、[find_item_by_id](../../src/core/registry.rs#L367)。
- [rename/remove/refresh](../../src/core/registry.rs#L123)、[group 成员解析](../../src/core/registry.rs#L466)。
- [LinkableItem::link_name](../../src/core/linkable.rs#L79)、[link_name_for_request](../../src/core/registry.rs#L717)。
- [项目 link 标识符消费](../../src/core/project_links.rs#L201)、[既有命名测试](../../src/core/registry.rs#L954)。

## 修改清单

1. 所有注册项标识符解析只匹配 name 或精确主键 id，删除 SQL 中的 `OR alias = ...`。覆盖 show、rename、remove、refresh、统一 link 和 group 成员解析的共用入口。
2. alias 仅覆盖默认落盘链接名；保留 Skill 与 Resource 的既有默认名计算以及 `--as` 单次覆盖，不将 alias 自动转为 name。
3. 更新旧的 alias 查询相关测试，增加 `alpha --alias beta` 与名称为 `beta` 的另一条注册项场景；Skill 用例可令后者 `--alias gamma`，避免默认目标重名先阻断注册。目标冲突与查询身份不得混为一谈。
4. rename 只修改全局注册项；保持内部 id、已有项目 Manifest 和磁盘路径。09 负责同步落库 `default_link_name` 的更新，不在本步加项目扫描。
5. 同步模块 05、06 的标识符、alias、rename 说明；注明 name 全局唯一约束的实施项为 09。

## 验收

- `remove beta`、`show beta`、`rename beta ...`、`refresh beta`、`link beta` 和 group 成员解析均选中 name 为 beta 的记录，不能选中 alpha。
- 只有 alias 匹配时查询失败；精确 id 查询仍可用；alias 仍决定默认链接文件名。
- 临时数据库与 mock 项目验证 rename 后 id 不变、原链接路径和内容不变，按 item_id 的解绑及分组解绑正常。
- 运行 `cargo fmt --check`、`cargo test --locked`。

## 执行记录

未执行。完成时填写修改文件与验证结果，再勾选 TODO 05。
