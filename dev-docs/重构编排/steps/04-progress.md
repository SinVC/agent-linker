# 04 · Symlink 原子替换与断链 force 进度

## 启动交接记录（历史保留）

- 授权：用户已授权继续下一步，仅 04；05 未授权、未启动。当前仅完成启动准备，尚未实施或验收。
- 任务入口：[计划 README 第 14 行](../../重构计划/README.md#L14) → [04 下钻](../../重构计划/04-symlink-replace.md)。按下钻契约及验收执行，必要调用点、测试与契约同步均限本步。
- 固定代码基线：`0cea138c7cd8dca736d1b16e4055beb9d731d5e2`，已由 Git 元数据核实为 `codex/manifest-origin-save-02-03` 及旧隔离 worktree 的 HEAD；包含已独立 PASS、本地提交的 02/03 实现。其 parent 为 `f583cc8c3daded38bd47f8b003e7a284ee60bb45`；最新提交仅更新 02/03 结案文档。
- 隔离要求：待从上述固定 HEAD 新建 04 隔离分支/worktree，保留旧交付 worktree。主树 `9415577f5d647b7de989d0ab9253089ac76ba687` 仅含协调元数据，不能作为 04 代码起点；主树原有未提交修改不导入。
- 元数据交接：只导入本轮授权的 `dev-docs/重构计划/README.md`、`dev-docs/重构编排/strategy.md` 及本文件；按统筹提供的 SHA256 核验。全局索引仅在主树维护，执行会话无需读取。导入后检查差异范围，验收锁定最终候选树。
- 执行会话：待分配（新独立聊天，`gpt-6.1-sol` / `high`）；交接只给 README 第 14 行及本文件路径。
- 独立验收会话：待分配；需核实实际差异、故障注入与平台实跑证据，执行自述不代替 PASS。
- 当前受验版本／证据：待执行后登记；尚无 04 实现差异或实跑结论。
- 验证阻塞／限制：启动准备未运行代码验证；待执行会话读取实际项目配置和根 AGENTS.md 后运行本步要求的检查。未运行的平台必须记录限制，不以 mock 代替平台结论；暂无已确认的其他阻塞。
- 本地提交：待独立 PASS、确认无阻塞后由 `gpt-6.1-terra` 子代理提交；核对提交树与最终受验版本一致，默认不 push。完成复选框由指定统筹文档维护者在独立验收 PASS、必要验证及本地提交成功确认后更新，执行会话不提前勾选。

## 执行交付（2026-10-29，待独立验收）

- 当前状态：04 实现与必要验证已完成，候选冻结等待独立验收；本记录不是独立 PASS。未启动 05，未提交、未 push，未勾选 README 04。
- 执行会话：`01a1873a-affc-7960-b0ee-94459e564a2a`；独立验收会话：`01a18740-590d-7003-9c43-bb5e1b741fd0`。
- 隔离树：`C:\Users\SinVC.Windy\.codex\worktrees\symlink-replace-04\agent_linker`；独立分支：`codex/symlink-replace-04`；HEAD 仍为固定代码基线 `0cea138c7cd8dca736d1b16e4055beb9d731d5e2`。保留旧 02/03 交付树和分支，未合回主树。
- Bootstrap 起始差异只有三个授权文件：计划 README（SHA256 `6e5b060b98bc87fa3a93ca4f8b14cd65ef6fe3c755c12504c390045b8b9f8b35`）、strategy（`62a91d374747ce7b6107901a41c0898137e9f2bf5497d4dad1aec3bdb36708e2`）、本进度初始副本（`2eebf229eabfe63b93d49c3c1f99d9eb96030ece6eb433f833d40288d0812ba4`）。先核哈希再导入；README/strategy 导入后未编辑，本文件仅追加本步交接。全局索引未导入、未读内容、未写入。

### 实现增量与契约

- `src/core/symlink.rs`：Provider 新增 `replace_symlink`；ensure 在 force 下委派错误目标/断链替换，不再先删除。Std 同目录创建临时 symlink，再 rename，失败仅清理自身临时链接；Broker 普通创建参数 false、替换参数 true，保留结构化错误码；Mock 新增一次性替换故障注入，变更前验证新 source 和旧 symlink。
- `src/core/project_links.rs`：同步 dry-run 的 BrokenSymlink force 分支；仍保留现有判定结构，共享纯函数和选项简化不在本步范围。
- `dev-docs/modules/01-symlink-core.md`、`06-cli-command-surface.md`：同步 Provider、原生替换、source 校验、force 与 dry-run 契约。
- `dev-docs/重构计划/04-symlink-replace.md`：更新符号行号与当前状态分类事实，动态入口改指本文件。
- 状态口径保持：缺失的旧目标与新 source 不同仍为 WrongSymlinkTarget；期望 source 自身缺失为 BrokenSymlink，force 仍不能跳过新 source 校验。

### 验证与平台证据

证据目录：`C:\Users\SinVC.Windy\.codex\visualizations\2026\10\29\01a1873a-affc-7960-b0ee-94459e564a2a\step04`。全部 Cargo 命令在隔离树显式 cwd、使用默认 Cargo cache 并直接提权；未创建 `.tmp-cargo-home`。

- `cargo fmt --check`：退出 0，`fmt-check.log`。
- Windows `cargo test --locked`：82 单元测试通过、4 Broker 专项默认 ignored；2 CLI 集成测试通过，`windows-tests.log`。
- Windows `cargo test --locked -- --ignored --test-threads=1`：4/4 通过，`windows-broker.log`。已向统筹明确释放 Broker 排他时段，验收可串行重跑。
- Broker 新增真实断言：file/directory × 活链/断链，在无 force 时拒绝、有 force 时原生替换成功；请求 false 得到 `LINK_ALREADY_EXISTS`，请求 true 对真实文件/目录得到 `LINK_PATH_IS_NOT_SYMLINK`；稳定错误码经映射仍保留。缺失新 source 拒绝且旧目标不变。另有 fixture 覆盖 `UNSAFE_REPARSE_POINT` 与 `REPLACEMENT_PARTIALLY_COMPLETED` 映射。
- WSL Ubuntu（Linux 6.18.54.1）执行 `CARGO_TARGET_DIR=/tmp/aglink-step04-target cargo test --locked`：85 单元测试与 2 CLI 集成测试通过，0 ignored，`unix-tests.log`。真实 Unix Std 覆盖 file/directory 的同目录 rename、断链替换、无临时文件残留、缺失新 source 保留旧链，以及真实文件/含数据目录保护。该日志后源码仅有一行测试注释更正，最终源码的平台专项复跑见 `unix-final-symlink.log`。
- Mock 与临时项目数据库/Manifest：活链与断链各自无 force 拒绝、有 force 成功；一次性替换失败保留旧链接；缺失或类型错误的新 source 拒绝；真实文件/目录受保护。项目 dry-run 与实际执行对上述矩阵判定一致，dry-run 前后链接及 Manifest 字节不变；项目替换故障也验证 Manifest 字节不变。
- 文档链接存在性、`git diff --check` 均通过。源码冻结 SHA256：`src/core/symlink.rs` = `d825dfe93d37d76f18a113f44c5e80f2770f5e2cf8b3fd60a7d7c0118e0367fe`；`src/core/project_links.rs` = `4d3eef78ab111f58cfa76f7b804356aa3195b7ca0b8573b9fc900d534c31663e`。完整候选文件哈希见证据目录 `candidate-files.json`，受验实现相对于固定 HEAD 的补丁见 `candidate.patch`（本进度单独保存副本）。

### 主树保全与限制

- 主树只读快照：`main-before.json`（创建于 `2026-10-29T10:36:00.3197615Z`）、`main-after.json` 和 `main-comparison.json`；主 HEAD `697317285f65a28dba276e75b58be3806dea5b56`、index SHA256 `fa68a3b5ced6a41c20f4e16c32c037dd41fcdd641a9a1716f485edb7ef602731` 均保持不变。88 项 tracked/untracked 中，87 项无差异，唯一变化为统筹维护的全局索引协调例外。未 stash/reset/覆盖/合并主树文件。
- 全局索引起始 SHA256 为 `fee85252a0cc7e7b26daf38358d659024afdc4058bcca8a76fe5258b91002173`；中途统筹发现其旧字节回写并独立处理，执行核查未发现本人写入该路径的操作。最终快照该路径 SHA256 `8403984244020bc13300d5769293100d4c9590cf33efd83a14a6fca35dd21046`；只核哈希，未读其内容或其他步骤进度。
- 无已确认验证阻塞；Windows 默认 Broker 与 Unix Std 均有真实平台结果。Windows 显式 Std fallback 替换未专项实跑，不以默认 Broker 测试代替；其 rename 不支持覆盖时返回错误并保留旧链。Broker 的部分完成错误仅覆盖稳定映射，没有主动制造服务内部部分完成故障。
- 冻结后不再改源码或候选文档；等待独立验收反馈。PASS、必要验证成功且无阻塞后，才由指定 `gpt-6.1-terra` 子代理按验收锁定版本本地提交；该提交尚未发生。

## 独立验收（2026-10-29，04 PASS）

- 验收会话：`01a18740-590d-7003-9c43-bb5e1b741fd0`。执行方明确冻结并释放 Broker 后开始实跑；只在指定 `C:\Users\SinVC.Windy\.codex\worktrees\symlink-replace-04\agent_linker` 验收，分支 `codex/symlink-replace-04`、HEAD `0cea138c7cd8dca736d1b16e4055beb9d731d5e2` 均核实。没有在主树验旧实现，没有修改实现、提交或 push，没有推进 05。以下结论针对本次冻结候选，不以执行会话结束状态代替验收。
- 结论：04 **PASS**，无已确认代码缺陷或阻塞。Windows 默认 Broker 与 Linux/WSL Std 均有独立真实文件系统结果；允许交指定 `gpt-6.1-terra` 子代理按锁定候选本地提交。README 完成框仍由统筹处理，本会话未修改。

### 逐项证据

独立证据根：`C:\Users\SinVC.Windy\.codex\visualizations\2026\10\29\01a18740-590d-7003-9c43-bb5e1b741fd0`。

1. 错误目标/断链的 force 与 source 校验：独立程序 `real_matrix.rs` 链接受验库并调用受验 CLI，分别在 Windows Broker 和 WSL Std 实跑文件/目录两种源 × 活链、断链、缺失源、错误 kind、真实文件、含用户数据的真实目录 × force false/true，共每平台 24 组，每组执行 dry-run 和实际命令。仅有效源的活链/断链在 force 下成功；其余拒绝。Windows `windows-real-matrix.log`、Unix `unix-audit.log` 均为 24/24 PASS。实际解析链接目标等于新源；拒绝时原链接目标、真实文件/目录中的 `user data`、Manifest 原始字节保持不变。
2. 替换故障不丢旧链：独立执行 `cargo test --locked replacement -- --nocapture`（Windows 2 PASS、1 Broker 默认 ignored；Unix 3 PASS），以及 `cargo test --locked force -- --nocapture`（两平台各 7 PASS）。已审断言并实际执行：Mock 一次性替换失败返回 Io，旧 symlink entry/target 不变；项目失败保留旧 entry 与 Manifest 字节；新源缺失或 kind 错误拒绝；真实文件和目录均保留。证据 `windows-replacement.log`、`windows-force.log`、`unix-audit.log`。
3. 项目 dry-run 一致性：上述独立真实矩阵逐组断言 dry-run 与实际执行成功/拒绝一致；错误 stderr 相同；每次 dry-run 后链接目标及 Manifest 字节不变。成功执行产生新源链接与 Manifest 记录。临时项目和数据库逐组隔离，无用户项目写入。
4. Windows 原生请求与错误码：独占串行执行 `cargo test --locked broker_native_replacement_and_error_codes -- --ignored --test-threads=1`，1/1 PASS，`windows-broker.log`。审过并实际执行 file/directory × live/dangling 的原生替换断言；普通请求 false 返回 `LINK_ALREADY_EXISTS`，替换 true 对真实文件/目录返回 `LINK_PATH_IS_NOT_SYMLINK`，映射后稳定码保留；有效替换最终状态 CorrectSymlink。独立真实项目矩阵亦经默认 Broker 创建与替换，没有 fallback。
5. Unix 实际同目录 rename：独立 `audit_unix.sh` 使用默认 Cargo cache、`CARGO_TARGET_DIR=/tmp/aglink-independent04-target`，运行上述测试和 `cargo build --locked --bin aglink` 后，以 `strace -f -e trace=symlink,symlinkat,rename,renameat,renameat2` 执行真实矩阵。`unix-rename.trace` 记录四组有效 force（file/dir × live/dangling）均先在目标 `assets/` 内创建 `.aglink-symlink-<pid>-0.tmp`，再 `rename(同目录临时链接, assets/asset) = 0`。不存在先删除旧目标再创建的流程。源码在脚本起止的 SHA256 相同。
6. 必需检查与契约：独立 `cargo fmt --check` 退出 0；`git diff --check` 通过；16 个当前修改文档的相对链接目标存在，代码锚点与符号匹配。复核交付完整日志：Windows `cargo test --locked` 为 82 unit + 2 CLI PASS、4 ignored；Broker 串行全套 4/4 PASS；Unix 为 85 unit + 2 CLI PASS，最终源码另有 Unix 替换专项 1/1 PASS。最终版本未变，独立验收按范围复跑上述断言及真实矩阵，没有无必要重复全套。

### 锁定范围、指纹与保全

- 实现增量仅两个 Rust 文件和模块 01/06、04 当前下钻；另有三个已授权 bootstrap 文件（其中本步 progress 为动态文件）。README/strategy SHA256 与导入值完全一致，没有把 bootstrap 误判为实现越权。未读其他 step 或 global 内容；没有向主树导入候选。
- 所有 `src/`、`tests/` 文件的 SHA256 和规范化 Git blob 已独立记录到 `src-tests-frozen.json`，该清单 SHA256 = `2610B5C6C9B5A6CA5237B6D86A5466B7E21A767EFBF2E1D7DA2875A6C4B8A5AF`；实跑后逐文件核验无变化。两个变更源文件为：`src/core/symlink.rs` SHA256 `D825DFE93D37D76F18A113F44C5E80F2770F5E2CF8B3FD60A7D7C0118E0367FE` / Git blob `a0aa3abe8201a1595a7e7230e3b8327e9183df6e`；`src/core/project_links.rs` SHA256 `4D3EEF78AB111F58CFA76F7B804356AA3195B7CA0B8573B9FC900D534C31663E` / Git blob `15be96bf225fecde3ffe8dd5d5e809c8a921b591`。`tests/` 相对基点无改动，新增断言位于两个源文件内部。
- 核实执行冻结清单 `candidate-files.json`（SHA256 `CE81DBDD13397EC256076511A20F9CD3B16CCAAA73EF328BE6E105541D1E3520`）逐文件完全匹配；原 `candidate.patch` SHA256 `B21AFEF74B4A3F6EE8FC0EC5EF4C9947611FB8B200C8297EFCC2C974F2F5D95F` 已核实。以固定 HEAD 应用该补丁，再放置本次最终 `04-progress-final.md` 副本可重建候选及独立验收区；本验收证据根另保存相同 tracked 差异的 `candidate.patch`，SHA256 `4F64D749988E97102E095E5047918794C3BE4819B7522831E638CFF4D72E6CA6`。
- 独立读取执行起始主树快照并与当前主树逐项比较，排除唯一授权 global 协调路径：87 个受保护 tracked/untracked 路径内容与路径集合均无变化，主 HEAD `697317285f65a28dba276e75b58be3806dea5b56` 与 index 哈希不变。证据 `main-independent-comparison.json`；没有 stash/reset/覆盖用户文件或旧 02/03 交付。全局索引仅作既有明确例外，未调查或读取其内容。

### 平台限制与交接

- Windows 显式 Std fallback 未专项实跑；本步要求 Windows 默认 Broker 原生替换和 Unix Std rename，二者均已独立真实验证，故该非默认后端限制不阻塞本步。没有用默认 Broker 成功宣称 fallback 已验证。
- Broker 服务内部 `REPLACEMENT_PARTIALLY_COMPLETED` 未主动制造；其稳定码映射为 fixture 覆盖。计划明确要求 Mock 注入失败保留旧链，本次已实际执行；不将 fixture 扩称为服务内部故障实跑，也不将其列作本步未完成条件。macOS 未运行，Unix 实测平台为 WSL Ubuntu。
- 初次 Unix 脚本未加载 Rust PATH，加载 `$HOME/.cargo/env` 后全部成功；受验源码未改。无剩余环境阻塞。验收 Broker 时段已经完成并释放。
- 首次向执行会话的协调消息被自动审批拒绝，理由为委派文本未被视为直接用户消息授权；消息未发送，未重试或绕过。依照交接要求由统筹读取本记录/会话中转，不影响代码验收。后续本会话只追加本独立验收区，停止写入，交由指定提交代理。

## 04 结案（2026-10-29）

- 状态：04 独立验收 PASS，必要验证成功，无已确认阻塞；指定 terra 已完成本地实现提交 `1a9a207e40b12a9219d32c1b49a5c4205ba594c2`，parent `0cea138c7cd8dca736d1b16e4055beb9d731d5e2`，tree `251c7b1cdc9db9094e3edb78ba3c1f670b78849a`。结案追加前只读复核 Git 元数据吻合、隔离工作树 clean、本进度仍为已验收 SHA256 `890CBA8B2CCD25C7473BF000D0CB2EB7164B7B5355322EFB9BF5360639742587`。
- 交付位置：分支 `codex/symlink-replace-04`，路径 `C:\Users\SinVC.Windy\.codex\worktrees\symlink-replace-04\agent_linker`。既有冻结范围与独立验收结论保持；实现未 push，源码继续冻结，本次只追加结案，不读取源码、不重跑测试、不提交。
- 必要验证沿用前述证据：fmt、完整 cargo test、Windows Broker 串行实际替换/错误码、Unix 同目录 rename，以及两平台各 24 组真实项目矩阵和 Mock 故障旧链/Manifest 保留均通过。Windows 显式 Std fallback、macOS、Broker 内部部分完成故障未实跑，仍为前述三项非阻塞限制，不扩大已验证结论。
- 本步收尾止于 04，05 未启动。README 完成框与主树唯一 step 文件同步由指定元数据维护者处理；本步自动跟进将由统筹关闭。
