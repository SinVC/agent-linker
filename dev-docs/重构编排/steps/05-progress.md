# 05 · 注册项标识符与 alias 查询空间隔离进度

- 授权：仅 05；06 未授权、不启动。本文件为本轮执行与独立验收的唯一进度交接入口。稳定策略仅保留通用规则，本轮范围以最新用户授权及当前步骤记录为准。
- 静态任务入口：`dev-docs/重构计划/README.md:15` → [05 下钻计划](../../重构计划/05-item-identity.md)；验收入口为该下钻 `:24`（标准 `:26–29`）。执行会话自行读取根 AGENTS.md、架构入口及该步条件文档，不读取全局索引或其他步骤进度。
- 当前状态：实现与必要验证完成，已冻结交独立验收；未提交，执行完成不代表独立 PASS。README 保持 01–04 `[x]`、05 及以后 `[ ]`，待独立验收 PASS、必要验证及本地提交成功后由指定统筹维护者处理。

## 固定代码基线与隔离

- 来源分支：`codex/symlink-replace-04`；已只读核实固定提交 `514f9ab48bed043fe1376af5a95934b2fc90a8cf`，tree `788ef85c134b36f65d6603357fbc2b85b41fd66c`。
- 该提交为 04 结案文档提交，直接 parent 为已审实现 `1a9a207e40b12a9219d32c1b49a5c4205ba594c2`；已核实包含该实现与前序已交付代码。保留原 02/03、04 交付树。
- 新执行会话从上述固定提交新建隔离 worktree/分支，记录实际路径、分支和 HEAD；不要从主树 `main@dfdd39f207709f10609c8818c972214a247edf02` 开始，主树只有协调文档提交及用户未提交基线。
- 本轮启动元数据仅三路径：主树全局索引、清理旧轮动态文字的稳定策略、新建当前 05 进度。新隔离树仅导入两份已审文件：`dev-docs/重构编排/strategy.md` 与 `dev-docs/重构编排/steps/05-progress.md`，导入前核对统筹交接 SHA256；不导入 global、README 或其他 step。代码基线已含正确 README 完成复选框与隔离约定；旧策略以本轮已审通用规则副本替换。
- 主树保全快照：`C:/Users/SinVC.Windy/.codex/visualizations/2026/10/28/01a18414-be15-7533-8e09-04f9b9971ffc/step05-recovery-20261030/`，包含 HEAD/index/status、88 个 tracked/untracked 路径 SHA256/链接元数据；原 16 修改/4 未跟踪保留，`.tmp-cargo-migration` 本轮启动时不存在，来源未证实的旧并行缓存信息不作为删除许可。不得 stash/reset、覆盖用户内容或混入其他阶段。
- 全局索引仅主树权威、由统筹唯一维护者写前 SHA 守卫维护；执行/验收不得读写 global。执行成果、必要契约同步与证据仅留当前新隔离树及本文件，不合回脏主树代码。

## 会话与执行

- 执行聊天：`01a19093-6f3b-7010-8857-5e731f7ca071`（新独立聊天）。
- 独立验收聊天：待分配（按当前步隔离）。
- 实际隔离 worktree：`C:/Users/SinVC.Windy/.codex/worktrees/item-identity-05/agent_linker`；分支 `codex/item-identity-05`；起始及当前 HEAD `514f9ab48bed043fe1376af5a95934b2fc90a8cf`。主树本轮实际 HEAD 为 `51b9b472937609425ae935798c65034a24feb3bf`（前述旧 main ref 仅保留启动交接历史，不作为代码基线）。
- 本轮修改：`src/core/registry.rs` 仅删除两个共用 SQL 查询的 alias 匹配，并增加两个临时数据库回归测试；`src/core/project_links.rs` 仅增加 mock 项目回归测试。契约同步 `dev-docs/modules/05-linkable-item.md`、`06-cli-command-surface.md`；当前下钻计划仅修正代码行引用与进度入口。`strategy.md` 为精确授权 bootstrap 副本，无额外扩写。

## 验证、受验版本与阻塞

- 验收标准：采用 05 下钻计划 `:24–29`；本文件不改写标准。
- 验证命令与实跑证据：见下方冻结证据；全部使用上述隔离树绝对 cwd，默认 Cargo cache，Cargo test 直接提权。
- 精确受验 HEAD / tree / 文件指纹：见下方冻结证据；payload tree 排除本动态进度文件，便于避免自引用。
- 独立验收结论与问题：待验收；执行自称完成不作为 PASS。
- 当前阻塞：无技术阻塞；等待独立验收。尚未独立 PASS，不启动 terra 提交。
- 平台限制 / 未实跑项：4 个既有 Windows Broker 专项测试按默认标记 ignored；本步为 Registry 身份隔离，已由临时 SQLite 和 mock 项目实际验证，未请求或占用 Broker。
- 本地提交：待独立验收 PASS 且必要验证完成后，由指定 terra 子代理提交；记录提交、parent、tree 与受验一致性，默认不 push。

## 2026-10-30 执行冻结证据

- 授权严格止 05；未读取其他 step 进度，未修改 README、迁移、schema、默认名算法、rename 生产逻辑或任何后续阶段。没有新增唯一约束或扫描。没有 git commit/push，真实 Git index 未暂存实现。
- Bootstrap 核验：strategy SHA256 `7FF982D6AFB9B7F659DA96B0D6408B27BB794F3A40860FD8A4AAF69BC8AFD4C6`，05-progress 初始 SHA256 `D489E0E4CC5C7619F04AD61B05E33DF81702882F8C2E086B03259A67DA87103E`，复制前均一致；只导入这两个文件。
- 行为证据：`registry_identifiers_ignore_aliases_for_both_item_types` 在两类临时数据库中验证 alpha/alias beta、另一个 beta/alias gamma 的 show、rename、refresh、remove、group add/remove 选择；只有 alias 时所有相关操作失败，精确 id 仍可查。`alias_of_another_type_does_not_make_name_lookup_ambiguous` 覆盖跨类型 alias/name 重叠。`link_identity_alias_and_rename_preserve_existing_project_state` 对两类 mock 项目验证 link beta 正确、alias-only 拒绝、id 可 link、默认路径遵循 alias；rename 不变 id、alias、Manifest 字节、既有链接路径/目标及源内容；随后 item_id 解绑、--as 覆盖且不改 alias、分组按 id 解绑成功并保留 alpha。既有默认 Skill/Resource 命名和冲突测试继续通过。
- `cargo fmt --check`：成功，exit 0。`cargo test --locked`：最终 exit 0；85 库测试通过，4 既有 Broker 专项 ignored；2 集成测试通过；main/doc tests 0 失败。`git diff --check`：exit 0。首次测试新增 fixture 少建 mock 父目录，84 pass / 1 fail / 4 ignored；仅补 mock `.agents/skills`、`docs` 后完整复跑成功。
- 全部外部证据目录：`C:/Users/SinVC.Windy/.codex/visualizations/2026/10/31/01a19093-6f3b-7010-8857-5e731f7ca071/`。首次失败日志 `cargo-test.log`；最终日志 `cargo-test-final.log` SHA256 `014E109B75E7785DC9EB84C7B9C7C707962086CAD432B0C55A4B965A6CC0D2C4`。
- 受验 payload tree：`06c0a32a9c0ee296248ef52300b88a97d694f8e8`（固定 HEAD 加 6 个受验 tracked 差异，包含授权 strategy bootstrap；排除本进度文件）。通过外部 `GIT_INDEX_FILE=candidate-payload.index` 执行 `git read-tree HEAD`、精确路径 git add、git write-tree 生成，退出后移除该环境变量；未修改真实 index。
- 可重建 payload：证据目录 `candidate-payload.patch`，SHA256 `6CEE473C1A4FCD674077137B6A765752804240CD56756FAEEF8D58835CC5B95E`；在固定 HEAD 应用该 binary patch，再复制冻结的本进度即可重建完整候选。外部另存 `05-progress-frozen.md`、`full-candidate.patch`、`full-candidate-tree.txt`；完整 tree 不回填自身以避免自引用，验收时以这三份精确内容核对。
- 最终 src/tests 全清单：`src-tests-sha256.json`，SHA256 `81A27792B6D3D627886AE1EAE59454E3BC4076300B9EA7EDA6721F6125AB49BB`；变更文件 `src/core/registry.rs` SHA256 `4E69952D2CD1B82FA7D62645845543B5F8D2ADF778164D5D9318573DDC441F1B`，`src/core/project_links.rs` SHA256 `026814E77A0B78C29CD758BB09A58FC13044AD4CC106876497FCF2F7A7D81AFC`，其他 src/tests 与固定基线一致。
- 主树保全：外部 `main-before.json`（SHA256 `A875A106FAEE926F2ABB24703725C11FD686B033013F3BD802E9E4E2148846EC`）、`main-after.json`。前后主树 HEAD/index 相同；88 个非 global tracked/untracked 路径 SHA256、链接属性/目标及非 global status 完全一致，保留原 16 修改/4 未跟踪；唯一 status 增量为统筹维护的 `dev-docs/重构编排/progress.md`，未自行恢复。`.tmp-cargo-migration` 前后不存在，本轮未造/删/移其文件。
- 范围偏差披露：初次快照的 global 排除表达式按文件名前缀匹配，遗漏实际名为 `progress.md` 的全局索引，因此初始全路径清单包含其 SHA256 项。没有将该文件正文读取到模型上下文、没有写该文件，已向统筹纠正先前“排除 global”的表述。后续快照与比对明确排除完整路径；无非 global 主树文件变化。
- 冻结后停止实现，交独立验收；验收/terra 提交由统筹另行协调。若需修改，先解除冻结并重新生成指纹、patch/tree，必要时复跑验证。

## 2026-10-30 独立验收 · PASS

- 独立验收聊天：`01a19099-5991-73c1-8666-eb8cc7dd4b5c`。已自行读取根 AGENTS、README 1–7/15、05 下钻及架构入口、模块 02/04/05/06/07；未读取 global 或其他 step 正文。执行方正式确认源码与当前进度停止写入后，才开始独立实跑与追加本区。授权范围止 05。
- 受验目录：`C:/Users/SinVC.Windy/.codex/worktrees/item-identity-05/agent_linker`，分支 `codex/item-identity-05`，固定 base / 验收前 HEAD `514f9ab48bed043fe1376af5a95934b2fc90a8cf`。所有验收 shell 显式使用此 cwd；真实 CLI 仅在外部专属临时 fixture 中运行，并显式设置独立 `AGLINK_DATA_DIR`，未在主树运行产品测试。
- 结论：**PASS，无未解决缺陷或必要验证阻塞**。已核对实际生产差异只有两个共用 SQL 的 alias 匹配删除，show/rename/remove/refresh、统一 link、group add/remove 的消费链全部归于这两个入口；默认名、alias、--as、rename 生产逻辑、schema 与迁移未改。两份模块契约及本步静态计划同步符合范围；strategy 原字节 SHA256 与明确授权 bootstrap 完全一致。README 与其他阶段没有差异。

### 完整标准与实际断言

1. `show beta`、`rename beta ...`、`refresh beta`、`remove beta`、`link beta`、group 成员 add/remove 均选中 name 为 beta 的记录：独立完整测试实跑了新增双类型临时 SQLite 和 mock 项目用例，并追查每项断言，确认 alpha/alias beta 与 beta/alias gamma 同时存在时返回的 id/source 为后者。另用受验真实 CLI 在 Skill、Resource 各一套临时数据库完成 72 次调用；group show、name/id 查询、rename/remove 结果及 link dry-run 的 source/path 与预期一致，alpha 保留。跨类型 alias/name 重叠测试通过。
2. alias-only 查询失败、精确主键 id 可用、alias 仍决定默认落盘名：双类型临时 DB 与 CLI 均实证 beta 只有 alias 匹配时 show/rename/refresh/remove/group/link 拒绝；id 查询/链接成功。mock 实际链接断言及 CLI dry-run 的 beta/gamma 路径通过；--as 生成 custom 并保持 Registry alias。既有 Skill/Resource 默认命名和目标冲突测试继续通过。
3. rename 后 id、原 Manifest 字节、原链接路径/目标和源内容不变，按 item_id 解绑及分组解绑正常：逐项检查并独立实跑 `link_identity_alias_and_rename_preserve_existing_project_state`，两类 fixture 均对实际 mock provider entries、临时 manifest 原字节和 source 内容作比较；随后 id 解绑成功、按新名称重新以 --as 链接、group 按成员 id 解绑并保留 alpha。CLI 另证 rename 后 id/alias、group 跟随 id、源内容及 manifest 不变。
4. 必要命令：独立 `cargo fmt --check` exit 0；`cargo test --locked` exit 0，**85 库测试 + 2 集成测试通过，0 失败，4 既有 Windows Broker 专项 ignored**，main/doc 0 失败；`git diff --check` exit 0。额外 `cargo build --locked --bin aglink --message-format=json` exit 0，受验 CLI 72 次临时数据库/dry-run 回归 PASS。所有使用默认 Cargo cache 的验证均直接提权。

### 平台判断、范围与保全

- 四个 ignored 分别覆盖真实 symlink 父目录/项目发现边界、项目操作 symlink 逃逸、Broker 原生替换与错误码。本步未改这些路径或 Broker 后端；身份选择通过临时 SQLite、真实 CLI 与 mock 项目充分覆盖。因此它们是未实跑的既有平台专项，不充作通过项，也不构成本步必要验证阻塞；本验收未占用 Broker。
- 独立核对执行冻结的 7 个文件原字节 SHA256 / Git blob、全部 17 个 src/tests 指纹。完整 patch SHA256 `9308B783603F11379BBC75CE2479F4B10D2829CCE416AADB71720B98647A21E9`；独立临时 index 从固定 base 实际应用该 patch，重建完整 tree `d51d8e386f84235d55a83f6770bc6f21c690b763`，真实 index 未变。排除动态进度的 payload tree 仍为 `06c0a32a9c0ee296248ef52300b88a97d694f8e8`。本区追加后完整 tree 必然变化，最终 accepted 完整 tree、可重建 patch 和含本文件的 mode/blob/raw SHA256 清单统一存外部证据，避免自引用；不得将旧完整 tree 当作最终提交树。
- 独立读取 main-before/main-after 元数据并现场只读复核主树：HEAD `51b9b472937609425ae935798c65034a24feb3bf`、index SHA256 `EA88E0EDB3D62476E8E886E2EB3514EEE92DEC8227262DB0F624B7E1B21DC848` 与执行前一致；88 个非 global tracked/untracked 路径原字节 SHA256、属性、LinkType/Target 和非 global status 全部一致，16 修改/4 未跟踪保留。global 的授权维护为唯一 status 例外，未读取正文或自行恢复；`.tmp-cargo-migration` 不存在，未造/删/移。
- 已独立证实初始 main-before 确实包含 global SHA256 元数据项，不能说“首次已排除”；后续比较使用精确相对路径排除，共 88 个保护路径。本验收未把 global 正文或其他 step 进度读入上下文。
- 独立审计脚本首轮将既有 CLAUDE.md symlink 错当应不存在，修正为对照原 LinkType/Target/属性；CLI 首轮断言漏开 --verbose 取不到路径细节，按实际输出契约修正并在全新 fixture 完整通过。均为外部验收脚本问题，没有修改候选实现或放宽产品标准。

### 独立证据与冻结交接

- 独立证据目录：`C:/Users/SinVC.Windy/.codex/visualizations/2026/10/31/01a19099-5991-73c1-8666-eb8cc7dd4b5c/`。
- 完整测试日志 `step05-independent-cargo-test.log` SHA256 `421E51FEE938E9A785EB7861B47825E474A7A13542888C56057BEAF907150839`；CLI 回归逐条命令/退出码/stdout/stderr `step05-cli-regression.json` SHA256 `73F23F5389F7CB410EF4AA79EAF48BD7A3BFAA41DFB1FF532AB5CB8DC15DE4BD`。fmt 成功无输出，Tee 未生成空日志，退出码 0 以本会话工具记录为准。
- 原冻结版本与主树保全核验：`step05-independent-preflight.json`、`step05-main-metadata.json`；独立重建 index：`independent-rebuild.index`。最终冻结交付：`step05-accepted.patch`、`step05-accepted-files-blobs.json`、`step05-accepted-progress.md`、`step05-accepted-tree.txt`；该清单包含本新增进度文件，逐项记录 Git mode/blob 与原字节 SHA256。
- 写完本区即冻结，不再修改源码、契约或本步进度。必要验证已成功、无阻塞，随后仅指定 `gpt-6.1-terra` 子代理在此隔离树暂存精确 7 路径，要求 staged write-tree 严格等于 accepted 完整 tree 后本地提交；禁止 push、改文件、修改 README/其他 step/global。提交及 parent/tree 一致性写外部 `step05-commit-receipt.json` 并向统筹报告，不回填本文件造成受验树漂移。

## 2026-10-30 结案

- 第 05 步独立验收 **PASS**，必要验证已完成，无未解决缺陷或验证阻塞；本结案仅追加交付事实，不改已审实现。
- 实现提交：`c5351ec79aadcfdf59da2b306f8be32e45567195`；parent `514f9ab48bed043fe1376af5a95934b2fc90a8cf`；完整实现 tree `6ee9ae23822519937f74808e8d3a31b9ae87f77a`。指定 terra 已本地提交，独立验收再次核对 parent/tree、7 路径 blob/raw SHA256 和隔离树干净，未 push。
- 交付分支：`codex/item-identity-05`；工作树：`C:/Users/SinVC.Windy/.codex/worktrees/item-identity-05/agent_linker`。实现源码保持冻结，本轮不重复测试。
- 必要实跑：`cargo fmt --check`、`cargo test --locked`、`git diff --check` 成功，85 个库测试与 2 个集成测试通过；额外真实 CLI 双类型临时数据库/dry-run 回归共 72 次调用通过。4 个既有 Broker 专项未改、未实跑，按前述独立相关性判断不构成本步阻塞，不算通过项。
- 前述快照口径更正完整保留：初始 main-before 包含 global 的 SHA256 元数据项，不能表述为首次已排除；未读取其正文到上下文、未写 global。独立提交后复核主树 HEAD/index、88 个非 global 路径原字节/链接元数据/status 保全，包含既有 CLAUDE.md symlink；global 仅为授权统筹维护例外。
- 范围止 05，06 未启动；README 完成复选框由 root 单一元数据维护者处理。本轮自动跟进于收尾后关闭，关闭操作由统筹统一完成。本会话只追加本结案区，不改 README、strategy、global、其他 step 或主树文件，不提交；结案文档提交由 root 另行统一安排，不回填未来文档提交自身 hash。
