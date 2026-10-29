# 04 · Symlink 原子替换与断链 force

返回 [TODO](README.md)。前置：[01](01-paths.md)、[03](03-manifest-save.md)。依据：第四章 P0-4、第八章阶段一第 3 项。条件文档：[01](../modules/01-symlink-core.md)、[06](../modules/06-cli-command-surface.md)。

## 本步结果与边界

将已有 symlink 的替换下沉至 Provider，修复新建失败丢旧链以及有效新 source 不能替换断链的问题。共享纯判定函数和选项简化留给 15。

## 代码入口

- [SymlinkProvider 与 ensure_symlink](../../src/core/symlink.rs#L274)。
- [Std Provider](../../src/core/symlink.rs#L379)、[Windows Broker Provider](../../src/core/symlink.rs#L413)、[Mock Provider](../../src/core/symlink.rs#L608)。
- [validate_source](../../src/core/symlink.rs#L745)、[平台创建](../../src/core/symlink.rs#L809)、[force 测试](../../src/core/symlink.rs#L1254)。
- [现有 dry-run 判定](../../src/core/project_links.rs#L987)、[项目 force 回归](../../src/core/project_links.rs#L1386)。

## 修改清单

1. Provider 增加替换已有 symlink 的能力。`ensure_symlink` 不再先 `remove_symlink` 再 `create_symlink`。
2. Windows Broker 替换请求使用 `replace_existing_symlink = true`；普通创建仍为 false。Unix Std 在同目录创建临时 symlink 后，用 `fs::rename` 原子覆盖。
3. 新 source 必须存在且 kind 合法。目标为 `WrongSymlinkTarget` 或 `BrokenSymlink` 时，`--force` 允许替换；新 source 缺失仍报错，真实文件或目录仍拒绝替换。
4. Mock 实现对应替换语义，能够注入替换失败并保留旧链；现有项目 dry-run 判定同步 force 行为，待 15 删除重复判定实现。
5. 同步模块 01、06 的 Provider 接口与 force 契约，不引入额外平台后端或自动 fallback。

## 验收

- 错误目标与断链分别覆盖无 force 拒绝、有 force 成功；新 source 缺失拒绝。
- 故障注入证明替换失败仍保留旧 symlink，真实文件、目录均不受 force 影响。
- 项目实际执行与 dry-run 对上述场景给出相同判定，dry-run 不修改链接或 Manifest。
- Windows 验证 Broker 原生替换参数与错误码，Unix 验证实际同目录 rename 覆盖；平台未运行项记录，不以 mock 代替实际平台结论。
- 运行 `cargo fmt --check`、`cargo test --locked`。

## 执行记录

未执行。完成时填写修改文件、故障注入和平台结果，再勾选 TODO 04。
