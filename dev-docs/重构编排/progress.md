# 重构统筹恢复索引

仅供统筹；执行与验收会话只接收当前授权步骤的 `steps/NN-progress.md`，无需读取本文件或范围外步骤进度。

- 统筹聊天：`01a18414-be15-7533-8e09-04f9b9971ffc`。
- 恢复顺序：[策略](strategy.md) → 本索引 → 当前步骤文件；根据当前步骤证据继续，不构建其他步骤实现上下文。
- 当前授权：04 独立验收 PASS、必要验证成功且实现已本地提交，README 04 已勾选；当前仅结案协调文档收尾。02→03 已交付成果完整保留。05 未授权、未启动。
- 自动跟进：当前 `agent-linker-04` 已创建，每 5 分钟仅跟进 04；无实质变化时安静，全部收尾后删除，05 不启动。既有 `agent-linker-02-03` 与旧实例 `agent-linker` 均已删除。
- 编排元数据：独立审验 PASS，terra 已本地提交 `b1781bcee041296ab28398ff9e96bb1b9010c46e`（仅 6 文件，非目标哈希无变化，index 空）。
- 交付：执行聊天 `01a1870a-76e2-70f1-8fe6-856b528d5949`（local，`gpt-6.1-sol` / `high`）完成实现；terra 子代理 `/root/commit_steps02_03` 已本地提交 `f583cc8c3daded38bd47f8b003e7a284ee60bb45`，parent `b1781bcee041296ab28398ff9e96bb1b9010c46e`，10 路径，提交时隔离树 clean，未 push。
- 独立验收：聊天 `01a18710-e2f4-79f0-8096-c7209d319207` 已交付 02 PASS、03 PASS，无未解缺陷或阻塞。实现提交 tree `eadfbdf080b65d61c0e40a452a3fda01920ad91f` 与受验树一致；Windows 已实测，03 的 Unix 路径未实跑，限制及详细证据留各自步骤记录。
- 冻结证据：执行聊天 `2026/10/29` visualization 的 `preservation/final-candidate.patch`、`preservation/final-files.json`。
- 隔离 worktree：`C:/Users/SinVC.Windy/.codex/worktrees/manifest-origin-save/agent_linker`；固定起点 `b1781bcee041296ab28398ff9e96bb1b9010c46e`。分支 `codex/manifest-origin-save-02-03` 已创建；成果不合回主树。
- 保全基线：执行会话 visualization 下 `preservation/main-before.json` 已保存主树 HEAD/index SHA256 及 87 个 tracked/untracked 路径快照。
- 主树保全：02/03/04 代码均未合回，用户原有基线保留；代码提交后已实际核验 87 个非 global 路径与原执行及恢复快照原字节一致，HEAD/index 保全。结案明确例外仅主树计划 README、当前 04 进度、本全局索引及隔离树 README/04 进度；策略和其他 step 不改。原快照：`C:/Users/SinVC.Windy/.codex/visualizations/2026/10/29/01a1873a-affc-7960-b0ee-94459e564a2a/step04/main-before.json`；恢复与结案前快照在统筹 visualization 的 `recover-step04-index/`。
- 04 启动元数据：四协调文件已独立审阅 PASS，由 terra 精确本地提交 `697317285f65a28dba276e75b58be3806dea5b56`；88 个文件哈希全匹配、提交后主 index 空、用户基线保留。
- 04 执行：新聊天 `01a1873a-affc-7960-b0ee-94459e564a2a`（local，`gpt-6.1-sol` / `high`）已完成实现并保持冻结；新隔离树 `C:/Users/SinVC.Windy/.codex/worktrees/symlink-replace-04/agent_linker`，实际分支 `codex/symlink-replace-04`。仅 README、策略、04 进度三个已审协调副本核验 SHA 后导入，旧交付树保留。
- 04 固定代码基线：`0cea138c7cd8dca736d1b16e4055beb9d731d5e2`，来源 `codex/manifest-origin-save-02-03`，包含已交付 02/03；parent 为实现提交 `f583cc8c3daded38bd47f8b003e7a284ee60bb45`。主树 `697317285f65a28dba276e75b58be3806dea5b56` 仅新增启动协调元数据、不含 02/03 新代码，不可用作 04 代码起点。
- 04 验证状态：独立验收 PASS、必要验证成功，无已确认阻塞。Windows 默认 Broker 与 Linux/WSL Std 均有独立真实平台结果；Windows 显式 Std fallback、macOS、Broker 内部部分完成故障未实跑，为非阻塞限制，详细证据留当前步骤记录。源码继续冻结，当前只收尾协调文档。
- 04 独立验收与提交：聊天 `01a18740-590d-7003-9c43-bb5e1b741fd0`（local，`gpt-6.1-sol` / `high`）已交付 PASS 及最终 8 文件指纹；指定 terra 已本地提交实现 `1a9a207e40b12a9219d32c1b49a5c4205ba594c2`，parent `0cea138c7cd8dca736d1b16e4055beb9d731d5e2`，tree `251c7b1cdc9db9094e3edb78ba3c1f670b78849a` 严格等于受验 8 blob 加 parent 构造树及真实 staged tree；提交后隔离树 clean，CLAUDE 保全，未 push。交付保留在 `codex/symlink-replace-04` / `C:/Users/SinVC.Windy/.codex/worktrees/symlink-replace-04/agent_linker`。
- 04 恢复维护：客户端当地日期 `2026-10-29`，统筹子代理 `/root/recover_step04_index` 接替已退出的 `/root/prepare_step04`，为当前全局索引与 README 唯一维护者，并按独立验收方最终 SHA 同步主树当前 04 进度；写前核验预期 SHA，不符即停写取证。结案文档待独立 metadata 审验及指定 terra 精确 doc-only 提交；自动跟进仍 active，仅用于本轮收尾，全部文档提交后由统筹删除。
- 后续代码起点：仅在用户另行授权 05 后，才从 `codex/symlink-replace-04` 最新已提交 HEAD 继续；不得从主树协调 metadata HEAD 开始，不合回脏主树代码。05 当前未授权、未启动。
- 非阻塞观察：曾发生 global 精确旧可信字节回写，来源未证实；已取证并独立核实无用户内容或基线损害。当前由唯一维护者写前核验预期 SHA，不符即停写取证。

| 步骤 | 当前状态 | 恢复入口 |
| --- | --- | --- |
| 01 | 已完成；实现和结案均已本地提交 | [01 进度](steps/01-progress.md) |
| 02 | 完成；独立 PASS，实现已本地提交 | [02 进度](steps/02-progress.md) |
| 03 | 完成；独立 PASS，实现已本地提交 | [03 进度](steps/03-progress.md) |
| 04 | 独立 PASS、实现已本地提交；仅结案协调文档收尾 | [04 进度](steps/04-progress.md) |

全局索引只维护当前授权、状态和步骤入口；会话 ID、差异、证据、阻塞及提交明细写在对应步骤文件。后续步骤获授权时才新增条目。
