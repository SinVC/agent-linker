# 03 · Manifest 原子保存

返回 [TODO](README.md)。前置：[01](01-paths.md)、[02](02-manifest-origin.md)。依据：第四章 P0-5 第 4 个问题及整改第 2 项、第八章阶段一第 2 项。条件文档：[02](../modules/02-project-init-manifest.md)。

## 本步结果与边界

替换直接覆盖清单的写法，避免原文件写入过程中被截断。只修改保存过程，批量操作只保存一次由 16 完成，不在此步建立 LinkPlan 或改写序列化器。

## 代码入口

- [save_manifest](../../src/core/manifest.rs#L216)、[load_manifest](../../src/core/manifest.rs#L196)。
- [init 保存点](../../src/core/manifest.rs#L186)、[link 保存点](../../src/core/project_links.rs#L270)、[unlink_selected_indexes](../../src/core/project_links.rs#L804)、[clean](../../src/core/project_links.rs#L503)。
- [现有清单测试](../../src/core/manifest.rs#L674)、[损坏清单不覆盖测试](../../src/core/manifest.rs#L721)。

## 修改清单

1. 保留 `manifest.validate()`；目标清单、父目录与临时文件路径均复用 01 的安全入口。
2. 将序列化结果写入同目录 `.agents/.links.toml.tmp.<pid>`，写入成功后通过 `fs::rename` 替换 `.agents/links.toml`。
3. 更新保存调用点所需错误传递；不通过先删原文件再 rename 的方式替代原子覆盖。
4. 保持损坏 Manifest 失败且不覆盖的既有语义，在模块 02 记录原子保存契约。不增加裁决之外的日志、备份或持久化协议。

## 验收

- 临时目录测试新建清单、替换已有清单以及完整往返读取。
- 覆盖写临时文件失败和 rename 失败，原清单内容保持完整。
- 真实 Windows 文件系统验证已有目标的替换行为；Unix 在对应平台运行同一保存测试。
- 运行 `cargo fmt --check`、`cargo test --locked`，未运行的平台如实记录。

## 执行记录

未执行。完成时填写修改文件、失败场景与平台结果，再勾选 TODO 03。
