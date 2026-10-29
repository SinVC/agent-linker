`group list` 本来只需要读数据库。你遇到的问题来自数据库入口把“打开连接、迁移 schema、写入默认数据”绑定在一起。整体审查还发现了几处比多余写库更严重的问题，包括可能删错注册项、批量操作部分成功，以及 manifest 路径越界。

本次没有修改项目文件。审查基于当前工作区及已有未提交改动；现有 **67 项测试全部通过**，另外用临时数据库和 mock provider 复现了关键问题。

1. **`group list` 写库的原因已定位，当前工作区已有修正。**  
   已提交版本的调用链是：
   ```text
   group list
     → RegistryStore::open
     → open_migrated_connection
     → 创建数据库 / 执行迁移 / seed 默认 Harness
     → 查询分组
   ```
   当前未提交改动已将查询改为 `open_readonly`。所以这个问题在当前源码中已有修正，但你实际运行的二进制是否包含该修正，本次没有验证。见 [registry.rs:203](F:/00_Workspace/AI_Agents/agent_linker/src/core/registry.rs:203)。

2. **【P1】普通写命令仍然隐式执行迁移和 seed。**  
   即使数据库已经是最新版本，仅调用 `open_migrated_connection`，也会执行三次默认数据写入。临时验证还确认：添加不存在的 Skill 失败后，数据库及默认数据却已经创建。连接打开、初始化、迁移、默认数据维护应有明确边界，业务操作失败不应留下这些无关副作用。见 [db.rs:402](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:402)。

3. **【P1】名称和 alias 共用查询空间，可能删除错误的注册项。**  
   可同时注册 `alpha --alias beta` 和 `beta --alias gamma`。此时 `skill show beta`、`skill remove beta` 使用 `id OR name OR alias` 查询并直接取第一条，实际可能选中 `alpha`；统一 `link beta` 却会报告歧义。**临时验证确认 `remove beta` 删除了 `alpha` 的注册记录。** 需要统一标识符解析及歧义规则。见 [registry.rs:313](F:/00_Workspace/AI_Agents/agent_linker/src/core/registry.rs:313)。

4. **【P1】分组数据库操作没有事务，批量失败会留下部分修改。**  
   `group add/remove` 先检查全部输入，再逐条提交 SQL。检查不能替代事务：在临时库中让第二条 INSERT 失败后，第一条成员记录仍然存在。应以一次命令作为事务边界。见 [registry.rs:466](F:/00_Workspace/AI_Agents/agent_linker/src/core/registry.rs:466)。

5. **【P1】manifest 没有验证链接路径必须位于项目内。**  
   `Manifest::validate` 检查了大量字段非空，却允许绝对 `link_path`、`../` 和不同 ID 对应相同目标路径。后续 unlink/clean 直接拼接并操作这些路径。**mock 验证确认，项目外的链接会被传给删除逻辑并删除。** 这是当前最应优先补齐的安全边界。见 [manifest.rs:57](F:/00_Workspace/AI_Agents/agent_linker/src/core/manifest.rs:57)、[project_links.rs:804](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:804)。

6. **【P1】路径安全检查在创建、读取和删除流程中不一致。**  
   创建链接会逐层检查父目录，unlink/clean 没有对应检查；manifest 只检查文件本身，没有检查 `.agents` 等祖先目录。父目录后来被替换为 symlink 时，操作可能沿路径进入项目外。应统一“项目内受管理路径”的解析和检查。见 [manifest.rs:196](F:/00_Workspace/AI_Agents/agent_linker/src/core/manifest.rs:196)、[project_links.rs:1054](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:1054)。

7. **【P1】文件系统与 manifest 更新缺少一致性保障。**  
   link 先创建链接，再保存 manifest；unlink/clean 先删除链接，再保存 manifest。保存失败会导致实际状态和记录不一致。manifest 又直接用 `fs::write` 覆盖，缺少原子替换和并发协调，存在文件截断、并发更新丢失的问题。见 [manifest.rs:216](F:/00_Workspace/AI_Agents/agent_linker/src/core/manifest.rs:216)、[project_links.rs:220](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:220)。

8. **【P1】分组链接缺少整体规划，dry-run 会漏报组内冲突。**  
   当前逐个调用单项链接逻辑；dry-run 不维护虚拟执行状态，所以两个成员计划写入同一路径时，都可能通过。**临时验证中 dry-run 成功，实际执行到第二项失败，第一项已经创建并记录。** 应先生成整个分组的计划，检查内部冲突，再执行并报告结果。见 [project_links.rs:324](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:324)。

9. **【P1】`--force` 替换链接失败时会丢失原链接。**  
   当前流程是先删除旧 symlink，再创建新 symlink。第二步遇到 Broker 或其他错误时，没有恢复旧链接的机制。mock 已验证“创建失败后原链接消失”。至少需要明确的部分失败结果及恢复策略。见 [symlink.rs:286](F:/00_Workspace/AI_Agents/agent_linker/src/core/symlink.rs:286)。

10. **【P2】`db check` 的健康判断过于表面，退出码也不可靠。**  
    它主要检查版本号、可写性和 Harness 行数，没有验证 Registry/Group 必需结构。**删除临时库的 `group_items` 表后，仍报告正常。** 此外，`CommandReport::is_success` 只对 doctor 判断失败，`db check` 即便显示 `needs attention` 也会退出 0，安装脚本无法据此可靠判断。见 [db.rs:223](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:223)、[commands/mod.rs:232](F:/00_Workspace/AI_Agents/agent_linker/src/commands/mod.rs:232)。

11. **【P2】数据库可写性、锁竞争和结构健康混为一谈。**  
    可写性探测会获取写锁并执行回滚事务，错误最终被压成一个 `writable=false`。用户无法区分权限不足、数据库正忙和其他异常。应分别报告连接权限、锁状态、schema 完整性；回滚式写入探测本身有用途，但结果表达需要重构。见 [db.rs:237](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:237)。

12. **【P2】数据库约束和应用层规则分布不合理。**  
    mapping 的 `(framework_id, link_path)` 唯一性只靠“先查后插”；有效链接名的冲突也靠遍历应用层对象检查，无法可靠处理并发。另外，Windows 文件路径大小写等价与 SQLite 文本比较规则没有统一，可能让同一个实际目标产生多份管理记录。需要先定义路径身份，再确定数据库约束。见 [db.rs:494](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:494)、[registry.rs:672](F:/00_Workspace/AI_Agents/agent_linker/src/core/registry.rs:672)。

13. **【P2，过度防御】Resource 的链接名唯一性限制范围过大。**  
    不同目标目录下的同名文件本可共存，目前却按整个 Resource 类型拒绝相同 basename。临时验证中，`tool-a/config.json` 和 `tool-b/config.json` 的注册被拦截。冲突应围绕实际目标路径判断；注册名称、alias、文件名也不应完全套用同一种验证规则。见 [registry.rs:672](F:/00_Workspace/AI_Agents/agent_linker/src/core/registry.rs:672)、[linkable.rs:214](F:/00_Workspace/AI_Agents/agent_linker/src/core/linkable.rs:214)。

14. **【P2，过度设计】部分数据库字段没有当前业务支撑。**  
    `source_type/source_ownership` 目前各只有一种值；`repo_url/repo_commit` 属于预留能力；旧的 `version/commit_hash` 又仍然保留，形成重复概念。这些字段已经带来迁移、解析和展示成本。建议明确哪些必须保留兼容，哪些应延后设计；不能直接删旧库字段。见 [db.rs:505](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:505)、[linkable.rs:36](F:/00_Workspace/AI_Agents/agent_linker/src/core/linkable.rs:36)。

15. **【P2】Harness 管理与实际链接行为脱节，默认映射存在两份来源。**  
    `harness enable/disable`、mapping 增删能保存，但 `init` 固定使用代码中的 Claude 映射，数据库里的设置没有被应用。同时 db seed 又维护了一份默认映射。当前文档已承认这一限制，但 CLI 容易让用户误以为配置已经生效。应确定唯一默认数据来源，并收拢未闭环的功能。见 [harness.rs:60](F:/00_Workspace/AI_Agents/agent_linker/src/core/harness.rs:60)、[db.rs:563](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:563)。

16. **【P2，过度设计】`config` 目前是缺少消费者的通用键值存储。**  
    可以写入任意配置，但运行逻辑没有据此解析有效设置，用户难以分辨哪些键会生效。应围绕实际支持的配置定义键、类型、默认值和读取流程，再决定是否需要通用配置接口。见 [db.rs:269](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:269)。

17. **【P2】手写 TOML 解析器增加维护成本，校验重点也偏离业务。**  
    manifest 使用自制 TOML 子集，正常 TOML 的部分写法无法读取；大量代码处理引号、转义和字段，却遗漏了路径安全约束。另一个过度限制是：仅因历史记录里的 `provider_backend` 值未知，就可能阻止清理，即便清理并不使用该后端。建议采用标准序列化，并区分执行必需字段与审计元数据。见 [manifest.rs:466](F:/00_Workspace/AI_Agents/agent_linker/src/core/manifest.rs:466)。

18. **【P2】dry-run 与真实操作维护了重复状态机。**  
    `plan_symlink` 和 `ensure_symlink` 分别实现冲突规则；删除也有 preflight、preview、执行多套判断。规则容易漂移。适合重构为共享操作计划，执行阶段保留必要的安全复查；真实文件保护和操作前复查仍应保留。见 [project_links.rs:987](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:987)。

19. **【P2】模块边界形式清晰，内部职责仍然过度集中。**  
    `commands` 基本是转发层，业务编排集中在 `project_links`；它同时承担链接、分组、清理、doctor 和 Windows 探测。`db` 同时处理安装 receipt、路径、配置、迁移和默认 Harness，而 `paths.rs` 为空。多层 `_with_provider_and_db_with_options` 包装也增加了入口数量。建议按职责收拢入口、拆分模块，同时保持底层 SQL、manifest 和平台操作封装在 core。见 [project_links.rs:146](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:146)、[paths.rs](F:/00_Workspace/AI_Agents/agent_linker/src/core/paths.rs)。

20. **【P2】错误模型丢失原因，verbose 对普通错误没有作用。**  
    多数数据库和 IO 错误被转换成字符串，难以区分不存在、约束冲突、锁竞争和权限不足。`print_error` 接收输出选项却完全忽略，普通模式和 verbose 都输出同样内容。适合保留结构化原因，由 CLI 决定展示深度。见 [error.rs:8](F:/00_Workspace/AI_Agents/agent_linker/src/core/error.rs:8)、[cli.rs:1036](F:/00_Workspace/AI_Agents/agent_linker/src/cli.rs:1036)。

21. **【设计待明确】`group unlink` 按当前成员操作，缺少链接来源语义。**  
    链接分组后移除某个成员，再执行 group unlink，该成员原有链接不会被选中；手动链接但属于当前分组的成员却会被选中。现有 manifest 没有记录分组来源。需要先明确它代表“取消当前成员的链接”还是“撤销此前这次分组链接”，再决定数据设计。见 [project_links.rs:463](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:463)。

22. **【P2】安装升级缺少失败恢复，数据位置选择也带来较高复杂度。**  
    Windows 和 Unix 都先替换可执行文件，再初始化或迁移数据库；数据库检查失败时，旧程序已被替换。Windows 把用户数据放在 Program Files 安装根下，还引入 ACL、提权 helper、receipt 识别及多用户归属问题。若该目录布局是硬性需求，这些机制有保留理由，但应明确数据所有者，并设计升级预检和恢复流程。见 [maintain-install.ps1:51](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/maintain-install.ps1:51)、[install.sh:55](F:/00_Workspace/AI_Agents/agent_linker/packaging/unix/install.sh:55)。

23. **【P2】测试偏重正常流程，缺少失败边界验证。**  
    当前测试覆盖了不少安全规则，但缺少事务中途失败、manifest 保存失败、并发更新、alias 交叉歧义、项目外路径和组内计划冲突。架构测试靠源码字符串黑名单，也容易随合法重构误报。下一阶段应优先补这些行为验证，而不是继续增加实现细节断言。见 [commands/mod.rs:492](F:/00_Workspace/AI_Agents/agent_linker/src/commands/mod.rs:492)。

建议处理顺序是：**先修路径安全和删错对象，再建立事务及文件状态一致性，随后分离数据库连接与生命周期，最后收缩未闭环功能、重复实现和模块职责。** SQLite 保存 Registry、Group 及成员关系的总体选择合理；目前最突出的问题在副作用边界、约束和操作一致性。真实 Broker、系统安装及跨平台行为，本次未作实机验证。
