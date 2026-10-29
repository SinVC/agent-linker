# 重构 TODO

范围仅为[最终裁决](../重构-最终裁决.md)第四至九章的具体修改，不收录第一至三章的决策论证，不增加补充设计或防御措施。本次只编写计划，以下实现任务均未完成。

按编号执行，每次只完成一个下钻文档规定的修改及其必要调用点、测试和契约同步。先读根 [AGENTS.md](../../AGENTS.md)、[架构入口](../architecture.md)和该步骤列出的条件文档。模块契约中尚未同步的旧行为按用户指定的最终裁决修改，不重新做设计裁决。当前工作区有未提交修改，以实际文件为基线，不回退或覆盖已有工作。

下钻文档的代码链接按 2026-10-28 工作区定位，同时标明符号名；前序步骤移动代码后，按符号定位并更新链接。每步验收结果写回该下钻文档，通过后再勾选此处。实际使用默认 Cargo cache 的验证命令遵守根 AGENTS.md 的直接提权要求。

## 阶段一：安全边界与状态隔离

- [x] 01 [统一路径入口与受控项目路径](01-paths.md)
- [ ] 02 [Manifest 来源隔离与解绑范围](02-manifest-origin.md)
- [ ] 03 [Manifest 原子保存](03-manifest-save.md)
- [ ] 04 [Symlink 原子替换与断链 force](04-symlink-replace.md)
- [ ] 05 [注册项标识符与 alias 查询空间隔离](05-item-identity.md)

## 阶段二：存储与 Schema v3

- [ ] 06 [Store 三入口与唯一内置 Seed 来源](06-store.md)
- [ ] 07 [写命令事务与分组输入幂等](07-transactions.md)
- [ ] 08 [移除 config 命令及运行时代码](08-remove-config.md)
- [ ] 09 [Schema v3 迁移与数据模型同步](09-schema-v3.md)
- [ ] 10 [分组列表聚合查询](10-group-summaries.md)
- [ ] 11 [数据库检查状态与退出码](11-db-check.md)
- [ ] 12 [init 消费已启用 Harness 映射](12-init-harness.md)
- [ ] 13 [安装和打包的显式初始化](13-distribution.md)

## 阶段三：执行链路与模块收敛

- [ ] 14 [Manifest 改用 serde + toml 与后端审计兼容](14-manifest-toml.md)
- [ ] 15 [共享 Symlink 判定与选项收敛](15-symlink-rules.md)
- [ ] 16 [共享 LinkPlan 与一次执行提交](16-link-plan.md)
- [ ] 17 [ProjectCtx 与命令层组装](17-project-context.md)
- [ ] 18 [拆出 Doctor 模块](18-doctor.md)
- [ ] 19 [结构化错误与 verbose 错误链](19-errors.md)
- [ ] 20 [CLI 公共结构与命令归一](20-cli.md)
- [ ] 21 [测试、平台代码隔离与剩余死代码删除](21-code-cleanup.md)
- [ ] 22 [裁决覆盖核对与故障回归验收](22-verification.md)

P0-2 的查询误选在 05 修复，数据库唯一性在 09 完成；P0-5 分别由 03、06、07、16 闭合。08 先移除 config 消费入口，09 再删除表；14、15、16、17 分别处理序列化、判定、执行计划和上下文，避免同一步混做这些重构。

## 统筹进度

- 统筹聊天 ID：`01a18414-be15-7533-8e09-04f9b9971ffc`；自动跟进 ID：`agent-linker`，每 5 分钟唤醒统筹，已更新为仅收尾 01，完成后删除，不推进 02。
- 调度与停止条件：用户最新指令“阶段1完成后暂停”按当前第 01 编号步骤执行；当前继续收尾 01，隔离候选构造、新验证与独立再验通过后由 `gpt-6.1 terra` 子代理本地提交，成功后暂停并删除自动跟进，不启动 02。执行与验收保持独立，后续证据写入对应下钻文档；此条件替换此前通过后继续下一编号步骤的安排。
- 当前阶段：01；状态：原工作树代码独立验收 PASS 保持／B 方案隔离候选已构造，正在专属构建目录验证，候选独立再验待完成。01 尚未提交，本步新验收与本地提交成功后暂停，02 不启动。README 自本次更新后冻结，除真正验收结论或提交结果外不再更新，包括 cursor，避免改变候选 tree。
- 执行会话：`01a18422-2ff4-7492-a3e3-3cfaa38030c9`（local，GPT-6.1 sol high）；候选保留 HEAD + 01 + 必要 receipt，已排除旧 readonly API 与调用、dbcheck、VACUUM 备份及安装根备份默认值；27 份计划来源文档按批准范围复制，不改当前工作树、index 或 HEAD；最后记录的 `wait_threads` cursor：`f198cc56-50e9-4387-864b-368588af54b9:23`。
- 独立验收会话：`01a1842f-d52d-7763-ad7b-0c5c4f94621c`（local，GPT-6.1 sol high，已保存主工作树指纹，等待候选专属二进制复验及树与保全证据）；最后记录的 `wait_threads` cursor：`581af4d1-fade-4994-b457-c0dae9c3970d:9`。原工作树历史证据：独立 Windows `cargo test --locked -- --include-ignored --test-threads=1` 的 77 单测与 2 个 CLI 集成测试全部通过、0 ignored，`cargo fmt --check` 通过；真实隔离 CLI 对 `./assets` 与 `.` 的 dry-run 无副作用、Broker 建链、status 正确、unlink、源保全均 PASS。16 个本步文件的 baseline/frozen/current 指纹匹配，代码联合 SHA256：`c3da3eac027edb82a3ab892de1de046fecb3d68694cce839734573f6c28cd12f`；独立报告已写入 `01-paths.md`。最初默认模型 `gpt-5.5` 不支持而启动失败，切换 sol high 后复启成功，非代码失败。`gpt-6.1 terra` 提交子代理：`/root/commit_step01`，本地提交尚未完成。
- 候选与提交要求：当前候选位于 `C:/Users/SinVC.Windy/.codex/visualizations/2026/10/28/01a18422-2ff4-7492-a3e3-3cfaa38030c9/b01/`，使用自己的构建目录验证。早期 terra 对照 patch 位于统筹可视化目录的 `step01-baseline-review/`；receipt-only 当时仅通过 apply check、未 build，full-overlap 仅供对照、禁止直接提交。当前候选必须重新验证，并交由原独立验收会话再次验收通过后才可由 terra 提交，不能沿用原工作树的旧 PASS。
- 只读反思会话：`01a18444-3feb-7c12-a5c4-5eb149937665`（local，GPT-6.1 sol high）；最后记录的 cursor：`3c7369eb-4ff1-4700-9cd7-b2a22e6d3a5f:1`。按用户对多次失败与遗漏的反思要求，审查 B 范围泄漏、残留锁测试根因与最终 tree 交接，不并发 Broker、不修改实现；原独立验收会话仍是唯一最终 PASS 负责者。当前 B 新发现残留旧 baseline 写锁测试，已交执行修正，此项并非主工作树代码新回归。
- 未解决问题：当前候选专属构建与独立再验、最终 tree 及工作树保全证据待完成。此前 01 增量应用到 HEAD 在模块 04 文档及 `db.rs`、`harness.rs`、`project_links.rs`、`registry.rs` 冲突，整文件提交会混入原有 Windows 安装路径与只读 DB 等基线改动，现通过隔离候选剥离无关改动。原工作树全部保留，HEAD 仍为 `369c5c9dcc01f14b6659ec43e0f8bd8646f3712b`，index 空，无提交、无 push，无用户决策事项。首次 FAIL 与验收返工第 1 轮历史保留——合法 Resource target-dir `.` 与 `./assets` 曾因父目录遍历的 `CurDir` 被拒绝，经最小修复、回归测试与独立真实 CLI 复核后通过。并行 Broker 曾一次报 `SERVICE_UNAVAILABLE`，仅作环境观察，串行全通过；macOS 由模拟覆盖，Linux 执行侧 77 单测与 2 个 CLI 测试日志已核对。
- 初始工作树基线（统筹记录前已有，不作为本次实现新增）：
  - 已修改根文档：`CLI_REFERENCE.md`、`README.md`、`RELEASE_CHECKLIST.md`。
  - 已修改设计文档：`dev-docs/architecture.md`、`dev-docs/modules/04-global-store-registry.md`、`dev-docs/release-distribution.md`。
  - 已修改安装打包脚本：`packaging/unix/install.sh`、`packaging/windows/install-broker.ps1`、`packaging/windows/install.ps1`、`packaging/windows/package-windows.ps1`、`packaging/windows/uninstall.ps1`。
  - 已修改代码路径（仅记录状态，未读取代码）：`src/core/db.rs`、`src/core/harness.rs`、`src/core/project_links.rs`、`src/core/registry.rs`。
  - 已有未跟踪文档及计划目录：`CLI_REFERENCE_zh.md`、`dev-docs/handoff-windows-install-data-layout.md`、`dev-docs/重构-claude.md`、`dev-docs/重构-gpt.md`、`dev-docs/重构-最终裁决.md`、`dev-docs/重构计划/`。
  - 已有未跟踪安装脚本：`packaging/unix/test-install.sh`、`packaging/windows/maintain-install.ps1`、`packaging/windows/test-install.ps1`。
