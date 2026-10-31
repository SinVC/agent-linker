# 02 · Manifest 来源隔离与解绑范围

返回 [TODO](README.md)。前置：[01](01-paths.md)。依据：第四章 P0-3、第六章第 1 项 scope、第八章阶段一第 2 项、第九章。条件文档：[02](../modules/02-project-init-manifest.md)、[05](../modules/05-linkable-item.md)、[06](../modules/06-cli-command-surface.md)。

## 本步结果与边界

明确 Manifest 中 Harness 基础设施与注册项链接的来源，修复批量解绑范围，同时落地已裁决的 `.gitignore` 托管规则。本步仍使用当前序列化实现；serde + toml 替换属于 14，原子保存属于 03。

## 代码入口

- [LinkOrigin / LinkRecord](../../src/core/manifest.rs#L33)。
- [Manifest::validate](../../src/core/manifest.rs#L78)。
- [序列化与兼容读取](../../src/core/manifest.rs#L657)。
- [gitignore 托管块](../../src/core/manifest.rs#L475)。
- [item 记录与解绑选择](../../src/core/project_links.rs#L1117)。
- [来源隔离测试](../../src/core/project_links_tests.rs#L1113)。
- [旧清单往返测试](../../src/core/manifest_tests.rs#L572)。

## 修改清单

1. 条目引入 `origin = "harness" | "item"`。Harness 名称仅在 Harness 来源条目中有值，普通条目不再伪装为 `harness_name = "registry"`。同步所有记录构造、校验、序列化和读取点。
2. 兼容旧 Manifest：`framework_name = "registry"` 读取为 item 来源，其余旧 Harness 条目映射为 harness 来源；保留原有 `id`、`item_id`、路径和时间信息，不要求用户重建项目。
3. 删除恒定 `scope = "project"` 的运行时字段与新写出字段，旧清单中的 `scope` 仍能读取。只做已裁决字段兼容，不扩展其他清单能力。
4. `unlink --all` 只选择 item 来源记录。`group unlink <group>` 只选择 item 来源且 `item_id` 属于该分组当前成员的全部项目链接；同一成员以不同 `--as` 链接多次时均属于该范围。
5. `.gitignore` 托管块固定为 `[".claude/", ".agents/"]`；更新已有托管块，保留块外用户内容。`AGENTS.md` 保持项目共享指令文件。
6. 在模块 02、05、06 明确来源字段、旧字段读取、解绑范围和 `.gitignore` 规则；写明 rename 只更新全局注册名称，既有项目通过不可变 `item_id` 继续识别，不反向重命名项目路径。

## 验收

- 旧 Harness 清单、旧 registry 清单与混合清单读取后得到正确来源，重新保存仍可读取且不输出 scope。
- 临时项目执行 `unlink --all` 后，`CLAUDE.md`、`.claude/skills` 及其 Harness 记录保留；item 链接按选中范围解绑。
- 分组成员变更后按当前成员解绑；分组外记录和 Harness 记录保留；rename 不改变内部 ID，不妨碍按 ID 及分组成员匹配。
- `.gitignore` 托管块与新规则一致，块外内容不变；重复 init 幂等。
- 使用 mock Provider、临时 Manifest 和临时数据库验证，运行 `cargo fmt --check`、`cargo test --locked`。

## 执行记录

2026-10-31：已将原分支实现与专项回归适配到当前 Store、i64 时间戳及独立测试文件；本轮集成实跑与完成状态统一记录于[阶段一二收口](阶段一二收口.md)，历史独立分支验收不替代 main 集成验收。
