# 19 · 结构化错误与 verbose 错误链

返回 [TODO](README.md)。前置：[09](09-schema-v3.md)、[11](11-db-check.md)、[18](18-doctor.md)。依据：第六章第 1 项 NotImplemented、第七章第 4 项、第八章阶段三第 3 项。条件文档：[01](../modules/01-symlink-core.md)、[06](../modules/06-cli-command-surface.md)、[07](../modules/07-code-architecture-verification.md)。

## 本步结果与边界

让 core 错误保留类型、底层原因和路径，并使全局 verbose 真正展示错误源链。只调整错误模型及必要传播/输出，不添加重试、降级或新的恢复行为。

## 代码入口

- [Error 及转换](../../src/core/error.rs#L1)、原 [rusqlite 错误转换](../../src/core/store.rs)，06 后在 Store 相关代码。
- 原 [path_error](../../src/core/store.rs)、[source 校验错误](../../src/core/linkable.rs#L118)、[registry conversion_error](../../src/core/registry.rs#L627)。
- [SymlinkError](../../src/core/symlink.rs#L133)、[Broker 错误映射](../../src/core/symlink.rs#L511)。
- [CLI run_and_print](../../src/cli.rs#L301)、[print_error](../../src/cli.rs#L1036)。

## 修改清单

1. 增加/拆分 `Error::NotFound`、`Error::Conflict`、`Error::DatabaseBusy`、`Error::PermissionDenied`，并将 IO 错误表示为 `Error::Io { path: Option<PathBuf>, source: io::Error }`。
2. 数据库、IO 与 Symlink 错误传播保留底层 source，使用 `std::error::Error::source` 形成可遍历链；调整现有 From、Display、derive 和消费点，不再把底层错误一律 `.to_string()` 抹平。
3. 09 的唯一约束错误映射到 Conflict；11 已区分的锁竞争、权限不足继续保留分类，不退回仅含文案的通用 Database 错误。
4. `print_error` 消费全局输出选项：默认保持简洁，verbose 输出错误源链、相关路径、系统错误码与 Broker 稳定错误码，quiet 保留必要错误。
5. 删除 `Error::NotImplemented`、构造方法和无消费者分支，更新断言及调用点。
6. 同步模块 01、06、07 的错误传播和 verbose 契约，不新增错误输出模式或协议。

## 验收

- 临时路径 IO 失败保留 io::Error 和涉及路径，数据库唯一/忙/权限错误分类正确。
- verbose 能看到原始错误链与平台/Broker 代码，普通输出仍为简洁错误；quiet 不吞必要错误。
- 现有失败退出码、force 保旧链、事务回滚及 Manifest 损坏测试继续通过。
- 运行 `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`。

## 执行记录

未执行。完成时填写错误分类/传播改动、输出样例和验证结果，再勾选 TODO 19。
