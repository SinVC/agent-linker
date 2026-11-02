# 16 · 共享 LinkPlan 与一次执行提交

返回 [TODO](README.md)。前置：[01](01-paths.md)、[02](02-manifest-origin.md)、[03](03-manifest-save.md)、[09](09-schema-v3.md)、[15](15-symlink-rules.md)。依据：第四章 P0-5 文件系统部分、第七章 Project Links 及第 2 项、第八章阶段三第 2 项。条件文档：[01](../modules/01-symlink-core.md)、[02](../modules/02-project-init-manifest.md)、[05](../modules/05-linkable-item.md)、[06](../modules/06-cli-command-surface.md)、[07](../modules/07-code-architecture-verification.md)。

## 本步结果与边界

将 link/group link/unlink/clean 改为 Plan → Validate → Execute，提前发现整批冲突，并在执行后只原子保存一次清单。仍可沿用现有外层参数入口，17 再专门移除重载；不附加裁决之外的日志、重试或补偿协议。

## 代码入口

- [src/core/project_links.rs · `struct LinkPlan`](../../src/core/project_links.rs#L130)。
- [src/core/project_links.rs · `pub fn link_group`](../../src/core/project_links.rs#L327)。
- [src/core/project_links.rs · `fn execute_unlink`](../../src/core/project_links.rs#L405)。
- [src/core/project_links.rs · `pub fn clean`](../../src/core/project_links.rs#L426)。
- [src/core/manifest.rs · `pub fn save_manifest`](../../src/core/manifest.rs#L262)。

## 修改清单

1. 在 project_links 定义共享 LinkPlan，表达本次全部目标路径变更、缺失父目录和最终 Manifest 变化；只承载上述现有操作，不设计通用任务系统。
2. Plan 阶段只读取一次当前 Manifest，取得本次所需条目/成员，在内存计算完整变更集；group link 不再逐成员调用会立即落盘的单项函数。
3. Validate 阶段对整个变更集统一执行：受控路径解析、source/kind、已有 Manifest 路径占用、组内目标互冲、磁盘状态和 15 的共享规则。组内规划状态同时考虑尚未落盘的目标，不能只检查当前磁盘。同一 item 来源、item_id 和受控路径键命中旧记录时复用原 ID 与 created_at；其他来源或 item_id 占用同一路径仍报冲突。
4. `--dry-run` 在相同 Plan 完成校验后直接返回计划结果，不建立第二套模拟状态机，不创建父目录、不变更 symlink、不保存清单。
5. 校验全部通过后再按 Plan 创建缺失父目录、调用 Provider 执行链接变更，最后通过 03 保存一次最终 Manifest。unlink/clean 使用同一流程组织删除与 missing 记录移除，保留已有选择语义。
6. 报告由同一 Plan/执行结果生成，保留 created_dirs、创建/跳过/替换/删除结果以及 dry_run 标记；待建父目录和 created_dirs 按 managed_relative_path_key 去重，保留首次规划的路径拼写；必要调用点调整限于该执行流程。
7. 同步模块 02、06、07 的批量预检、dry-run 与一次保存契约。不得把一次原子 Manifest 保存描述为 SQLite 与文件系统的统一事务。

## 验收

- 以临时项目和 mock Provider 构造同一组两成员最终目标相同的场景；即使磁盘原先没有链接，dry-run 和实际执行都在 Validate 阶段报冲突。
- 后续成员 source 无效、真实路径占用、祖先 symlink 或 Manifest 路径冲突时，整批校验失败且不建父目录、不建/删链、不写清单。
- 成功 group link 只有一次清单提交；unlink/group unlink/clean 同样只提交一次。通过执行流程的观测验证，不靠检索源码字符串计数。
- 单项/批量及 dry-run/实际执行判定一致；Harness/item 范围、--as/target-dir、rename 旧 item_id 和未知审计后端的既有测试继续通过。
- force 执行中的 Provider 替换失败仍保旧链；不将预检测试当作任意执行故障可整体回滚的证明。
- 运行 `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`。

## 执行记录

实现、验证结果和未解决问题统一记录于[阶段三进度 · 16](阶段三进度.md#步骤-16)。
