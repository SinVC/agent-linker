# 15 · 共享 Symlink 判定与选项收敛

返回 [TODO](README.md)。前置：[04](04-symlink-replace.md)、[14](14-manifest-toml.md)。依据：第六章第 4 项 CreateSymlinkOptions、第七章第 2 项。条件文档：[01](../modules/01-symlink-core.md)、[07](../modules/07-code-architecture-verification.md)。

## 本步结果与边界

让创建、删除、dry-run 与实际执行共用 Symlink Core 的纯判定规则。只收敛判定与选项传递，不在本步改写整个项目批量操作，LinkPlan 在 16 实现。

## 代码入口

- [ensure_symlink](../../src/core/symlink.rs#L286)、[CreateSymlinkOptions](../../src/core/symlink.rs#L241)、[LinkStatus](../../src/core/symlink.rs#L78)。
- [plan_symlink](../../src/core/project_links.rs#L987)、[preflight_unlink](../../src/core/project_links.rs#L847)、[planned_remove_outcome](../../src/core/project_links.rs#L896)。
- [LinkOptions](../../src/core/project_links.rs#L98)、[create_symlink_options](../../src/core/project_links.rs#L1046)。
- [init 的创建选项](../../src/core/manifest.rs#L168)、[Provider 删除入口](../../src/core/symlink.rs#L391)。

## 修改清单

1. Symlink Core 提供无副作用的 `evaluate_create` / `evaluate_remove` 纯判定函数，统一状态到操作结果/错误的映射。输入为已经取得的状态与必要选项，不创建链接、删除链接、建目录或保存 Manifest。
2. 创建判定保留 04 的最终语义：正确链幂等；wrong/broken 在新 source 有效且 force 时可替换；真实文件/目录拒绝；source 缺失不能伪装成功。
3. 删除判定统一处理 missing、正确/错误/断链、真实文件/目录与不支持类型，保留 clean 的选择条件与 unlink 的来源范围。
4. 项目预判和实际创建/删除都调用同一套纯规则，删除 `plan_symlink`、`preflight_unlink`、`planned_remove_outcome` 中重复的状态匹配实现，不能再维护另一套 LinkStatus → SymlinkError 映射。
5. 简化 `CreateSymlinkOptions` 为无多余构造/转换器的单字段结构，或直接内聚现有 LinkOptions；选择不造成 core 反向依赖的最小实现。删除 `new/force_wrong_symlink/create_symlink_options` 的冗余转换，更新 init、commands 和测试所需调用点。
6. 同步模块 01 的共享判定职责和选项契约，不新增状态或冲突策略。

## 验收

- 对既有全部 LinkStatus 验证纯创建/删除规则，覆盖 force 与 source 有效性组合。
- 同一输入在 dry-run 和实际执行前判定一致；纯函数不调用 Provider 修改方法。
- 04 的替换失败保旧链、真实文件/目录拒绝测试继续通过。
- 运行 `cargo fmt --check`、`cargo test --locked`。

## 执行记录

未执行。完成时填写共享规则位置、删除的重复分支和验证结果，再勾选 TODO 15。
