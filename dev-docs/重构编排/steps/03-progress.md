# 03 · 进度与证据

状态：03 独立验收 PASS，实现已本地提交并结案；同轮 02 亦独立 PASS。02→03 组合轮次已完成，止于 03，04 未启动；Windows 已实测，Unix 未实跑。

- 静态计划：[03-manifest-save.md](../../重构计划/03-manifest-save.md)。组合执行聊天仅接收 02/03 两份进度，不读取全局索引或范围外步骤进度。
- 只读评估结论：统筹已确认可在同一执行聊天先 02 后 03，分别独立验收；评估聊天及报告位置待统筹登记。
- 执行聊天：本隔离执行聊天；独立验收聊天：`01a18710-e2f4-79f0-8096-c7209d319207`。
- 受验版本与实跑证据：见下方实现证据及独立验收区；03 判定 PASS，实现提交与结案元数据见末尾。
- 阻塞：无未解决缺陷或阻塞；原基线与保全证据见下方，Unix 未实跑限制保持。
- 实现提交：`f583cc8c3daded38bd47f8b003e7a284ee60bb45`；当前结案文档尚待独立审阅与指定 terra 提交，不记录文档提交自身 hash。

写回规则：下钻计划中的旧执行记录/勾选 TODO 提示由本隔离规则替代。动态状态、实跑证据与验收结论仅写本 `03-progress.md`；README 仅静态导航、不勾选，不回写其他步骤。

## 本轮实施起点与基线保全

实施固定起点：`b1781bcee041296ab28398ff9e96bb1b9010c46e`；实际分支 `codex/manifest-origin-save-02-03`；worktree `C:\Users\SinVC.Windy\.codex\worktrees\manifest-origin-save\agent_linker`。主树已有修改未移入候选，未 stash/reset/覆盖/合回；实现已完成下方记录的本地提交。

本文件分别登记 03 自身的范围、证据与独立验收结论，不以组合轮次的一次总 PASS 代替。不得自动推进 04。

## 03 实现与验收覆盖（2026-10-29）

- 02 完成并保存独立检查点后开始 03。03 生产改动仅 `src/core/manifest.rs` 保存过程；模块 02 同步原子保存契约；两份静态下钻仅更新最终符号行号和指向当前步骤进度的导航，未写入动态状态。
- `save_manifest` 保留校验、受控入口和调用点 Result 传递；独占创建同目录 `.agents/.links.toml.tmp.<pid>`，完整写入、关闭后 `fs::rename` 替换原清单，不先删目标。失败时尝试清理本次创建的临时文件，预先占用的路径不覆盖、不清理。
- 新建/替换/往返：`atomic_save_creates_and_replaces_existing_manifest` 直接在真实 Windows 文件系统验证新建及已有目标替换，原有完整往返与损坏清单不覆盖测试继续通过。
- 临时创建失败：`atomic_save_temporary_creation_failure_preserves_original_and_occupied_path` 分别以预存真实文件、目录阻止 create_new，原清单与占用路径内容完整。
- 临时写入失败：`atomic_save_partial_write_failure_preserves_original` 使用私有 I/O 注入边界，实际先写部分字节再返回 WriteZero，断言 rename 不调用、原字节不变、半成品清理且可再次保存。
- rename 失败：`atomic_save_rename_failure_preserves_original` 在完整临时内容写入后注入 PermissionDenied；`atomic_save_windows_locked_destination_preserves_original_and_allows_retry` 使用真实 Windows 拒绝 delete-sharing 的目标句柄令 `fs::rename` 失败，验证原字节不变、临时清理、解锁后替换和往返成功。
- Unix/Linux/macOS 本轮无可运行环境，未运行；跨平台保存测试保留为同一测试源，不能视作 Unix 平台已通过。

## 最终共同版本实跑证据

所有命令均显式在上方隔离 worktree 执行，Cargo 直接提权并使用默认缓存。证据根目录：`C:\Users\SinVC.Windy\.codex\visualizations\2026\10\29\01a1870a-76e2-70f1-8fe6-856b528d5949\preservation`。

- `cargo test --locked`：退出 0；78 单元通过，3 个既有 Broker 测试常规 ignored；2 集成通过。日志 `final-cargo-test.log`。03 首轮 4 项保存测试通过；新增部分写失败测试后在此最终源码重新完整验证。
- `cargo test --locked -- --ignored --test-threads=1`：退出 0，3 个 ignored Broker 测试显式串行全部通过（0 ignored），服务 `WinSymlinksBroker` 为 Running；日志 `final-broker-ignored.log`。不将常规跳过误记为真实平台通过。
- `cargo fmt --check`：退出 0，日志 `final-fmt.log`。
- 主树前后核对：`main-before.json`、`main-after.json`、`main-comparison.json`；HEAD 与 index SHA256 一致，前后 87 路径；唯一变化 `dev-docs/重构编排/progress.md` 为统筹授权例外，其余 tracked/untracked 哈希无变化。
- 最终源码指纹：同证据目录 `final-files.json`；最终可重建差异 `final-candidate.patch`（以上固定 HEAD 为基底）。交接后本执行会话停止源码与 02/03 进度写入，Broker 交独立验收独占；独立验收可在这两份进度补自己的结论。
- 未新增备份、日志协议、LinkPlan 或批量只保存一次能力；04 及后续阶段未实施。独立验收须分别判 02、03；本实现会话不提交、不 push。

## 独立验收区（2026-10-29）

判定：**PASS（03，Windows 已实测；Unix 未实跑）**。验收聊天 `01a18710-e2f4-79f0-8096-c7209d319207` 在执行方正式冻结实现与 progress 后，独立审阅最终共同版本，实际运行验证后作出本步判定；与 02 分别判定。

- 受验路径 `C:\Users\SinVC.Windy\.codex\worktrees\manifest-origin-save\agent_linker`；分支 `codex/manifest-origin-save-02-03`；HEAD `b1781bcee041296ab28398ff9e96bb1b9010c46e`；基底 tree `e499ceee9ec8ebc4e35fc895f404cd15d21b8aac`。验收前可重建 patch SHA256 `53C23D1F89927D4A1EABD35244976FC35F28B4940F0346F04308F59ADE280B26` 与交接完全匹配，10 文件原始字节核对一致。验收后仅追加两份当前 progress 的独立区，源码不变。
- `cargo fmt --check` 退出 0；`cargo test --locked` 退出 0（78 单元、2 集成通过）；`cargo test --locked -- --ignored --test-threads=1` 退出 0（3 个 Broker 专项全部通过）。三者在受验绝对 cwd、默认 Cargo cache、明确提权下运行。`git diff --check` 通过。
- 实现审查：保留 Manifest 与项目路径验证，目标、父目录、临时路径复用受控入口；同目录 `.agents/.links.toml.tmp.<pid>` 独占创建，写完关闭文件后仅以 `fs::rename` 替换原目标；没有预删原文件。私有 I/O 注入用于验证既有写入/rename 边界，未新增日志、备份、LinkPlan 或持久化协议。各原调用点 `?` 已传递保存错误，无需改写 commands。
- 真实 Windows 正常替换：`atomic_save_creates_and_replaces_existing_manifest` 验证首次新建、替换已存在目标、完整 load 回读和目录仅剩目标。完整往返与 damaged_manifest_fails_without_overwrite 在相同最终版本通过。
- 临时创建失败：预存真实文件、真实目录分别令 create_new 失败，断言原清单字节及原占用路径保持；此失败不会误清理已有用户路径。
- 部分写入失败：`atomic_save_partial_write_failure_preserves_original` 实际向临时文件写入前半段，再注入 WriteZero；rename closure 若调用即 panic。断言调用失败、原字节完全相同、半成品清理、下一次正常保存回读成功。不是只用 open 失败替代写入失败。
- rename 失败：注入测试断言临时与目标同父目录、临时内容完整且原目标尚保持，再返回 PermissionDenied，失败后原字节/回读/清理通过。Windows 锁定目标测试用拒绝 delete sharing 的真实句柄使实际 fs::rename 失败，检查原字节不变、临时清理，释放句柄后替换/回读成功；此项为真实文件系统验证，未使用 Mock 代替。
- 兼容及边界：02 检查点至最终 manifest.rs 的增量只有保存流程和五项保存测试；未改序列化器结构或保存批次语义。03 的当前下钻全部代码链接命中最终符号。真实 CLI 的 19 项独立断言还实际触发了 init/link/unlink 保存过程和旧清单重存，全部通过。
- 指纹：manifest.rs `1F11783B1B16E4CC40DA1269C23E6B987F12230C8DB9707839815F6A6ECA9542`；project_links.rs `327F61470D69DA922BC5AFB1D33353B71D834837A201E5C0B6F7DA6D9AE45304`；cli.rs `2D0E216DF155BCB6416744138B3D60A802165B1F5D9E2D683190AD7CB639AEB3`。src/tests/Cargo 的 19 路径指纹实跑前后全部一致；清单见独立证据根 before-src-tests.json（SHA256 `1816F09697FD5FD2F8F28E6E3CD5F1D3967FCBBB630F64577E6E3093D72E67B8`）。
- 独立证据根：`C:\Users\SinVC.Windy\.codex\visualizations\2026\10\29\01a18710-e2f4-79f0-8096-c7209d319207\acceptance`，含 cargo-test.log、fmt.log、broker-ignored.log、real-cli.log、source-verification.json、doc-links.log。独立比较执行方主树两份 87 路径快照：HEAD/index 一致，仅统筹授权全局 progress 变化，其余哈希保持；本验收未读写主树实现。
- 未解决缺陷/阻塞：无。Unix/Linux/macOS 未运行，按计划明确记录，不能据 Windows 结果宣称 Unix 实测通过。两步分别 PASS 后，统筹可将验收记录追加后的最终差异及相同源码指纹交指定 terra 本地提交；本验收不 commit/push，不推进 04。

## 提交与结案（2026-10-29）

- 本步独立结论：03 独立验收 **PASS**；组合轮次的 02、03 均已分别 PASS，无未解决缺陷或阻塞。先前实跑命令、断言、源码指纹和证据路径保留在上方；Windows 已实际验证，Unix/Linux/macOS 未实跑的限制保持，不宣称 Unix 平台实测通过。
- 实现提交：`f583cc8c3daded38bd47f8b003e7a284ee60bb45`；parent `b1781bcee041296ab28398ff9e96bb1b9010c46e`；提交 tree `eadfbdf080b65d61c0e40a452a3fda01920ad91f`，与独立验收候选完全一致，恰含 10 个授权文件。
- 成果保留在分支 `codex/manifest-origin-save-02-03`、worktree `C:\Users\SinVC.Windy\.codex\worktrees\manifest-origin-save\agent_linker`。本地实现提交后，结案复核仅读取 Git 元数据及两份当前 step 文档，不读取源码、不重新测试；更新前隔离工作树与 index 均干净。本次新增两份结案文档差异不属于上述已提交实现 tree，不声称当前工作树仍等于该 tree。
- 统筹明确授权仅将这两份 step 进度同步至主树同相对路径，属于新增协调文档例外；复制前两份主树文件分别匹配初始模板 SHA256：02 `14ef8902e64a7652b38facbf857ed23a2c025bc2e4cb5a1fe6cc87d0bb54ff34`，03 `d2561c6f9d47edad8ae241da6b640d2ac1291359a98832ec03fff5fdc1c34bcf`。本次仅同步当前两份进度，不合回实现代码，不覆盖其他主树 baseline。
- 统筹已确认自动跟进关闭。02→03 组合轮次完成并止于 03，04 未启动。本次收尾不提交、不 push、不合回代码；结案文档交独立文档审阅后，由指定 terra 另行提交，不写文档提交自身 hash。