# 17 · ProjectCtx 与命令层组装

返回 [TODO](README.md)。前置：[06](06-store.md)、[12](12-init-harness.md)、[16](16-link-plan.md)。依据：第七章第 1 项、第八章阶段三第 2 项。条件文档：[06](../modules/06-cli-command-surface.md)、[07](../modules/07-code-architecture-verification.md)。

## 本步结果与边界

只改变项目操作的依赖传入和入口结构，消除重载套娃。复用已完成的 LinkPlan、Store 与路径逻辑；Doctor 独立模块拆分在 18，CLI 命令变体归一在 20。

## 代码入口

- [src/core/project_links.rs · `pub struct ProjectCtx`](../../src/core/project_links.rs#L125)。
- [src/core/project_links.rs · `pub fn link_item`](../../src/core/project_links.rs#L308)。
- [src/commands/mod.rs · `pub fn run`](../../src/commands/mod.rs#L195)。
- [src/commands/mod.rs · `fn run_link`](../../src/commands/mod.rs#L390)。
- [src/commands/mod.rs · `fn run_unlink`](../../src/commands/mod.rs#L408)。

## 修改清单

1. 在 project_links 按裁决定义：

   ```rust
   pub struct ProjectCtx<'a> {
       pub root: &'a Path,
       pub provider: &'a mut dyn SymlinkProvider,
   }
   ```

2. 每个项目操作仅保留一个核心入口；例如 `link_item(ctx: &mut ProjectCtx, store: &Store, req: &LinkItemRequest, opts: LinkOptions)`。group link/unlink、普通 unlink、clean、status 按实际需要接收 ctx、Store 和请求，不再增加 `_with_provider_and_db_with_options` 等变体。
3. commands 层使用 paths 定位 root、构造默认 Provider 和 ProjectCtx，按用例打开一次 Store 并传给 core。仅消费 Manifest 的操作不因此新增全局库依赖。
4. 删除 core 中只为读取 current_dir、解析全局路径、选 Provider、再转发的所有项目操作包装；已有 Registry/Harness Store API 不在每次调用时重新连接。
5. tests 直接构造 TestDir + MockSymlinkProvider + 临时 Store + ProjectCtx；不为测试保留生产套壳。所有业务执行继续调用 16 的唯一 LinkPlan 流程。
6. 同步模块 07 与架构入口的命令组装职责，维持 `cli -> commands -> core`；commands 不出现 SQL、Manifest 文件读写或平台 symlink API。

## 验收

- 每个项目操作只有一个核心用例入口，测试与生产消费同一入口。
- 一次 group link/unlink 复用同一 Store；普通 status/clean 等原本不依赖数据库的项目操作继续可用。
- 路径上查、init 当前目录、dry-run、来源隔离和 Plan 执行回归保持通过。
- 运行 `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`、`cargo run --locked -- --help`。

## 执行记录

实现、验证结果和未解决问题统一记录于[阶段三进度 · 17](阶段三进度.md#步骤-17)。
