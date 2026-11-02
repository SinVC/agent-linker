# 20 · CLI 公共结构与命令归一

返回 [TODO](README.md)。前置：[08](08-remove-config.md)、[17](17-project-context.md)、[19](19-errors.md)。依据：第六章第 4 项 clap 重复校验、第七章第 3 项。条件文档：[06](../modules/06-cli-command-surface.md)、[07](../modules/07-code-architecture-verification.md)。

## 本步结果与边界

消除 CLI 和 commands 的重复结构与双线分发，保持现有命令形状和已经完成的业务语义。只清理解析、转换、分发及其直接测试。

## 代码入口

- [src/cli.rs · `enum CliItemCommand`](../../src/cli.rs#L133)。
- [src/cli.rs · `struct CliResourceAddArgs`](../../src/cli.rs#L155)。
- [src/cli.rs · `fn group_command`](../../src/cli.rs#L401)。
- [src/cli.rs · `struct CliLinkArgs`](../../src/cli.rs#L227)。
- [src/commands/mod.rs · `pub enum LinkCommand`](../../src/commands/mod.rs#L105)。

## 修改清单

1. 合并 Skill/Resource 的公共子命令结构和公共转换代码；保留 Resource add 的必填 target-dir、两类 source 约束等实际差异，不引入新命令。
2. 在解析后将 `group link <g>` 与 `link --group <g>` 归一为同一个 Command 变体；`group unlink <g>` 与 `unlink --group <g>` 同样归一。commands 删除重复的 Group Link/Unlink 分发路径，继续调用相同 core 用例。
3. 删除与已有 clap `conflicts_with`、required 等定义完全重复的手工互斥/缺参判断；业务差异校验保留在其负责层，不能因精简而接受原先非法的参数组合。
4. 调整 parser/help/report 测试，保留 global quiet/verbose、dry-run、force、status JSON 和原命令参数含义。
5. 同步模块 06、07 的解析归一职责；如用户命令示例没有变化，不重写示例。

## 验收

- 两组 group/link 别名形状分别解析为相同 Command；dry-run/force 参数传递一致。
- Skill/Resource 公共操作行为一致，Resource add 缺 target-dir 仍拒绝。
- link 缺目标、多个互斥目标、group 配 item 专属参数、unlink 目标互斥和 clean 模式互斥仍由解析契约正确拒绝。
- status JSON 字段与 escaping、help、退出码保持通过。运行 `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`、`cargo run --locked -- --help`。

## 执行记录

实现、验证结果和未解决问题统一记录于[阶段三进度 · 20](阶段三进度.md#步骤-20)。
