# 02 · 进度与证据

状态：02 独立验收 PASS，实现已本地提交并结案；同轮 03 亦独立 PASS。02→03 组合轮次已完成，止于 03，04 未启动。

- 静态计划：[02-manifest-origin.md](../../重构计划/02-manifest-origin.md)。按统筹提供的计划 README 精确行引用进入本步，自行读取所列条件文档；组合执行聊天仅接收 02/03 两份进度，无需读取范围外步骤进度。
- 执行聊天：本隔离执行聊天，分支 `codex/manifest-origin-save-02-03`。
- 独立验收聊天：`01a18710-e2f4-79f0-8096-c7209d319207`。
- 02/03 只读评估结论：统筹已确认可在同一执行聊天先 02 后 03，分别独立验收；评估聊天及报告位置待统筹登记。
- 受验版本与实跑证据：见下方 02 实施检查点及独立验收区；02 判定 PASS，实现提交与结案元数据见末尾。
- 阻塞：无未解决缺陷或阻塞；原基线与保全证据见下方，Unix 未实跑限制保持。
- 实现提交：`f583cc8c3daded38bd47f8b003e7a284ee60bb45`；当前结案文档尚待独立审阅与指定 terra 提交，不记录文档提交自身 hash。
- 停止条件：授权阶段完成后停止；不得因本记录自动推进 04。

写回规则：下钻计划中的旧执行记录/勾选 TODO 提示由本隔离规则替代。动态状态、实跑证据与验收结论仅写本 `02-progress.md`；README 仅静态导航、不勾选，不回写其他步骤。组合轮次的 03 自身记录独立写其进度文件。

## 本轮实施起点与基线保全

固定起点：`b1781bcee041296ab28398ff9e96bb1b9010c46e`。执行 worktree：`C:\Users\SinVC.Windy\.codex\worktrees\manifest-origin-save\agent_linker`；实际分支：`codex/manifest-origin-save-02-03`。从该提交新建，未带入主树已有修改。主树仅保全，不 stash/reset/覆盖或合回；统筹更新主树 `dev-docs/重构编排/progress.md` 是授权例外。

执行前主树 HEAD、index SHA256、tracked + untracked 共 87 路径快照位于 `C:\Users\SinVC.Windy\.codex\visualizations\2026\10\29\01a1870a-76e2-70f1-8fe6-856b528d5949\preservation\main-before.json`；结束时另行比对。

## 02 实施检查点（2026-10-29）

- 实现：`src/core/manifest.rs` 引入 `LinkOrigin` 与可选 Harness 名称，兼容旧 registry/Harness/scope 字段，新写出 origin 且不写 scope；更新 gitignore 托管块，精确保留块外换行、空行及无末尾换行的文本。
- 调用点：`src/core/project_links.rs` 的 item 构造、all/group 筛选；`src/cli.rs` 的测试记录构造。模块 02、05、06 同步来源、解绑范围、rename 的不可变 ID 以及 gitignore 契约。未改动序列化器架构或 03 保存逻辑。
- 回归覆盖：旧 Harness/registry/混合清单完整往返和字段保留；来源/名称校验；当前分组成员及多 alias 全部解绑；同 ID 的 Harness 记录仍保留；分组外 item 保留；rename 保持 ID、Manifest 不变并可按 ID 解绑；all 的 dry-run/实际解绑保留 CLAUDE.md 和 .claude/skills 及记录；gitignore 旧块更新与重复 init。
- 检查点命令（均在上方 worktree，Cargo 直接提权、默认缓存）：`cargo test --locked` 退出 0，73 单元测试通过、3 个需 Broker 的既有测试 ignored，2 集成测试通过；`cargo fmt --check` 退出 0；`git diff --check` 退出 0。
- 首轮与第二轮测试发现新综合测试 Mock 分别缺 `.claude`、`docs` 父目录；仅补齐测试 fixture，第三轮全套通过。无未解决失败。
- 独立保存的 02 代码/契约 patch：`C:\Users\SinVC.Windy\.codex\visualizations\2026\10\29\01a1870a-76e2-70f1-8fe6-856b528d5949\preservation\step02\step02.patch`，SHA256 `6DDC51FF895D2EB282D52941EE98938D708355E238E677AA26E08AF76CD7AC81`；同目录 `files.json` 与源文件副本固定 03 实施前状态。此检查点未提交，也不代表独立验收 PASS。
- 02 检查点已完成后才开始 03。共同最终版本及主树保全核对结果在本文件继续独立登记。

## 02 在最终共同版本的验证

- 最终 `cargo test --locked` 退出 0：78 单元通过、3 个需 Broker 的既有测试常规 ignored、2 集成通过；02 专属来源兼容、解绑范围、rename ID 与 gitignore 测试均再次通过。
- 随后 `cargo test --locked -- --ignored --test-threads=1` 退出 0，3 项真实 Windows Broker 边界测试串行通过；`cargo fmt --check` 退出 0。日志位于上方证据根目录 `final-cargo-test.log`、`final-broker-ignored.log`、`final-fmt.log`。
- 主树核对 `main-comparison.json`：HEAD/index 不变，前后 87 路径，唯一变化为统筹获准更新的 `dev-docs/重构编排/progress.md`，其余 tracked/untracked SHA256 一致。
- 受验源码/契约指纹 `final-files.json` 与可重建 `final-candidate.patch` 位于相同证据根目录，以固定 HEAD 为基底。02 专属初始 patch 仍保留；03 的新增保存逻辑及覆盖在其自身进度记录。
- 源码与本进度在交接后冻结；独立验收分别判断 02 和 03。未提交、未 push，04 未实施。

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

## 独立验收区（2026-10-29）

判定：**PASS（02）**。验收聊天 `01a18710-e2f4-79f0-8096-c7209d319207` 在执行方明确冻结实现及两份 progress、交出 Broker 后，对 02/03 最终共同版本审阅和实跑；本区为独立结论，不采信执行会话自称完成作为通过依据。

- 受验 worktree：`C:\Users\SinVC.Windy\.codex\worktrees\manifest-origin-save\agent_linker`；分支 `codex/manifest-origin-save-02-03`；HEAD `b1781bcee041296ab28398ff9e96bb1b9010c46e`，基底 tree `e499ceee9ec8ebc4e35fc895f404cd15d21b8aac`。验收前重导出的 `reviewed-candidate.patch` SHA256 为 `53C23D1F89927D4A1EABD35244976FC35F28B4940F0346F04308F59ADE280B26`，与执行方交接完全相同；10 个变更文件原始字节亦全部匹配交接 `final-files.json`。02 检查点 patch SHA256 `6DDC51FF895D2EB282D52941EE98938D708355E238E677AA26E08AF76CD7AC81` 已核对。
- 命令均显式以受验绝对路径为 cwd，Cargo 默认缓存直接提权。`cargo fmt --check` 退出 0；`cargo test --locked` 退出 0，78 单元、2 集成通过，3 个专项在常规轮次 ignored；随后 `cargo test --locked -- --ignored --test-threads=1` 退出 0，3 个 Broker 专项显式串行全部通过。没有将 ignored 计作已验证。
- 断言审查：旧 Harness、旧 registry、混合及新旧混合来源读取/完整往返，ID、item_id、路径、时间保持；新写出 origin 且不写 scope，item 无 Harness 名称；异常来源与不一致名称被拒绝。临时数据库/Manifest/Mock 测试覆盖当前分组成员、多 alias、rename 后不可变 ID、同 ID 的 Harness 保留、分组外 item 保留、all 的 dry-run/实际解绑以及 Harness 记录和 Provider entry 保持。
- 独立真实 CLI 脚本 `real-cli.ps1` 对最终 `target\debug\aglink.exe` 在全新项目及隔离 AGLINK_DATA_DIR 上执行 19 项断言，全部通过：两次 init，真实 Broker 链接及 status，旧混合清单加载再保存与 item 记录逐字保持，rename 不动 Manifest，当前分组成员的两 alias 全部移除，组外成员保留，all dry-run/实际解绑后恰好保留两个真实且目标正确的 Harness symlink，源实文件完整。托管块替换精确保留块外 CRLF、空行与无末尾换行，规则为 `.claude/`、`.agents/`，共享 AGENTS.md 保持。
- 两次 Mock fixture 修复合理：仅给新增综合测试登记 `.claude`、`docs` 父目录。既有 Mock 的 parent_exists/生产 Provider 的父目录要求未放宽；生产流程创建真实父目录，不是掩盖实现缺陷。
- 范围核查：仅 10 个授权文件有差异；02 的 project_links.rs/cli.rs 与 02 检查点指纹完全一致；02 检查点至最终 manifest.rs 增量只有 03 保存过程、I/O 测试边界和保存测试。模块 02/05/06 契约与两份当前下钻链接已同步，19 个代码行链接逐一命中对应符号。未修改 README、strategy、其他 step、全局 progress；未实施 04、LinkPlan、序列化器替换或批量只保存一次。
- 保全：独立比较执行方 main-before/main-after 两份 87 路径快照，HEAD/index 一致，仅统筹获准的全局 progress 哈希变化；其余 tracked/untracked 指纹一致。本验收未读/修改主树实现，未 stash/reset/commit/push。
- 独立证据根：`C:\Users\SinVC.Windy\.codex\visualizations\2026\10\29\01a18710-e2f4-79f0-8096-c7209d319207\acceptance`；包含 fmt.log、cargo-test.log、broker-ignored.log、real-cli.ps1/real-cli.log、doc-links.log、preservation-independent-comparison.json、step02-to-final-manifest.patch。`before-src-tests.json` 覆盖 src/tests/Cargo 配置 19 文件，SHA256 `1816F09697FD5FD2F8F28E6E3CD5F1D3967FCBBB630F64577E6E3093D72E67B8`；实跑后重新核对 19/19 无变化。
- 源码 SHA256：manifest.rs `1F11783B1B16E4CC40DA1269C23E6B987F12230C8DB9707839815F6A6ECA9542`；project_links.rs `327F61470D69DA922BC5AFB1D33353B71D834837A201E5C0B6F7DA6D9AE45304`；cli.rs `2D0E216DF155BCB6416744138B3D60A802165B1F5D9E2D683190AD7CB639AEB3`。
- 未解决问题：无。Unix 平台未实跑，未宣称跨平台实测通过。02 的 PASS 与 03 的独立判定分开记录；本验收只追加本区和 03 独立验收区，不修改实现、不提交。验收记录追加后的可重建最终 patch/文件指纹由同证据目录另行导出，源码保持上述哈希；由统筹交指定 terra 做本地提交。

## 提交与结案（2026-10-29）

- 本步独立结论：02 独立验收 **PASS**；组合轮次的 02、03 均已分别 PASS，无未解决缺陷或阻塞。先前实跑命令、断言、源码指纹和证据路径保留在上方；Windows 已实际验证，Unix/Linux/macOS 未实跑的限制保持，不宣称 Unix 平台实测通过。
- 实现提交：`f583cc8c3daded38bd47f8b003e7a284ee60bb45`；parent `b1781bcee041296ab28398ff9e96bb1b9010c46e`；提交 tree `eadfbdf080b65d61c0e40a452a3fda01920ad91f`，与独立验收候选完全一致，恰含 10 个授权文件。
- 成果保留在分支 `codex/manifest-origin-save-02-03`、worktree `C:\Users\SinVC.Windy\.codex\worktrees\manifest-origin-save\agent_linker`。本地实现提交后，结案复核仅读取 Git 元数据及两份当前 step 文档，不读取源码、不重新测试；更新前隔离工作树与 index 均干净。本次新增两份结案文档差异不属于上述已提交实现 tree，不声称当前工作树仍等于该 tree。
- 统筹明确授权仅将这两份 step 进度同步至主树同相对路径，属于新增协调文档例外；复制前两份主树文件分别匹配初始模板 SHA256：02 `14ef8902e64a7652b38facbf857ed23a2c025bc2e4cb5a1fe6cc87d0bb54ff34`，03 `d2561c6f9d47edad8ae241da6b640d2ac1291359a98832ec03fff5fdc1c34bcf`。本次仅同步当前两份进度，不合回实现代码，不覆盖其他主树 baseline。
- 统筹已确认自动跟进关闭。02→03 组合轮次完成并止于 03，04 未启动。本次收尾不提交、不 push、不合回代码；结案文档交独立文档审阅后，由指定 terra 另行提交，不写文档提交自身 hash。