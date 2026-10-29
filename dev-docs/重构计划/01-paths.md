# 01 · 统一路径入口与受控项目路径

返回 [TODO](README.md)。前置：无。依据：最终裁决第四章 P0-1、第六章第 4 项、第七章 paths 职责、第八章阶段一第 1 项。条件文档：[01](../modules/01-symlink-core.md)、[02](../modules/02-project-init-manifest.md)、[04](../modules/04-global-store-registry.md)、[05](../modules/05-linkable-item.md)、[06](../modules/06-cli-command-surface.md)。

## 本步结果与边界

统一项目内受控路径解析、全局路径解析和项目根定位。只改路径责任及其调用点，Manifest 来源格式、保存算法、Store 连接和数据库结构分别留给后续步骤。

## 代码入口

| 位置 | 当前符号与用途 |
| --- | --- |
| [paths.rs](../../src/core/paths.rs#L50) | `resolve_managed_project_path`，统一词法规则和祖先检查；`find_project_root`、`normalize_windows_path` |
| [全局路径类型](../../src/core/paths.rs#L130) | `DbPathReason`、`DbPathResolution`、`DbPathContext`、`TargetPlatform` 已从 `db.rs` 迁入 |
| [数据库路径](../../src/core/paths.rs#L190) | `resolve_database_path*`、Receipt 解析、平台默认路径和环境解析 |
| [Manifest 校验](../../src/core/manifest.rs#L60) | `Manifest::validate`；[读取](../../src/core/manifest.rs#L202)、[保存](../../src/core/manifest.rs#L224)及记录路径祖先检查 |
| [项目命令](../../src/core/project_links.rs#L149) | 所有 `*_current_project*` 使用上查入口；[父目录准备](../../src/core/project_links.rs#L1063)和所有受控 source/link 解析 |
| [来源规范化](../../src/core/linkable.rs#L190) | `validate_source_path` canonicalize 后使用统一 Windows 规范化 |
| [映射路径](../../src/core/harness.rs#L42) | `HarnessMapping::source_in/link_in` 返回安全解析结果；[命令编排](../../src/commands/mod.rs#L265)改用 `paths` 的数据库解析入口 |
| [CLI 工作目录测试](../../tests/project_discovery.rs#L1) | 子进程真实 cwd 下验证 status/clean/unlink/link dry-run 上查和 init 不上查 |

## 修改清单

1. 在 `core/paths.rs` 实现 `resolve_managed_project_path(project_root, rel_path)`：拒绝空路径、绝对路径、盘符前缀和任何 `..` 组件；逐级用 `fs::symlink_metadata` 检查已有父目录，不允许父目录是 symlink。解析入口不创建目录。
2. 创建、读取、状态检查、unlink、clean 以及 `.agents/links.toml` 的读写全部使用该入口。检查 `.agents` 本身为真实目录；允许受管理链接的末级路径是 symlink，不能把合法链接本体一并拒绝。项目内相对 source 也经安全入口解析；注册项绝对 source 仍允许位于项目外。
3. `Manifest::validate` 校验每条 `link_path` 为合法项目相对路径，并检查唯一性；Windows 下按大小写无关判重。相对路径的词法规则与统一入口保持同一口径。
4. 将全局数据库路径类型、环境解析、平台默认路径和 Receipt 识别从 `db.rs` 迁至 `paths.rs`；更新 imports 和原有路径测试。保持 `AGLINK_DATA_DIR`、Windows 安装、portable、平台默认的现有优先级。Receipt 最小身份校验为 `schema_version == 1`、`product == "agent-linker"`、`portable == false`；损坏或不可读仍报错，不再依赖 `command`、`installed_by` 字段值。
5. 项目操作 `link/unlink/status/clean/doctor` 从 `current_dir()` 向上查找 `.agents/links.toml`，遇文件系统根或符号链接目录停止；`init` 严格使用当前目录。删除与上述定位重复的 `cwd.is_dir()` 防御检查。
6. 统一 Windows canonical 路径规范化：普通、非超长磁盘路径去掉 `\\?\` 前缀后用于落库、展示和比较；不对超长路径或其他前缀一概截断。
7. 同步模块 02、04、05、06 的对应路径契约。根架构只保留导航和职责描述。

## 验收

- 临时 Manifest 拒绝绝对路径、盘符路径、`..` 和重复路径；Windows 覆盖大小写重复。
- 临时项目的 `.agents` 或链接父目录被换为指向项目外的 symlink 时，load/save/status/link/unlink/clean 均在访问外部受控路径前失败，外部内容保持原样。实际祖先检查使用真实临时目录和平台 symlink；Provider 行为使用 mock。
- 从子目录操作能定位已有项目；`init` 不上查；上查不跨越符号链接目录。
- 现有多平台路径、portable 和安装 Receipt 测试迁移后通过；补齐最小 Receipt 和 Windows 路径规范化用例。
- 运行 `cargo fmt --check` 和 `cargo test --locked`，记录实际平台与未执行的平台项。

## 执行记录

2026-10-28：实现与执行侧验证完成，源码已冻结，等待独立验收；未勾选 TODO、未暂存、未提交。独立验收结果请另行追加，不以本记录代替验收。

### 基线与本步差异

首次源码编辑前已记录初始 Git 状态（15 个已修改跟踪文件、9 个顶层未跟踪路径条目），并在仓库外保存基线，避免将原有工作混入阶段提交：

```text
C:/Users/SinVC.Windy/.codex/visualizations/2026/10/28/01a18422-2ff4-7492-a3e3-3cfaa38030c9/step01-baseline/
```

该目录包含 `tracked.patch`、`status.txt`、本步涉及文件的原始副本、`untracked/` 下原有未跟踪文件快照。相邻 `step01-delta.patch` 是相对此基线的本步差异，`step01-delta.json` 列出文件及前后 SHA-256，`step01-final/` 保存冻结交付文件。以上证据均不进入仓库提交。

本步修改文件：

- `src/core/paths.rs`：项目路径词法与真实父目录检查、项目根上查、全局数据库路径职责及测试、Windows canonical 路径规范化。
- `src/core/manifest.rs`：link_path 合法性与唯一性、load/save 及相对 source 的祖先检查、Manifest 本体真实文件检查、init 路径调用与测试。
- `src/core/project_links.rs`：link/status/unlink/clean/doctor 及 group 路径调用、删除重复 cwd 目录校验、真实祖先逃逸集成测试。
- `src/core/db.rs`：移出全局路径类型/解析及其测试；保留原有数据库行为和 Windows 默认备份目录覆盖。
- `src/core/harness.rs`、`src/core/registry.rs`、`src/core/linkable.rs`、`src/core/symlink.rs`、`src/commands/mod.rs`：必要 import、映射路径安全解析、canonical 路径的存储/展示/比较调用同步。
- `src/core/test_support.rs`、新增 `tests/project_discovery.rs`：统一 canonical 测试路径；真实 symlink 夹具通过默认 Provider；CLI 子进程 cwd 回归测试。
- 模块契约 `02`、`04`、`05`、`06` 与本下钻文档。

其中 `db.rs`、`harness.rs`、`project_links.rs`、`registry.rs` 和模块 `04` 在本步开始前已有用户修改；仅以基线差异划定本步修改。根 README、CLI 参考、发布/安装文档、打包脚本、架构入口及其他重构计划文件的已有修改不属于本步；本执行会话未编辑重构计划 README。

### 验收标准覆盖证据

| 标准 | 实际证据 |
| --- | --- |
| 非法相对路径与唯一性 | 临时 Manifest 测试覆盖空值、绝对路径、盘符、`..`、重复及 `./` 形式；Windows 实跑覆盖大小写重复。统一入口另覆盖 `./C:...`、嵌套盘符组件，防止逐组件组合时切换盘符。 |
| 真实祖先 symlink 与外部内容保护 | Windows Broker 和 Linux Std 真实创建 symlink；`.agents`、链接父目录、相对 source 父目录三类替换均验证 load/save/status/link/unlink/clean 失败。Provider 用 mock，受管理链接条目与外部文件内容均保持原样；另验证 Manifest 末级 symlink 不能用于读写，受管理链接末级仍允许为 symlink。 |
| 项目定位与 init 边界 | 临时目录验证最近 Manifest、符号链接目录停止；CLI 子进程以真实子目录为 cwd 运行 status、clean、unlink、link dry-run；覆盖 Resource `./assets` 与 `.`（使用独立 alias），同时确认与真实 source 冲突时拒绝且源内容不变；init 测试在子目录制造真实文件冲突，确认只创建子目录源结构且保留父项目 Manifest。 |
| 全局路径及规范化 | 迁移后的 Windows/macOS/Linux 路径、portable、安装 Receipt 优先级测试通过；最小 Receipt、无关字段、损坏和不可读数据均覆盖。Windows 实跑覆盖普通磁盘前缀、长路径、UTF-16 长度、UNC 和非磁盘前缀保留。 |

### 实际命令与结果

以下 Cargo 缓存命令均按根 AGENTS.md 直接提权；使用默认 Cargo cache，未设置 `.tmp-cargo-home`。

| 命令 | 平台与结果 |
| --- | --- |
| `cargo fmt --check` | Windows，最终退出码 0；`step01-format.log`（无输出即成功）。 |
| `cargo test --locked` | Windows，74 个单元测试及 2 个 CLI 测试通过，3 个需要现有 Broker 的平台项默认 ignored；退出码 0。日志 `step01-windows-tests.log`；其中 ignore 文案为改用 Broker 前的旧文案，最终源码已更新为 Broker 前提。 |
| `cargo test --locked -- --ignored` | Windows，改用现有 Broker 后首次并行运行 2 项通过、1 项夹具创建出现 `SERVICE_UNAVAILABLE`，尚未进入该项路径断言。日志 `step01-windows-symlink-tests.log`。 |
| `cargo test --locked -- --ignored --test-threads=1` | Windows，3 项真实 symlink 测试均通过，退出码 0。日志 `step01-windows-symlink-sequential.log`。 |
| `cargo test --locked -- --include-ignored --test-threads=1` | Windows，冻结源码最终完整复验：77 个单元测试及 2 个 CLI 测试全部通过，0 failed、0 ignored，退出码 0。日志 `step01-windows-all-tests.log`。 |
| `wsl.exe -d Ubuntu --cd /mnt/f/00_Workspace/AI_Agents/agent_linker -- bash -lc 'cargo test --locked --offline --target-dir /tmp/aglink-step01-target'` | Ubuntu WSL，冻结源码最终复验：77 个单元测试及 2 个 CLI 测试全部通过，0 failed、0 ignored；真实 Linux symlink 测试实际执行。日志 `step01-linux-tests.log`。使用默认 Linux Cargo cache，构建产物隔离于 WSL `/tmp`。 |
| `git diff --check` | 最终退出码 0。文档链接及目标文件存在性另已检查。 |

全部日志与 `step01-baseline/` 位于同一仓库外证据目录。

### 失败过程与遗留边界

- 首轮 Windows 测试因 canonical 输出去前缀后旧 mock/断言仍使用扩展路径出现 9 项失败；已将测试夹具与断言改为统一规范化路径，后续完整复验通过。
- 直接调用 Windows 标准库创建测试 symlink 曾报 1314。未更改 Developer Mode、用户权限或服务配置；读取现有服务确认 `WinSymlinksBroker` 为 Running 后，夹具改走项目默认 Broker Provider。真实 Windows 安全断言现已通过，不以 ignored 代替验证。
- 现有 Broker 的一次并发夹具请求曾返回 `SERVICE_UNAVAILABLE`；串行独立复验与最终全套串行复验均通过。Broker 并发可用性不是本步改动范围，独立验收应保留该环境观察。
- 未在真实 macOS 上执行；其默认目录解析以 `DbPathContext` 模拟覆盖。原子保存、Manifest 来源格式、Store 连接与 Schema 等后续步骤未推进。
- 回归测试夹具首次将 `.` 目标与项目根内真实 `source.txt` 同名，因正确的 `existing_real_file` 保护失败；已将成功路径改用独立 alias，并将同名情形保留为显式失败断言。
- 独立验收首次结论为 FAIL：合法 Resource `--target-dir .`、`./assets` 在 `prepare_link_parent` 累积开头 CurDir 后被安全入口拒绝。已仅在父目录遍历跳过 `Component::CurDir`，未放宽受管理末级路径规则，并补上述 CLI 回归。回归测试对 `.` 使用 `--as root-link.txt` 避开源文件本体；另显式保留真实源冲突拒绝断言。修复后重新执行 Windows 全套串行测试、Linux 全套测试和格式检查，最终日志记录该修复版本。
- 代码验收缺口：执行侧已修复上述 FAIL，等待独立会话复核；是否满足本步验收由独立验收会话判断。跨会话主动报告曾被自动审批拒绝（缺少可信的用户消息授权），本地交付和上述证据不受影响。

## 独立验收

2026-10-28，验收会话 `01a1842f-d52d-7763-ad7b-0c5c4f94621c`：最终结论 **PASS**。此前独立发现的合法 `.` / `./assets` 目标目录回归曾判 FAIL，执行会话修复并重新冻结后，以下复验针对最终版本完成。未勾选 README、未暂存或提交，也未推进下一步。

### 受验版本与阶段范围

- 独立核对根 AGENTS.md、计划通用要求、当前步骤及严格相关模块契约；逐项检查实际实现和测试断言。
- `step01-delta.json` 中 16 个阶段文件的当前 SHA-256 与冻结 `step01-final/` 快照一致；有原始副本的 15 项也与 `step01-baseline/` 的基线 SHA-256 一致。源码与测试在独立完整复验前后指纹一致，包含 `./C:...`、`safe/C:...` 拒绝与 CurDir 回归修复。
- 独立按排序后的源码/测试相对路径 UTF-8、NUL、原始文件字节重算阶段联合 SHA-256，得到 `c3da3eac027edb82a3ab892de1de046fecb3d68694cce839734573f6c28cd12f`，与冻结交付一致。
- 本步范围为 10 个既有 Rust 文件、1 个新增 CLI 集成测试、模块 02/04/05/06 及本记录，共 16 项。数据库 schema、连接策略、Manifest 来源格式及原子保存算法未推进。初始 15 个跟踪文件修改及 9 个顶层未跟踪条目不能整体视作本步变更；提交须使用基线增量，尤其重叠修改的 `db.rs`、`harness.rs`、`project_links.rs`、`registry.rs` 和模块 04，不能按整文件混入原有修改。
- 此独立验收区在执行会话明确交接编辑权后追加，因此本文件最终哈希会区别于执行侧冻结快照；源码、测试及其他阶段文件不因验收改变。提交者应将本验收区追加纳入文档增量，不因此重新打包基线其他文件。

### 逐项结论

| 验收标准 | 独立证据与结论 |
| --- | --- |
| Manifest 路径合法性与唯一性 | PASS。临时 Manifest 的非法路径加载/保存失败，原文件不变；相同路径及 `./` 等价形式判重，Windows 大小写重复实际执行。统一入口还执行了嵌套盘符组件拒绝测试。 |
| 真实祖先边界和外部文件保护 | PASS。亲自运行真实 Broker symlink 专项；检查测试确实对 `.agents`、链接父目录、相对 source 父目录分别调用 load/save/status/link/unlink/clean 并断言失败，外部 manifest、leaf、source 内容及 mock 管理链接条目均不变。另有 Manifest 末级 symlink 读写拒绝，以及管理链接末级允许为 symlink 的独立断言。实现中在访问这些外部受控路径前执行祖先检查。 |
| 子目录定位、init 与 symlink 上查边界 | PASS。最近项目发现及 symlink 目录停止测试实际通过；CLI 子进程真实 cwd 验证 status/clean/unlink/link dry-run 上查；init 在子目录创建源结构，父项目 Manifest 和用户真实冲突文件保持原样。doctor 及 group 入口经代码核对接入同一项目定位函数。 |
| 多平台全局路径、Receipt、Windows 规范化 | PASS。迁移后默认路径、显式数据目录、portable、安装 Receipt 优先级及最小身份通过；不依赖无关 receipt 字段，损坏和不可读内容失败。Windows 实际执行普通磁盘去前缀与长路径、UTF-16 长度、UNC、非磁盘前缀保留断言。macOS/Linux 默认目录通过上下文模拟覆盖。 |
| 格式和必要测试 | PASS。Windows 独立格式检查、默认测试及三个专项通过；修复后重新运行包含 ignored 的完整串行套件，77 单元测试和 2 CLI 测试全部通过，0 failed、0 ignored。 |

### 独立复现命令与结果

- `cargo fmt --check`：退出 0；修复后再次检查通过。
- `cargo test --locked`：74 单元测试、2 CLI 测试通过，3 项 Windows Broker 专项 ignored，退出 0。
- `cargo test --locked -- --ignored --test-threads=1`：3 项真实 symlink 专项全部通过，退出 0。
- 修复后 `cargo test --locked -- --include-ignored --test-threads=1`：77 单元测试及 2 CLI 测试全部通过，退出 0。Cargo 默认缓存命令均直接提权。
- 独立 CLI 脚本 `C:/Users/SinVC.Windy/.codex/visualizations/2026/10/28/01a1842f-d52d-7763-ad7b-0c5c4f94621c/verify-dot-target.ps1`：退出 0。在该验收产物目录建立隔离项目、隔离 SQLite，分别对 `./assets` 和 `.` 执行 dry-run、真实 Broker 建链、status、unlink；断言 dry-run 不创建叶路径或改 Manifest，真实叶为 SymbolicLink，状态为 `correct_symlink`，解绑成功且 source 内容始终不变。
- `git diff --check`：退出 0。审查中的 `git diff --no-index` 退出 1 表示发现预期差异，不是验证失败。

### 已解决问题与剩余边界

- 首次独立真实 CLI 复现：注册 `--target-dir ./assets` 成功，但 link dry-run 及 `--target-dir .` 失败。根因为父目录准备将开头 CurDir 单独交给要求非空实际叶组件的安全入口。最终只过滤父目录遍历的 CurDir，未放宽安全入口，现已独立验证修复；成功的根目录链接使用独立 alias，原 source 同名冲突仍拒绝。
- Windows 下这 3 项默认 ignored 仅表示依赖现有 Broker 的真实平台夹具，Unix 不忽略。本次已亲自执行所有安全断言，串行复验足以覆盖本步祖先路径安全；不会据此宣称 Broker 并发可用性已验证。执行侧一次并发 `SERVICE_UNAVAILABLE` 仍作为环境观察保留，无本步未解决代码缺陷或验收阻塞。
- Linux WSL 的执行侧最终日志显示 77+2 全通过，已读取核对；该 Linux 命令并非本独立会话重跑。未在真实 macOS 执行，不将模拟路径覆盖表述为 macOS 平台实测。
- 验收产物目录保存独立脚本、临时复现现场及 `verified-source-sha256.json`；执行侧目录保存基线、阶段 patch/json 和冻结文件。未更改用户真实链接、在役数据库、Broker 服务或系统配置。


## B 提交候选重建与重验（执行侧，等待候选独立验收）

2026-10-28：因旧未提交基线与本步交织，另建隔离 B 候选；上述原工作树 PASS 仅作历史，本节不继承其验收结论。候选基于 `369c5c9dcc01f14b6659ec43e0f8bd8646f3712b`，仅保留第 01 步与必要 Receipt 识别、四模块路径契约及统筹明确批准的 27 份导航/来源文档。不推进 02–22 实现。本次最终提交由 terra 执行，本执行会话未创建 commit。

候选及全部证据位于仓库外原证据目录：`C:/Users/SinVC.Windy/.codex/visualizations/2026/10/28/01a18422-2ff4-7492-a3e3-3cfaa38030c9/`。其中 `b01/` 为受验源码，`b01-git/` 为独立 index/object store，`b01-target/` 为 Windows 构建目录；`b01-candidate.json` 记录最终 tree、完整 src/tests 指纹与文件范围，`b01-head-to-candidate.patch` 可从指定 HEAD 独立重建同一 tree，`b01-head-hunk-ledger.json` 逐 hunk 标记 01/receipt/导航类别。独立验收追加本节后应仅更新报告文档、重算候选 tree，不因文档变化重新运行源码全套测试。

本候选保留路径词法规则、嵌套盘符拦截、真实祖先 symlink 边界、Manifest 末级真实文件要求、相对 source 处理、Windows 大小写判重、项目根上查、init 不上查及 CurDir 修复。相对原冻结源码，只排除旧 readonly API/查询调用、dbcheck 非 CREATE 探针、VACUUM/WAL 备份、Windows 安装根备份默认值及其测试、migration 错误包装和对应旧基线测试；详细排除 hunk 见 `b01-excluded-baseline-hunks.json`、`b01-candidate-to-original-final.patch`。模块 04 不带 ACL、只读查询或备份承诺，并同步 Receipt 优先于 LOCALAPPDATA 的旧 Windows 决策。三字段最小 Receipt 校验迁入 paths 后无重复路径类型或解析。

候选第一次测试时发现两处残留的旧 readonly 写锁夹具导致 `database is locked`（71 通过、2 失败），已严格按初始 baseline 相对 HEAD 的五段插入清理旧测试设置/preview 断言；未补入 readonly 实现或删除本步路径覆盖。该失败日志保留为 `b01-initial-unexcluded-test-failure.log`。

| 实际验证（均以候选为 cwd，默认 Cargo cache，缓存命令直接提权） | 结果与证据 |
| --- | --- |
| `cargo fmt --check` | 退出 0；`b01-fmt.log`。 |
| `CARGO_TARGET_DIR=<证据目录>/b01-target cargo test --locked` | Windows：70 单测和 2 CLI 测试通过，3 Broker 平台项默认 ignored；`b01-windows-default-tests.log`。 |
| `CARGO_TARGET_DIR=<证据目录>/b01-target cargo test --locked -- --include-ignored --test-threads=1` | Windows：73 单测和 2 CLI 测试通过，0 failed、0 ignored；真实 Broker 祖先逃逸测试实际执行；`b01-windows-all-tests.log`。 |
| `wsl.exe -d Ubuntu --cd <候选Linux路径> -- bash -lc 'cargo test --locked --offline --target-dir /tmp/aglink-b01-target'` | Linux：73 单测和 2 CLI 测试通过，0 failed、0 ignored；真实 Std symlink 实跑；`b01-linux-tests.log`。 |
| `b01-verify-cli.py`（默认明确指向 `b01-target/debug/aglink.exe`） | 18 条真实 CLI 调用通过：隔离 SQLite/项目、真实 init、子目录 status、`.` 与 `./assets` 的 dry-run/建链/解绑、真实文件 force 拒绝、source 保全和 init 子目录独立初始化；`b01-real-cli.json` 含可执行文件路径、SHA256、每条命令及退出码。 |
| HEAD → patch → 独立空 index 的重建检查 | `read-tree HEAD` 后应用完整候选 patch，`write-tree` 与受验树相同；不使用目录缺失状态构造删除。HEAD 的 `CLAUDE.md` 以 mode `120000`、相同 blob/target 保留；`b01-symlink-preservation.json`。 |
| 原工作树保全 | 原 HEAD/index 未变；82 个初始文件中仅统筹授权的计划 README 更新，其余字节不变；`b01-original-worktree-preservation.json`。 |

计划 README 已同步统筹最终冻结版本，SHA256 为 `28be1d3b7f5b4943b37d188d03ec1a2ab4a61912d62d4608c1fc2a6e1937d8e7`；停止条件是 01 成功提交后暂停，不启动 02。27 份来源文档保留原文；本节为候选专属新增执行记录。计划 13/22 中未来 fixture 与未跟踪中文 CLI 的引用不因本候选扩大实现/提交范围，不宣称全仓链接闭合。真实 macOS 未运行，仅保留平台路径模拟覆盖。最终候选 PASS 须由独立验收在下方另行追加。


### B 候选独立验收摘要（2026-10-28）

依据独立验收会话 `01a1842f-d52d-7763-ad7b-0c5c4f94621c` 的独立报告，B 候选源码、范围及必要测试 **PASS**；最终提交树仍待该会话核对并单独确认，不能用原工作树 77+2 的历史 PASS 代替。报告原文：`C:/Users/SinVC.Windy/.codex/visualizations/2026/10/28/01a1842f-d52d-7763-ad7b-0c5c4f94621c/b01-independent-acceptance.md`。

独立验收在候选 cwd 使用本会话专属 `b01-audit-target` 重新构建，`cargo fmt --check` 与 `cargo test --locked --target-dir <独立验收目录>/b01-audit-target -- --include-ignored --test-threads=1` 均退出 0：73 个单测、2 个 CLI 集成测试通过，0 failed、0 ignored；3 项真实 Windows Broker 安全测试均实际执行。真实 CLI 脚本显式指向该候选专属二进制，分别对 `.`、`./assets` 验证 dry-run 不建叶、不改 Manifest、真实 SymbolicLink 创建、correct_symlink 状态、解绑及 source 内容保全。独立二进制 SHA256 为 `0e28a25f53f3d3816fd58e3b82a2b15f886792561c3c15f792e1624afebe0e1d`。

独立复验前后所有 src/tests 指纹一致，完整联合 SHA256 为 `1bd699cc8170fada567610fbc54cbd7a2ad9cfe62b4f9e003f9e7aec71989f33`。独立范围核对确认原 readonly、dbcheck、快照备份、默认备份目录、迁移错误包装及相关测试已排除，第 01 步安全修复与 CurDir 回归修复保留。反思检查无其他源码阻塞。最终树需核对 42 文件批准范围、最终 README、候选本节及 HEAD `CLAUDE.md` 的 mode `120000` / blob `93d17ee607e3086f248831776f9c6db0d3cc9cf3` / 原 target 保留；该树确认由独立验收另行报告。源码与测试不再修改。
