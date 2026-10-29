# 12 · init 消费已启用 Harness 映射

返回 [TODO](README.md)。前置：[01](01-paths.md)、[02](02-manifest-origin.md)、[06](06-store.md)、[09](09-schema-v3.md)、[11](11-db-check.md)。依据：第七章 Manifest/Harness 职责、第八章阶段二第 4 项。条件文档：[02](../modules/02-project-init-manifest.md)、[03](../modules/03-harness-adapter.md)、[04](../modules/04-global-store-registry.md)、[06](../modules/06-cli-command-surface.md)。

## 本步结果与边界

闭合 Harness 设置到 init 的最小使用链路。06 已提供唯一内置定义和不覆盖用户状态的 Seed，本步只接入消费者，不增加 Harness 命令或项目配置。

## 代码入口

- [init_current_project/init_project_with_provider](../../src/core/manifest.rs#L131)、[ensure_harness_parent_dirs](../../src/core/manifest.rs#L241)、[record_from_mapping](../../src/core/manifest.rs#L409)。
- [内置映射](../../src/core/harness.rs#L50)，06 后为统一 `builtin_harnesses()` 数据。
- [enabled_harness_mappings](../../src/core/harness.rs#L208)、[HarnessMapping](../../src/core/harness.rs#L28)、[StoredHarnessMapping](../../src/core/harness.rs#L103)。
- [Command::Init 编排](../../src/commands/mod.rs#L240)、[init 测试](../../src/core/manifest.rs#L674)。

## 修改清单

1. 全局库已初始化时，init 通过只读 Store 查询已启用 Harness 及其 mappings，替代直接固定遍历两条 Claude mapping。
2. 全局库未初始化时使用同一内置定义中的 Claude 默认映射，纯本地 init 不要求先建库；不因 init 回退而隐式执行迁移或 Seed。旧 schema 等普通存储错误沿用 Store 的显式提示，不将所有错误吞并为回退。
3. 动态 mapping 复用 01 的安全路径解析、Symlink Core、02 的 Harness 来源记录以及 03 的保存入口；source/link kind 与 required 保留已有含义，不增加自动创建任意 mapping source 的能力。
4. init 仍只处理 current_dir；创建/确认 `AGENTS.md`、`.agents/`、`.agents/skills/`，重复执行保持幂等。Harness enable/disable、mapping 增删只改变全局设置，不自动反向改动已存在项目。
5. 同步模块 02、03、06 和架构入口中“init 固定内置映射、尚未消费全局设置”的旧描述，改成上述已实现契约。

## 验收

- 未初始化全局库时，临时项目 init 使用 Claude 默认映射且不创建全局数据目录/数据库。
- v3 临时库中关闭 Claude 后，新临时项目不创建 Claude 兼容链接；启用后创建；自定义 mapping 能被新 init 使用。
- 初始化库已存在但无启用 Harness 时不强制恢复 Claude；只读查询不修改 updated_at 或 Seed。
- 使用临时 Manifest、数据库和 mock Provider 验证 Harness 来源、幂等、真实路径冲突及受控路径检查。
- 运行 `cargo fmt --check`、`cargo test --locked`。

## 执行记录

未执行。完成时填写回退与数据库消费场景、修改文件和验证结果，再勾选 TODO 12。
