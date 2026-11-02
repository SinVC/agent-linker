# 14 · Manifest 改用 serde + toml 与后端审计兼容

返回 [TODO](README.md)。前置：[02](02-manifest-origin.md)、[03](03-manifest-save.md)、[09](09-schema-v3.md)、[13](13-distribution.md)。依据：第六章第 1、2 项、第八章阶段三第 1 项。条件文档：[02](../modules/02-project-init-manifest.md)、[07](../modules/07-code-architecture-verification.md)。

## 本步结果与边界

仅替换清单序列化实现，并让历史后端审计值不阻塞 unlink/clean。沿用已经完成的路径安全、origin、scope 兼容、时间与原子保存契约，不另立 Manifest 格式体系。

## 代码入口

- [Cargo.toml · `[dependencies]`](../../Cargo.toml#L12)。
- [src/core/manifest.rs · `pub struct LinkRecord`](../../src/core/manifest.rs#L42)。
- [src/core/manifest.rs · `fn serialize_manifest`](../../src/core/manifest.rs#L508)。
- [src/core/manifest.rs · `struct StoredLinkRecord`](../../src/core/manifest.rs#L520)。
- [src/core/manifest.rs · `fn deserialize_timestamp`](../../src/core/manifest.rs#L571)。

## 修改清单

1. 引入 `toml` crate 并更新 lockfile，通过 serde Serialize/Deserialize 完成清单读写，删除手写行解析、key/value 拆分、字符串转义/反转义等被替代代码。
2. serde 数据读取保留必要旧字段兼容：旧 framework_name 的 Harness/item 来源映射、旧 scope 的接受、已有路径/ID/时间的读取。02 已确定的新记录写出规则保持不变，validate 继续承担业务合法性检查。
3. provider_backend 作为审计字符串宽容解析，例如以 `Unknown(String)` 保留未知原值；包括历史 `external-ln` 在内的旧名称不能阻止状态读取、clean 或 unlink。真实 Provider 选择仍只使用实现的后端，不把未知审计值当成可执行后端。
4. 调整因审计值不再是固定 Copy 枚举而必须修改的字段消费点；保留 status/report 对后端字符串的展示。
5. 路径保持当前契约：绝对 source 的本机分隔符往返正确，项目相对 link_path 采用一致的相对表示，新计算的链接路径剥离 `.` 组件。继续通过 01 安全入口与 03 原子保存，不在此步重新实现文件写入。
6. 同步模块 02 的序列化与历史后端兼容说明。

## 验收

- 新旧清单往返、Windows 路径、引号/反斜杠/换行等字符串通过 TOML 正确解析；缺字段、损坏结构仍失败。
- 旧 origin 推导、scope、旧时间值、重复 link_path 和路径越界用例继续通过。
- 临时清单含未知 provider_backend 或历史 external-ln 时，status 可读，mock unlink/clean 可执行且不会因后端名称拒绝。
- 运行 `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`。

## 执行记录

实现、验证结果和未解决问题统一记录于[阶段三进度 · 14](阶段三进度.md#步骤-14)。
