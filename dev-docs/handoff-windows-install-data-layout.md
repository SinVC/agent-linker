# Handoff：Windows 单用户安装与数据库目录

状态：已实施（2026-10-28）。以下方案已落地；原始排障证据与历史限制保留供复核。

## 本轮实施与验证

- Windows 普通安装使用有效 receipt 识别的 `bin/`、`data/`、`backups/` 布局；磁盘辅助脚本处理保护目录与 ACL，普通用户外层处理 PATH、旧库快照和初始化。升级复用既有 Broker；普通卸载保留数据、备份、Broker 文件及维护脚本，purge 仅删除已知数据与默认备份。
- 查询及项目链接流程对全局库只读；备份改为只读源库的 SQLite 一致快照，不迁移源库。Portable 打包与 Unix 安装补齐初始化，使用说明已同步。
- Windows `cargo test --locked`：67 项通过；WSL Ubuntu：68 项通过。包含 receipt/平台路径、缺库与旧/新 schema、写锁期间查询和分组链接/解绑（含 dry-run）、WAL 快照及目标防覆盖。
- `cargo fmt --check`、`cargo check --locked`、CLI help 和 Windows 脚本语法检查通过。
- [Windows 离线夹具](../packaging/windows/test-install.ps1)已在可写自定义根和独立 Program Files 根执行，验证普通用户可写数据/备份而不能写 `bin/`、`-NoBroker`、旧库导入保留源、覆盖环境恢复、portable 首次查询、升级/卸载保留、purge 保留未知文件、损坏目标安装失败。Broker 复用不覆盖及必需安装失败通过独立子进程 mock 验证；没有替换或移除本机真实服务。
- Program Files 夹具在不设置 `AGLINK_DATA_DIR` 时完成普通 PowerShell 与 Agent 沙箱外命令的随机分组双向创建、回读、删除；`db path` 一致。Agent 沙箱内查询与 `link --dry-run` 成功，未要求数据库写权限。测试夹具已清理，未迁移本机正式数据库。
- [Unix 离线夹具](../packaging/unix/test-install.sh)已使用真实 Linux CLI 验证安装后首次查询、源码/手工安装显式初始化及初始化失败退出码。
- MSIX 身份重定向、macOS 实机和真实 Broker 首次安装/卸载仍不在本轮实机验证覆盖范围内。

## 要达到的结果

Windows 普通安装采用以下布局；`-InstallDir` 继续支持自定义安装根目录。

```text
C:\Program Files\aglink\
├── bin\          程序，升级时替换
├── data\         agent-linker.db 及 SQLite 日志；升级和普通卸载保留
└── backups\      用户显式生成的备份；升级和普通卸载保留
```

这是主要目录的示意，现有 `broker/`、`scripts/`、`install-receipt.json` 仍按用途保留，无需为凑成三个目录而重排。

核心要求：同一用户、同一安装和数据配置下，用户终端与 Agent 必须使用同一个物理数据库。权限不足应返回错误，不得换目录或创建替代库。显式设置不同的 `AGLINK_DATA_DIR` 属于主动切换数据配置。

本次按 Windows 单用户场景实现。Linux/macOS 保留现有路径；但只读连接是共享代码，必须补齐各安装形态的首次初始化，不能把“不改路径”理解为“不检查首跑流程”。

## 调整前的证据和验证缺口（历史）

- Program Files 普通权限下，数据库可读但不可写；只给 `.db` 文件写权限仍不足。给独立 `data/` 目录及子文件授予指定用户 Modify 后，SQLite rollback/WAL 写入及实际 aglink 命令均成功。
- `.aglink/data` 独立测试库通过普通 PowerShell、Codex 内置终端、Agent 沙箱外命令的双向读写；Agent 沙箱只读也访问了同一文件。核对了文件句柄最终路径、卷号、文件 ID 和库内测试标记，未只比较路径字符串。
- 沙箱中的 `LOCALAPPDATA` 指向另一目录；当前默认规则仍可能选错库。沙箱读取测试库成功，现有 `group list` 却因初始化写入而失败。
- 上述是两组独立实验，**尚未验证最终 Program Files 安装布局的完整 Agent 流程和升级/卸载流程**。本轮跨入口测试均未带 MSIX 包身份，不能声称已覆盖此前的 MSIX 重定向场景。

以上为早期排障记录；其本机原始报告、测试脚本和恢复快照已于 2026-10-28 按用户要求清理，不再作为文件依赖。新布局的实施验证见上方记录。

本机当前正式库为 `C:\Program Files\aglink\data\agent-linker.db`，含 11 个技能、2 个分组；CLI 位于 `C:\Program Files\aglink\bin\aglink.exe`。本次清理删除了旧 Roaming 库、历史备份、恢复与测试目录，正式库哈希未变且完整性检查通过。Codex/沙箱目录中未发现旧数据库残留；Broker 原有安装和运行状态保留。

## 最小实施范围和代码落点

### 1. 安装器保留普通用户外层，只提权写保护目录

- x64 包的默认根目录取 `ProgramW6432`（缺失时取 `ProgramFiles`）下 `aglink`，沿用 `-InstallDir`。自定义目录可写且不需要装 Broker 时，不强制 UAC。
- 普通用户外层先记录 SID、旧数据位置，下载并校验发布包；通过磁盘上的辅助脚本提权。官方入口是 `irm ... | iex`，不能假设 `$PSCommandPath` 指向可重新运行的脚本。
- 提权部分只负责受保护目录中的安装/更新、receipt、ACL 及必要的 Broker 安装。给发起安装的用户 SID 授予 `data/`、`backups/` 及子文件的 Modify，保留根目录、`bin/`、Broker 与安装脚本的保护权限。
- 回到普通用户外层更新用户 PATH、迁入旧库、执行 `db migrate` 和 `db check`，检查退出码。安装器自己的 CLI 调用显式指定本次目标数据目录，并在 `finally` 恢复原有 `AGLINK_DATA_DIR`；不修改用户的全局覆盖配置。外层本来已提权时，其运行成功不能代替普通用户 ACL 验收。
- 更新 receipt 的 `install_dir`、`data_dir`。安装辅助步骤及所要求的 Broker 安装失败必须报错；`doctor` 同时检查当前项目和 Broker，只作诊断输出，不把它的总退出码当作安装成功条件。`-NoBroker` 允许缺少 Broker，不能因此把数据库安装判为失败。

位置：[install.ps1:29](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/install.ps1:29) 修改用户 PATH；[install.ps1:50](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/install.ps1:50) 目前仅对 Broker 提权；[install.ps1:72](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/install.ps1:72) 是默认目录；[install.ps1:112](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/install.ps1:112) 写 receipt；[install.ps1:132](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/install.ps1:132) 初始化/诊断。`doctor` 的项目检查和 Broker 检查分别在 [project_links.rs:649](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:649)、[project_links.rs:711](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:711)。

### 2. 明确正常安装识别及其他形态的回退

路径优先级固定如下，不按数据库是否存在临时切换普通安装目标：

| 条件，按顺序匹配 | 使用位置 |
| --- | --- |
| 显式 `AGLINK_DATA_DIR` | 保留现有覆盖行为 |
| Windows 的 `bin/aglink.exe`，上一级有本产品正常安装的有效 receipt | 从 exe 布局推导 `<installRoot>/data/agent-linker.db` |
| 其他形态，exe 旁有 portable 数据库 | 保留既有 portable 行为 |
| 未识别为上述安装形态 | 保留现有平台默认路径，包括 Windows 的 LocalAppData 回退 |

不能只看父目录叫 `bin`，否则 `cargo install` 的 `.cargo/bin` 会被误判；也不要仅检查任意同名 receipt 文件，至少核对已有产品/安装形态字段。receipt 读取失败或已安装记录损坏时明确报错，不能当成未安装再选另一个库。正常安装不依赖 `LOCALAPPDATA`；未安装形态的兼容回退不属于这项保证，Agent 跨入口使用时应显式统一数据目录。无需新增定位文件或安装注册中心。

在 core 中处理路径及连接错误并附上目标路径。保留现有 `aglink db path`/`doctor --verbose` 输出，不为此强制 canonicalize 尚不存在的数据库。

位置：[db.rs:134](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:134) 是当前环境变量、portable、平台默认解析顺序；[db.rs:571](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:571) 是平台默认目录。路径输出在 [cli.rs:596](F:/00_Workspace/AI_Agents/agent_linker/src/cli.rs:596)，现有诊断在 [project_links.rs:613](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:613)。

### 3. 查询不负责创建和迁移数据库

为查询增加最小的只读连接入口，复用现有 SQL：

- 技能/资源、分组、配置、Harness 的查询和 mapping 查询只读；链接/分组链接/分组解绑及其 dry-run 对全局库也只读，项目改动仍走原 manifest/symlink 流程。
- 不创建数据库、不执行 migration 或 seed。缺库/旧 schema 提示初始化或 `db migrate`；库比程序新时提示升级程序。用 metadata 区分不存在和权限/IO 错误，不用 `exists() == false` 一概当成缺库。
- `db check` 的可写性探测使用不带 CREATE 的读写打开方式，不借检查建库。保留当前默认 rollback 日志模式，本轮不引入 WAL 优化，也不在查询时强制切换旧库的日志模式。WAL 源库的快照仍必须正确；只读 WAL 的附加文件权限限制见 [SQLite 官方说明](https://www.sqlite.org/wal.html#read_only_databases)。

位置：[db.rs:340](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:340) 每次打开都会迁移并 seed；[db.rs:352](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:352) 会创建目录和数据库。Registry 的 [registry.rs:47](F:/00_Workspace/AI_Agents/agent_linker/src/core/registry.rs:47)、[registry.rs:102](F:/00_Workspace/AI_Agents/agent_linker/src/core/registry.rs:102)、[registry.rs:197](F:/00_Workspace/AI_Agents/agent_linker/src/core/registry.rs:197)，配置的 [db.rs:224](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:224)，Harness 的 [harness.rs:122](F:/00_Workspace/AI_Agents/agent_linker/src/core/harness.rs:122) 都存在这条写连接调用链。继续遵守 `cli -> commands -> core`，不要把 SQL 移到 commands。

其他实际调用点：[project_links.rs:197](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:197)、[project_links.rs:332](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:332)、[project_links.rs:471](F:/00_Workspace/AI_Agents/agent_linker/src/core/project_links.rs:471)，[harness.rs:195](F:/00_Workspace/AI_Agents/agent_linker/src/core/harness.rs:195) 的 mapping 查询，以及 [db.rs:185](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:185) 的检查入口。

### 4. 升级、卸载和旧数据处理

- 升级保留 `data/`、`backups/`。默认复用现有可用 Broker，不因 CLI 换目录就强制迁移共享服务；保留其服务路径及所需文件。若实际要替换服务正在使用的 exe，先停服务再替换并恢复启动，不能无条件覆盖。
- **普通卸载禁止递归删除整个安装根。** 保护目录内的删除需要提权，用户 PATH 仍由普通用户外层处理。保留数据、备份，以及仍被保留的 Broker 程序和 `scripts/uninstall-broker.ps1`。
- `-Purge` 改为清理本安装的 `data/`、`backups/`，不要仍只清理旧 AppData。自定义 `AGLINK_DATA_DIR` 继续沿用现有保护，不假设其中有安装器从未创建的 marker，也不递归删除未知内容。已知数据文件包括 `.db`、`-journal`、`-wal`、`-shm`；Broker 仍由 `-RemoveBroker` 显式控制。
- 一次性旧库导入由安装器普通用户外层编排，在目录赋权后、目标初始化前执行。只检查外层用户的既有默认旧库；目标不存在且旧库存在就快照到新库，保留旧库；目标已有库就校验后使用，校验失败明确报错，不覆盖。旧库不存在才走新装，不扫描 Packages，也不做多源合并。
- 快照实现在 core。现有 `backup_database` **先迁移/可能创建源库，再 `fs::copy`**，两处都必须改：只读打开已存在源库，使用 SQLite 一致快照，之后只迁移目标。可以使用 [SQLite 的 VACUUM INTO](https://www.sqlite.org/lang_vacuum.html#vacuum_with_an_into_clause) 或 backup API，不增加独立迁移引擎。安装器调用时显式选择旧源目录和新目标，不用同盘 Move 替代迁入新目录后的 ACL 继承。
- 未指定备份目标时，**仅 Windows 正常安装**写入 `<installRoot>/backups/`；覆盖目录、portable、Unix 保留原默认位置。依据解析原因区分，不对任意数据路径直接取父目录的 `backups/`。显式目标仍有效。

位置：[install.ps1:105](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/install.ps1:105) 无条件复制 Broker；[install-broker.ps1:30](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/install-broker.ps1:30) 复用已安装服务。**[uninstall.ps1:109](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/uninstall.ps1:109) 当前删除整个安装根**；[uninstall.ps1:48](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/uninstall.ps1:48) 是删除保护，[uninstall.ps1:114](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/uninstall.ps1:114) 是旧 purge 位置。备份见 [db.rs:303](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:303)，默认备份位置见 [db.rs:619](F:/00_Workspace/AI_Agents/agent_linker/src/core/db.rs:619)。

### 5. 补首跑初始化及同步使用说明

- Portable 包不能再只放 0 字节数据库：打包阶段在明确的 staging 数据目录执行 `db migrate`，产出已初始化空库，不能误初始化构建机的默认库。
- Unix 安装脚本在安装后执行初始化并检查失败；源码安装/手工解压的快速开始明确先运行 `aglink db migrate`。这只补首跑流程，不修改 Unix 路径。
- 同步 receipt 数据位置、README、CLI_REFERENCE 的使用说明，以及 release-distribution、04-global-store-registry、RELEASE_CHECKLIST 和 architecture 的待实施状态。保留现有用户文档改动；不把本仓库开发用 AGENTS.md 当成外部使用手册。
- 用户/Agent 说明只保留可执行规则：持久化写操作默认经宿主授权在沙箱外执行，只读查询可在沙箱内执行；日常使用普通用户身份。CLI 权限不足明确失败，不自行解除沙箱。

位置：[package-windows.ps1:73](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/package-windows.ps1:73) 目前创建空文件，[package-windows.ps1:88](F:/00_Workspace/AI_Agents/agent_linker/packaging/windows/package-windows.ps1:88) 声明数据位置；[install.sh:55](F:/00_Workspace/AI_Agents/agent_linker/packaging/unix/install.sh:55) 仅安装程序；[README.md:79](F:/00_Workspace/AI_Agents/agent_linker/README.md:79) 的首跑示例缺少数据库初始化。

## 验收要求

1. 路径测试覆盖正常/自定义安装、覆盖目录、portable、cargo 的 `bin`、无 exe 上下文及原有平台回退；正常安装不受 `LOCALAPPDATA` 改变影响。
2. 在独立的 Program Files 安装夹具验证普通用户可读写 `data/`、生成备份但不能改受保护程序。另验证 `-NoBroker`、用户可写自定义根及初始化失败退出码；无需重复构造已经验证过的坏 ACL 组合。
3. **不设置 `AGLINK_DATA_DIR`**，普通终端与 Agent 沙箱外命令分别创建随机唯一分组，由另一方立即读到并删除；结合 `db path` 核对默认目标即可。File ID 仅在结果有疑点时排障，不是固定验收脚本。
4. 已初始化库的沙箱查询及链接 dry-run 不应要求数据库写权限；真实权限拒绝明确失败。缺库查询不建库，旧/新 schema 提示正确，失败后不生成替代库。
5. 升级和普通卸载保留数据库记录及备份，保留 Broker 时其文件/后续卸载脚本仍在；purge 清理新布局。迁移/备份保留源库逻辑内容及 schema，包含已提交 WAL 数据；缺源不建库、已存在目标不覆盖。
6. Portable、Unix 安装、源码/手工解压按文档首次运行能正常查询。无需把 MSIX 身份探测加入正式验收门槛；原始排障记录保留覆盖限制。

实际 Rust 工程为单 crate，已有单元测试在各模块 `#[cfg(test)]` 中，没有独立 `tests/` 目录。修改代码后运行 `cargo test --locked`；Windows 安装/升级/卸载还需实际脚本测试。按本项目 AGENTS.md，使用默认 Cargo 缓存的命令须直接申请沙箱外执行。本文仅为文档交接，不需要为写文档运行 Cargo。

## 复核后的收敛与反思

- 接受 Gemini 对 cargo 误判、两阶段安装、首跑初始化、遗漏只读调用链、源库先迁移及卸载/purge 的修正。这些均能定位到现有代码，不是扩展产品功能。此前将 receipt 识别视作可有可无，收缩过头；复用已有标识是必要的简单判别。
- 不采纳“Broker 必然占用新安装目录文件”或“CLI 换目录就必须迁移服务”的绝对结论。是否冲突取决于服务实际路径；默认复用可用服务，仅对实际覆盖处理停启。
- 不把 `doctor` 简化为“非 NoBroker 时必须成功”：它还检查当前项目。安装以明确的数据库检查结果判定，保留 doctor 供诊断。
- 删去没有具体触发条件的“来源不明确”迁移分支、普通使用说明中的 MSIX 机制解释，以及固定的包身份/File ID 检查和重复坏 ACL 实验；历史证据仍可用于排障。
- 不增加多用户管理、通用沙箱检测、新常驻服务、数据库 UUID schema、安装注册中心或定时备份系统；不改技能链接方式，也不把目录调整说成已解决侧边栏枚举问题。
