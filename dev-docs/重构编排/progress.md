# 重构统筹恢复索引

仅供统筹；执行与验收会话只接收当前授权步骤的 `steps/NN-progress.md`，无需读取本文件或范围外步骤进度。

- 统筹聊天：`01a18414-be15-7533-8e09-04f9b9971ffc`。
- 恢复顺序：[策略](strategy.md) → 本索引 → 当前步骤文件；根据当前步骤证据继续，不构建其他步骤实现上下文。
- 当前授权：用户已授权继续下一步 04，当前为启动准备；02→03 两子项分别独立验收 PASS 且已本地提交，完整保留。05 未授权、未启动。
- 自动跟进：当前 `agent-linker-04` 已创建，每 5 分钟跟进仅 04；无实质变化时安静，全部收尾后删除，05 不启动。既有 `agent-linker-02-03` 与旧实例 `agent-linker` 均已删除。
- 编排元数据：独立审验 PASS，terra 已本地提交 `b1781bcee041296ab28398ff9e96bb1b9010c46e`（仅 6 文件，非目标哈希无变化，index 空）。
- 交付：执行聊天 `01a1870a-76e2-70f1-8fe6-856b528d5949`（local，`gpt-6.1-sol` / `high`）完成实现；terra 子代理 `/root/commit_steps02_03` 已本地提交 `f583cc8c3daded38bd47f8b003e7a284ee60bb45`，parent `b1781bcee041296ab28398ff9e96bb1b9010c46e`，10 路径，提交时隔离树 clean，未 push。
- 独立验收：聊天 `01a18710-e2f4-79f0-8096-c7209d319207` 已交付 02 PASS、03 PASS，无未解缺陷或阻塞。实现提交 tree `eadfbdf080b65d61c0e40a452a3fda01920ad91f` 与受验树一致；Windows 已实测，03 的 Unix 路径未实跑，限制及详细证据留各自步骤记录。
- 冻结证据：执行聊天 `2026/10/29` visualization 的 `preservation/final-candidate.patch`、`preservation/final-files.json`。
- 隔离 worktree：`C:/Users/SinVC.Windy/.codex/worktrees/manifest-origin-save/agent_linker`；固定起点 `b1781bcee041296ab28398ff9e96bb1b9010c46e`。分支 `codex/manifest-origin-save-02-03` 已创建；成果不合回主树。
- 保全基线：执行会话 visualization 下 `preservation/main-before.json` 已保存主树 HEAD/index SHA256 及 87 个 tracked/untracked 路径快照。
- 主树保全：02/03 代码未合回；原 HEAD/index 及既有基线保全。本轮协调范围仅计划 README、策略、本全局索引及新增 04 进度；01–03 现有步骤进度与原下钻正文不改。准备前快照位于统筹 visualization 的 `step04-preparation/main-before.json`（87 个 tracked/untracked 路径及 HEAD/index SHA256）。
- 04 代码基线：Git 分支与旧隔离 worktree 实际 HEAD 均确认为 `0cea138c7cd8dca736d1b16e4055beb9d731d5e2`，分支 `codex/manifest-origin-save-02-03`；parent 为实现提交 `f583cc8c3daded38bd47f8b003e7a284ee60bb45`，其后仅 02/03 结案文档提交。04 应从该固定 HEAD 新建隔离分支/worktree，保留旧交付成果。主树 `main` / `9415577f5d647b7de989d0ab9253089ac76ba687` 仅含协调元数据、不含 02/03 新代码，禁止用作 04 代码起点。

| 步骤 | 当前状态 | 恢复入口 |
| --- | --- | --- |
| 01 | 已完成；实现和结案均已本地提交 | [01 进度](steps/01-progress.md) |
| 02 | 完成；独立 PASS，实现已本地提交 | [02 进度](steps/02-progress.md) |
| 03 | 完成；独立 PASS，实现已本地提交 | [03 进度](steps/03-progress.md) |
| 04 | 已授权；启动准备，执行与独立验收待分配 | [04 进度](steps/04-progress.md) |

全局索引只维护当前授权、状态和步骤入口；会话 ID、差异、证据、阻塞及提交明细写在对应步骤文件。后续步骤获授权时才新增条目。
