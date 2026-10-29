# 02 · 进度与证据

状态：待执行。组合可行性评估已完成；当前授权为 02→03 组合轮次，02 在前，03 依赖 02；同一执行聊天串行完成，分别独立验收，完成后停止，04 未授权。

- 静态计划：[02-manifest-origin.md](../../重构计划/02-manifest-origin.md)。按统筹提供的计划 README 精确行引用进入本步，自行读取所列条件文档；组合执行聊天仅接收 02/03 两份进度，无需读取范围外步骤进度。
- 执行聊天：待统筹登记（新独立聊天，`gpt-6.1-sol` / `high`）。
- 独立验收聊天：待统筹登记。
- 02/03 只读评估结论：统筹已确认可在同一执行聊天先 02 后 03，分别独立验收；评估聊天及报告位置待统筹登记。
- 受验版本与实跑证据：待执行、验收后登记；当前无 PASS 结论。
- 阻塞：当前无已知评估阻塞；执行前须按实际差异保全以下基线。
- 实现/结案提交：无。
- 停止条件：授权阶段完成后停止；不得因本记录自动推进 04。

写回规则：下钻计划中的旧执行记录/勾选 TODO 提示由本隔离规则替代。动态状态、实跑证据与验收结论仅写本 `02-progress.md`；README 仅静态导航、不勾选，不回写其他步骤。组合轮次的 03 自身记录独立写其进度文件。

## 本轮实施起点与基线保全

代码尚未启动。先对本次编排文档迁移独立审验，由 terra 本地提交；随后从该固定已提交 HEAD 创建隔离 worktree，实施、分别验收并提交 02/03。主树 readonly/backup 等既有未提交修改仅保全，不移入候选，不 stash/reset/整文件覆盖；前后核对主树 HEAD/index 与全部 tracked/untracked 哈希快照。成果留在隔离分支/worktree，不为结案合回脏主树；将来若需回写，另做三方合并与组合验证。

## 编排整理前工作树基线

2026-10-29，以下是本次编排文档改动前的只读 Git 状态快照；不代表这些修改属于 02，不允许覆盖或混入提交。已有 01 下钻记录差异亦保留。

HEAD：`c455f1efd9171eac142ce03f07c70cef42dee7b5`。

```text
 M CLI_REFERENCE.md
 M README.md
 M RELEASE_CHECKLIST.md
 M dev-docs/architecture.md
 M dev-docs/modules/04-global-store-registry.md
 M dev-docs/release-distribution.md
 M dev-docs/重构计划/01-paths.md
 M packaging/unix/install.sh
 M packaging/windows/install-broker.ps1
 M packaging/windows/install.ps1
 M packaging/windows/package-windows.ps1
 M packaging/windows/uninstall.ps1
 M src/core/db.rs
 M src/core/harness.rs
 M src/core/project_links.rs
 M src/core/registry.rs
?? CLI_REFERENCE_zh.md
?? packaging/unix/test-install.sh
?? packaging/windows/maintain-install.ps1
?? packaging/windows/test-install.ps1
```
