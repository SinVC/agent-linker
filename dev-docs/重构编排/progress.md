# 重构统筹恢复索引

仅供统筹；执行与验收会话只接收当前授权步骤的 `steps/NN-progress.md`，无需读取本文件或范围外步骤进度。

- 统筹聊天：`01a18414-be15-7533-8e09-04f9b9971ffc`。
- 恢复顺序：[策略](strategy.md) → 本索引 → 当前步骤文件；根据当前步骤证据继续，不构建其他步骤实现上下文。
- 当前授权：用户仅授权继续 05「注册项标识符与 alias 查询空间隔离」；05 独立验收 PASS、必要验证成功且实现已本地提交，当前仅结案协调文档收尾。01–05 已完成且 README 已勾选；06 未授权、不启动。此前 04 条目保留为历史记录，最新轮次授权与代码起点以本轮 05 记录为准。
- 自动跟进：本轮实例 `agent-linker-05` 已创建，每 5 分钟仅跟进 05；无实质变化时安静，仅完成、重要失败或需决策时通知，当前仍 active，待全部结案文档提交后由统筹删除，暂停/停止条件优先。06 不启动；实例 ID 与范围仅记本索引。
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
- 05 固定代码起点：已只读核实 `codex/symlink-replace-04` = `514f9ab48bed043fe1376af5a95934b2fc90a8cf`（04 结案文档提交），直接 parent 为已审实现 `1a9a207e40b12a9219d32c1b49a5c4205ba594c2`，包含已交付 02–04。执行会话已从该固定提交创建新隔离 worktree/分支；主树 `main` 当前为启动协调文档提交 `51b9b472937609425ae935798c65034a24feb3bf`（parent `dfdd39f207709f10609c8818c972214a247edf02`），只含协调元数据提交及用户脏基线，不可作为 05 代码起点。仅导入本轮已审 `strategy.md` 与 `steps/05-progress.md` 两文件，不导入全局索引、README 或其他步骤进度；旧交付树及脏主树保留。
- 非阻塞观察：曾发生 global 精确旧可信字节回写，来源未证实；已取证并独立核实无用户内容或基线损害。当前由唯一维护者写前核验预期 SHA，不符即停写取证。

- 05 启动元数据：global、strategy、新 05-progress 三文件已独立审阅 PASS，指定 terra 已精确本地提交 `51b9b472937609425ae935798c65034a24feb3bf`；86 个非目标路径 SHA/链接元数据保全、提交后 index 空。该提交不是 05 代码基线。
- 05 执行：独立聊天 `01a19093-6f3b-7010-8857-5e731f7ca071`（local，`gpt-6.1-sol` / `high`）已完成实现，源码冻结；隔离树 `C:/Users/SinVC.Windy/.codex/worktrees/item-identity-05/agent_linker`；实际分支 `codex/item-identity-05`，起始 HEAD 固定为 `514f9ab48bed043fe1376af5a95934b2fc90a8cf`，路径/分支/HEAD 已与主仓 worktree 登记核对。执行会话已报告两文件导入时 SHA 核验正确：strategy `7FF982D6AFB9B7F659DA96B0D6408B27BB794F3A40860FD8A4AAF69BC8AFD4C6`、05-progress 模板 `D489E0E4CC5C7619F04AD61B05E33DF81702882F8C2E086B03259A67DA87103E`。执行方已报告 fmt、cargo test 通过，候选已冻结并停止源码及当前步骤进度写入，已交接独立验收；05 已独立 PASS、实现已本地提交，README 05 已勾选 `[x]`；当前只收尾协调文档，06 不启。详细进展只由隔离执行者维护当前步骤进度，不在本索引抄录实现细节或频繁登记 cursor。
- 05 正式冻结：执行方已停止源码及当前步骤进度写入，并将冻结候选交接给独立验收聊天 `01a19099-5991-73c1-8666-eb8cc7dd4b5c`；冻结 full candidate tree 为 `d51d8e386f84235d55a83f6770bc6f21c690b763`，已有可重建候选的 7 文件 patch/blob 证据。该 tree 包含冻结时的步骤进度，验收追加 progress 后会变化，不替代最终受验版本及独立结论。Broker 本步未触；88 个非 global 路径及主树 HEAD/index 保全、初始快照范围更正继续有效。此后仅最终独立判定、实际失败或需决策时更新全局索引。
- 05 独立验收：新聊天 `01a19099-5991-73c1-8666-eb8cc7dd4b5c`（local，`gpt-6.1-sol` / `high`）已完成正式实跑与独立判定并交付 PASS，指定唯一 `gpt-6.1-terra` / `high` 子代理提交后已独立复核；实现提交 tree 严格等于最终 accepted full tree。范围止于 05，06 不启。
- 05 实现交付：本地提交 `c5351ec79aadcfdf59da2b306f8be32e45567195`，parent `514f9ab48bed043fe1376af5a95934b2fc90a8cf`，tree `6ee9ae23822519937f74808e8d3a31b9ae87f77a` 严格等于最终 accepted full tree，仅 7 个授权路径；代码提交后隔离树 clean，未 push。交付分支 `codex/item-identity-05`，路径 `C:/Users/SinVC.Windy/.codex/worktrees/item-identity-05/agent_linker`。必要验证已通过；4 个既有 Broker ignored 测试不涉本轮，Broker 本步未触，为已记录限制，不扩大本轮范围。
- 05 结案协调例外：仅主树 README/global/05-progress 与隔离树 README/05-progress；其他原基线、strategy、其他 step 和实现不动，不合回代码。README 仅第 15 行 05 `[ ]`→`[x]`；隔离旧 raw SHA `3AA67BA6C869B9DA8404930CF179C49D50E27E5236ED4F866E1F193316EA2DA5` 的 LF 归一 SHA 精确等于主旧模板 `19637A065D7961BA5C69574BB0D1207F4FF555AC7D6546D6A213C962F12A89F4`，已获明确换行例外，备份旧 raw 后用同一主 LF 模板生成两树一致最终字节，Git 语义差异仅该 checkbox。05-progress 已核对主旧模板及最终源 SHA 守卫，仅同步当前步到主树，两树最终 raw SHA 均为 `969CF53DDFEC9E961E8A64B66785C94036A0E0E5F9D38C8BD3D1AD36C7B873B0`；未覆盖未知编辑。
- 后续代码起点：06 仅在用户另行授权后，才从交付分支 `codex/item-identity-05` 最新已提交 HEAD 新建下一隔离树；当前实现提交为 `c5351ec79aadcfdf59da2b306f8be32e45567195`。不得从主树协调 metadata HEAD 开始，旧交付与脏主树继续保留；06 当前未授权、未启动。
- 05 执行保全快照：`C:/Users/SinVC.Windy/.codex/visualizations/2026/10/31/01a19093-6f3b-7010-8857-5e731f7ca071/main-before.json`，SHA256 已核实为 `A875A106FAEE926F2ABB24703725C11FD686B033013F3BD802E9E4E2148846EC`；执行会话报告初始快照覆盖主树 HEAD `51b9b472937609425ae935798c65034a24feb3bf`、index、原 16 修改/4 未跟踪路径哈希与链接元数据。初次排除表达式误按 global 旧文件名前缀，初始 SHA 清单实际含 `dev-docs/重构编排/progress.md` 一项哈希，仅属哈希元数据，未读取正文入上下文、未写 global；后续核对已明确排除正确相对路径，88 个非 global 路径哈希/链接元数据与主树 HEAD/index 保全，旧交付未改。原快照文件及上述 SHA 不变，不覆盖或恢复原快照。
- 05 启动准备：本轮唯一全局维护者为统筹子代理 `/root/prepare_step05`；首次读取 SHA256 `57196B462FF819BF6E3729E8ABF53BFDB7B4BF1CC24AAEA13152DAA2F2E98FB5`，每次写前核验自身最新预期 SHA，不符即停写取证。执行与独立验收只用 [05 进度](steps/05-progress.md)，不读本索引或其他步骤进度。README 05 精确入口为 `dev-docs/重构计划/README.md:15`，05 下钻验收入口为 `dev-docs/重构计划/05-item-identity.md:24`。
- 05 保全恢复：主树 HEAD/index/status 与 88 个 tracked/untracked 路径的 SHA/链接元数据已存于统筹 visualization 的 `step05-recovery-20261030/`；当前仍为原 16 修改、4 未跟踪，`.tmp-cargo-migration` 当前不存在。未删除、回退或搬入旧修改。代码 ref 中 README 已包含 01–04 完成复选框与步骤隔离约定，无需另行导入。统筹明确授权本轮启动元数据仅三路径：本全局索引、清理旧轮动态文字的稳定 `strategy.md`、新建 `steps/05-progress.md`；新隔离树仅导入已审策略与当前步模板两文件，代码 ref 中旧策略由该已审副本替换。

| 步骤 | 当前状态 | 恢复入口 |
| --- | --- | --- |
| 01 | 已完成；实现和结案均已本地提交 | [01 进度](steps/01-progress.md) |
| 02 | 完成；独立 PASS，实现已本地提交 | [02 进度](steps/02-progress.md) |
| 03 | 完成；独立 PASS，实现已本地提交 | [03 进度](steps/03-progress.md) |
| 04 | 独立 PASS、实现已本地提交；仅结案协调文档收尾 | [04 进度](steps/04-progress.md) |
| 05 | 独立 PASS、实现已本地提交；仅结案协调文档收尾 | [05 进度](steps/05-progress.md) |

全局索引只维护当前授权、状态和步骤入口；会话 ID、差异、证据、阻塞及提交明细写在对应步骤文件。后续步骤获授权时才新增条目。
