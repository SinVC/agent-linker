# 重构统筹恢复索引

仅供统筹；执行与验收会话只接收当前授权步骤的 `steps/NN-progress.md`，无需读取本文件或范围外步骤进度。

- 统筹聊天：`01a18414-be15-7533-8e09-04f9b9971ffc`。
- 恢复顺序：[策略](strategy.md) → 本索引 → 当前步骤文件；根据当前步骤证据继续，不构建其他步骤实现上下文。
- 当前授权：用户仅授权继续 05「注册项标识符与 alias 查询空间隔离」；当前为启动准备，执行与独立验收会话待分配。01–04 已完成且 README 已勾选；06 未授权、不启动。此前 04 条目保留为历史记录，最新轮次授权与代码起点以本轮 05 记录为准。
- 自动跟进：本轮实例 `agent-linker-05` 已创建，每 5 分钟仅跟进 05；无实质变化时安静，仅完成、重要失败或需决策时通知，全部收尾后删除，暂停/停止条件优先。06 不启动；实例 ID 与范围仅记本索引。
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
- 05 固定代码起点：已只读核实 `codex/symlink-replace-04` = `514f9ab48bed043fe1376af5a95934b2fc90a8cf`（04 结案文档提交），直接 parent 为已审实现 `1a9a207e40b12a9219d32c1b49a5c4205ba594c2`，包含已交付 02–04。由新执行会话从该固定提交创建新隔离 worktree/分支；主树 `main` = `dfdd39f207709f10609c8818c972214a247edf02` 仅协调元数据及用户脏基线，不可作为 05 代码起点。仅导入本轮已审 `strategy.md` 与 `steps/05-progress.md` 两文件，不导入全局索引、README 或其他步骤进度；旧交付树及脏主树保留。
- 非阻塞观察：曾发生 global 精确旧可信字节回写，来源未证实；已取证并独立核实无用户内容或基线损害。当前由唯一维护者写前核验预期 SHA，不符即停写取证。

- 05 启动准备：本轮唯一全局维护者为统筹子代理 `/root/prepare_step05`；首次读取 SHA256 `57196B462FF819BF6E3729E8ABF53BFDB7B4BF1CC24AAEA13152DAA2F2E98FB5`，每次写前核验自身最新预期 SHA，不符即停写取证。执行与独立验收只用 [05 进度](steps/05-progress.md)，不读本索引或其他步骤进度。README 05 精确入口为 `dev-docs/重构计划/README.md:15`，05 下钻验收入口为 `dev-docs/重构计划/05-item-identity.md:24`。
- 05 保全恢复：主树 HEAD/index/status 与 88 个 tracked/untracked 路径的 SHA/链接元数据已存于统筹 visualization 的 `step05-recovery-20261030/`；当前仍为原 16 修改、4 未跟踪，`.tmp-cargo-migration` 当前不存在。未删除、回退或搬入旧修改。代码 ref 中 README 已包含 01–04 完成复选框与步骤隔离约定，无需另行导入。统筹明确授权本轮启动元数据仅三路径：本全局索引、清理旧轮动态文字的稳定 `strategy.md`、新建 `steps/05-progress.md`；新隔离树仅导入已审策略与当前步模板两文件，代码 ref 中旧策略由该已审副本替换。

| 步骤 | 当前状态 | 恢复入口 |
| --- | --- | --- |
| 01 | 已完成；实现和结案均已本地提交 | [01 进度](steps/01-progress.md) |
| 02 | 完成；独立 PASS，实现已本地提交 | [02 进度](steps/02-progress.md) |
| 03 | 完成；独立 PASS，实现已本地提交 | [03 进度](steps/03-progress.md) |
| 04 | 独立 PASS、实现已本地提交；仅结案协调文档收尾 | [04 进度](steps/04-progress.md) |
| 05 | 已授权；启动准备，执行与独立验收待分配 | [05 进度](steps/05-progress.md) |

全局索引只维护当前授权、状态和步骤入口；会话 ID、差异、证据、阻塞及提交明细写在对应步骤文件。后续步骤获授权时才新增条目。
