# 21 · 测试、平台代码隔离与剩余死代码删除

返回 [TODO](README.md)。前置：[14](14-manifest-toml.md)、[18](18-doctor.md)、[19](19-errors.md)、[20](20-cli.md)。依据：第六章第 1、3 项、第七章可见性约束、第八章阶段三第 3 项。条件文档：[01](../modules/01-symlink-core.md)、[07](../modules/07-code-architecture-verification.md)。

## 本步结果与边界

仅执行裁决点名的条件编译、死代码与分层可见性清理。前序已删除的字段/错误/转换器只核对，不再安排重复重构，也不进行清单之外的泛化清理。

## 代码入口

- [src/core/symlink.rs · `pub enum MockEntry`](../../src/core/symlink.rs#L537)。
- [src/core/symlink.rs · `pub struct WindowsBrokerSymlinkProvider`](../../src/core/symlink.rs#L432)。
- [src/core/symlink.rs · `pub enum SymlinkBackend`](../../src/core/symlink.rs#L62)。
- [src/core/store.rs · `pub(super) fn connection(`](../../src/core/store.rs#L84)。
- [src/core/mod.rs · `pub mod doctor;`](../../src/core/mod.rs#L1)。

## 修改清单

1. MockSymlinkProvider、MockEntry 及其专用实现/辅助结构统一 `#[cfg(test)]`，生产构建不再编译这些测试代码；保留单元测试直接使用能力。
2. WindowsBrokerSymlinkProvider 的类型和整体实现统一 `#[cfg(windows)]`；删除非 Windows 平台四个仅返回“不支持”的桩方法，更新跨平台 imports 和测试条件编译。
3. 删除运行时 `SymlinkBackend::ExternalLn` 和 `LinkStatus::is_existing_real_path`。历史 external-ln 审计字符串继续由 14 的宽容读取保留，不恢复为执行后端。
4. 删除 commands 末尾 `include_str!("mod.rs")` 搜源码的架构自检测试，以 Rust 可见性约束入口：用例 API 按需收敛为 `pub(crate)`，底层连接和内部实现只在所需 core 范围可见；commands 继续只能经 core API 访问存储、清单和 Provider。
5. 核对前序已删除内容：config、SourceType/SourceOwnership、死列、scope、NotImplemented、bool 转换、冗余 CreateSymlinkOptions 转换与旧重载均无运行时消费者残留。删除无生产消费者的 Store::enabled_harness_mappings，相关测试继续通过现有 Harness 查询验证启用状态与映射。
6. 同步模块 01、07，移除外部 ln 执行后端、预留 source ownership 和旧测试架构描述，不增加新的扩展机制。

## 验收

- Windows 构建/测试通过，非测试构建不依赖 Mock；Unix 对应构建/测试不要求 Windows 类型或桩实现。
- unknown/external-ln 历史清单的 status/unlink/clean 回归继续通过。
- 可见性收敛后 `cli -> commands -> core` 依赖正确，commands 不直接访问连接、清单文件或平台 API；不再以源码字符串测试替代行为验证。
- 运行 `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`，记录 Windows/Unix 实际执行情况。

## 执行记录

实现、验证结果和未解决问题统一记录于[阶段三进度 · 21](阶段三进度.md#步骤-21)。
