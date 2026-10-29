# 01 · 进度与证据

当前状态：完成；无未解决阻塞。本文仅记录第 01 步，后续授权由统筹另行记录。

- 静态计划与已有实现、验收历史：[01-paths.md](../../重构计划/01-paths.md)。该下钻文档中保留的历史记录不改写。
- 实现提交：`476781bfbb6dfd7ac96edf978daef99f30f3f3c4`；父提交 `369c5c9dcc01f14b6659ec43e0f8bd8646f3712b`；受验 tree `79f5c8a0c35ab522999ff3ca0dd99625e924fdaa`。
- 结案文档提交：`c455f1efd9171eac142ce03f07c70cef42dee7b5`（仅计划 README；提交说明 `Record step 01 completion and pause`）。两次提交均为本地提交，无 push。
- 原工作树与隔离 B 候选的验收版本、结果和已有基线分别保留，不能互相替代。

## 原 README 统筹记录（完整迁移）

以下是第 01 步结案时的历史快照；其中“暂停”“02 未启动”等描述只反映当时状态，不代表当前授权。
- 统筹聊天 ID：`01a18414-be15-7533-8e09-04f9b9971ffc`；自动跟进 `agent-linker` 已由统筹删除，当前关闭。
- 调度与停止条件：第 01 编号步骤实现、独立验收及本地实现提交均已完成，按用户要求暂停；02 未启动，不推进其他阶段。本次仅补 README 统筹结案记录，交由 terra 做仅此文件的本地文档提交，不修改已受验实现提交。
- 当前阶段：01；原工作树实现与 B 隔离候选分别独立验收 PASS。本地实现提交：`476781bfbb6dfd7ac96edf978daef99f30f3f3c4`；父提交：`369c5c9dcc01f14b6659ec43e0f8bd8646f3712b`；实现 tree：`79f5c8a0c35ab522999ff3ca0dd99625e924fdaa`，42 个批准路径，与最终受验 tree 完全一致。
- 执行会话：`01a18422-2ff4-7492-a3e3-3cfaa38030c9`；B 候选保留指定 HEAD + 01 + 必要 Receipt，排除旧 readonly API/调用、dbcheck、VACUUM 备份、安装根备份默认值及对应既有测试；27 份计划与直接来源文档按批准范围纳入。最终本地实现提交由指定 terra 子代理 `/root/commit_step01_final` 完成，无 push。
- 独立验收会话：`01a1842f-d52d-7763-ad7b-0c5c4f94621c`；B 候选独立重新构建并通过 `cargo fmt --check`、完整 Windows 73 个单测及 2 个 CLI 集成测试（含 3 项真实 Broker 安全测试，0 failed、0 ignored）。候选专属二进制对 `.` 与 `./assets` 的 dry-run 无副作用、真实 symlink 创建、正确状态、解绑和 source 内容保全均通过。原工作树历史 77+2 PASS 与 B 新 PASS 分别记录，不互相替代。
- 候选与证据保留：执行产物根 `C:/Users/SinVC.Windy/.codex/visualizations/2026/10/28/01a18422-2ff4-7492-a3e3-3cfaa38030c9/` 中的 `b01/`、`b01-candidate.json`、`b01-head-to-candidate.patch`、`b01-git/index` 及 `step01-baseline/` 保留；最终独立报告为验收产物根 `C:/Users/SinVC.Windy/.codex/visualizations/2026/10/28/01a1842f-d52d-7763-ad7b-0c5c4f94621c/b01-independent-acceptance.md`。B 全部 src/tests 联合 SHA256：`1bd699cc8170fada567610fbc54cbd7a2ad9cfe62b4f9e003f9e7aec71989f33`；独立从 HEAD 重放 patch 得到相同实现 tree。
- 只读反思会话：`01a18444-3feb-7c12-a5c4-5eb149937665`，已完成。残留旧基线写锁测试已按范围排除；README、B 专属记录及 Git symlink 交接门槛均闭合。提交后独立只读元数据复核确认 commit/parent/tree 一致，`CLAUDE.md` 保持 mode `120000`、blob `93d17ee607e3086f248831776f9c6db0d3cc9cf3` 和原 target；index 与实现 HEAD 一致，无暂存修改。
- 本步未解决阻塞：无。CurDir 回归及提交范围冲突已解决；Broker 并发曾一次 `SERVICE_UNAVAILABLE` 作为环境观察保留，串行专项通过，未宣称并发可用性已验证；真实 macOS 未执行，仅路径模拟覆盖，Linux 候选 73+2 日志已核对。后续步骤不因本记录启动。
- 工作树保全：工作树并不干净；既有根/安装/发布文档、脚本、数据库相关源码等未提交基线继续保留。主工作树 `01-paths.md` 与已提交候选中的 B 记录仍有保留差异，显示为未暂存；未同步覆盖该文件或其他主文件。本 README 统筹区结案是明确授权例外，随后仅提交此文档变化，不混入这些既有改动。
- 初始工作树基线（统筹记录前已有，不作为本次实现新增）：
  - 已修改根文档：`CLI_REFERENCE.md`、`README.md`、`RELEASE_CHECKLIST.md`。
  - 已修改设计文档：`dev-docs/architecture.md`、`dev-docs/modules/04-global-store-registry.md`、`dev-docs/release-distribution.md`。
  - 已修改安装打包脚本：`packaging/unix/install.sh`、`packaging/windows/install-broker.ps1`、`packaging/windows/install.ps1`、`packaging/windows/package-windows.ps1`、`packaging/windows/uninstall.ps1`。
  - 已修改代码路径（仅记录状态，未读取代码）：`src/core/db.rs`、`src/core/harness.rs`、`src/core/project_links.rs`、`src/core/registry.rs`。
  - 已有未跟踪文档及计划目录：`CLI_REFERENCE_zh.md`、`dev-docs/handoff-windows-install-data-layout.md`、`dev-docs/重构-claude.md`、`dev-docs/重构-gpt.md`、`dev-docs/重构-最终裁决.md`、`dev-docs/重构计划/`。
  - 已有未跟踪安装脚本：`packaging/unix/test-install.sh`、`packaging/windows/maintain-install.ps1`、`packaging/windows/test-install.ps1`。
