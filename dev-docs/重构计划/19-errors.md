# 19 · 结构化错误与 verbose 错误链

返回 [TODO](README.md)。前置：[09](09-schema-v3.md)、[11](11-db-check.md)、[18](18-doctor.md)。依据：第六章第 1 项 NotImplemented、第七章第 4 项、第八章阶段三第 3 项。条件文档：[01](../modules/01-symlink-core.md)、[06](../modules/06-cli-command-surface.md)、[07](../modules/07-code-architecture-verification.md)。

## 本步结果与边界

让 core 错误保留类型、底层原因和路径，并使全局 verbose 真正展示错误源链。只调整错误模型及必要传播/输出，不添加重试、降级或新的恢复行为。

## 代码入口

- [src/core/error.rs · `pub enum Error`](../../src/core/error.rs#L10)。
- [src/core/store.rs · `impl From<rusqlite::Error>`](../../src/core/store.rs#L524)。
- [src/core/symlink.rs · `pub struct SymlinkError`](../../src/core/symlink.rs#L127)。
- [src/core/symlink.rs · `fn broker_error_to_symlink_error`](../../src/core/symlink.rs#L506)。
- [src/cli.rs · `fn print_error`](../../src/cli.rs#L927)。

## 修改清单

1. 增加/拆分 `Error::NotFound`、`Error::Conflict`、`Error::DatabaseBusy`、`Error::PermissionDenied`，并将 IO 错误表示为 `Error::Io { path: Option<PathBuf>, source: io::Error }`。
2. 数据库、IO 与 Symlink 错误传播保留底层 source，使用 `std::error::Error::source` 形成可遍历链；调整现有 From、Display、derive 和消费点，不再把底层错误一律 `.to_string()` 抹平。Manifest 加载/保存的业务校验和非普通文件错误保留清单路径，manifest/paths 中已知目标的 IO 错误附带涉及路径，不改变 Store 结构。
3. 09 的唯一约束错误映射到 Conflict；11 已区分的锁竞争、权限不足继续保留分类，不退回仅含文案的通用 Database 错误。
4. `print_error` 消费全局输出选项：默认保持简洁，verbose 输出错误源链、相关路径、系统错误码与 Broker 稳定错误码，quiet 保留必要错误。首层 caused by 与顶层文案完全相同时仅省略该重复行，保留后续原因与平台码；默认 SymlinkError 格式保持不变。
5. 删除 `Error::NotImplemented`、构造方法和无消费者分支，更新断言及调用点。
6. 同步模块 01、06、07 的错误传播和 verbose 契约，不新增错误输出模式或协议。

## 验收

- 临时路径 IO 失败保留 io::Error 和涉及路径，数据库唯一/忙/权限错误分类正确。
- verbose 能看到原始错误链与平台/Broker 代码，普通输出仍为简洁错误；quiet 不吞必要错误。
- 现有失败退出码、force 保旧链、事务回滚及 Manifest 损坏测试继续通过。
- 运行 `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`。

## 执行记录

实现、验证结果和未解决问题统一记录于[阶段三进度 · 19](阶段三进度.md#步骤-19)。
