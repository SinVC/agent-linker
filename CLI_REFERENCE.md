# Agent Linker CLI Reference

本文对应当前实现：全局 SQLite 保存注册与配置，项目 `.agents/links.toml` 保存受管理符号链接。

## 公共约定

- `<参数>` 必填，`[参数]` 可选，`...` 为一个或多个，`|` 为二选一（非 shell 管道）；空格参数加引号。各级命令支持 `--help`。
- **项目根就是工作目录，不向上查找。** `init/link/unlink/status/clean` 操作该目录；Registry、Group 定义、Config、Harness 操作全局数据库；`doctor` 检查两者。
- 源相对工作目录解析，注册为规范化绝对路径。目标目录、Harness 路径相对项目根，禁止绝对路径和 `..`。名称、别名、`--as` 须为非空单段，禁止 `/\<>:"|?*` 及单独的 `.`、`..`。
- 缺清单按空清单处理，损坏时报错；不强制先 `init`。`link` 创建缺少的真实父目录并保存清单，不补齐初始化结构；父路径含符号链接时报错。
- 不复制、移动或删除源。正确链接可重复链接并纳管；`--force` 只替换指向错误的符号链接，不能替换真实文件／目录或抢占其他条目的清单路径。
- Windows 创建链接依赖可用的 `WinSymlinksBroker`，不可用时报错，不自动回退；用 `aglink doctor --verbose` 诊断。
- `--dry-run` 仅用于列明的命令：检查预览，不修改项目目录、链接或清单；数据库仍可能创建／迁移。修改不保证事务性：遇错停止，不回滚。
- 全局 `-q/--quiet` 隐藏普通结果，`--verbose` 增加报告细节；可置于子命令前后，同时使用 quiet 优先。`status --json` 不受两者影响；错误写 stderr。
- 完成／帮助返回 `0`，参数／执行错误及 `doctor` 检查失败返回 `1`。`status` 异常、`db check` 的 `needs attention` 不导致非零退出。

## 名称与路径

`<item>` 接受注册名、别名或 Registry ID。Skill/Resource 维护限对应类型；`link`、`group add/remove` 跨类型查询，多项匹配时报错，用 ID 消歧。同类型注册名、默认链接名分别唯一；若注册名与其他别名交叉重合，类型内维护按注册名排序取首项，应使用 ID 避免误选。`<group>` 接受分组名或 ID。

| 规则 | Skill | Resource |
| --- | --- | --- |
| 源要求 | 目录，含可读、非空白 UTF-8 `SKILL.md` | 文件或目录 |
| 省略 `--name` | 输入路径末段去掉最后一个扩展名 | 同左，如 `guide.md` → `guide` |
| 默认链接名 | `--alias`，否则注册名 | `--alias`，否则规范化源路径完整末段，如 `guide.md` |
| 项目相对链接路径 | `.agents/skills/<链接名>` | `<目标目录>/<链接名>` |

`link --as` 覆盖默认链接名，不补扩展名；Resource 的 `link --target-dir` 覆盖注册目录。均不改 Registry；同一条目可链接到多个路径。

## 初始化、状态与清理

- `aglink init`：创建缺失的真实 `AGENTS.md`（空）、`.agents/`、`.agents/skills/`；创建 `CLAUDE.md` → `AGENTS.md`、`.claude/skills/` → `.agents/skills/` 并合并入清单。保留源内容，正确链接跳过；源位置类型不符、错误链接或真实路径冲突时报错。`.gitignore` 的 aglink managed block 包含 `.claude/`、`.agents/skills/`、`.agents/links.toml`，保留块外内容。可重复执行；无 `--dry-run/--force`。
- `aglink status [--json]`：只检查清单记录，不扫描未管理链接，不修复。JSON 顶层为 `project_root`、`manifest_path`、`links`；每项含 `id`（清单 ID）、`item_id`、`item_name`、`link_path`、`source_path`、`link_kind`（`file/directory`）、`provider_backend`、`status`。

| `links[].status` | 含义 |
| --- | --- |
| `missing` | 链接路径不存在 |
| `correct_symlink` | 指向记录源且源类型正确 |
| `wrong_symlink_target` | 指向其他目标，即使该目标不存在 |
| `broken_symlink` | 指向记录源，但该源不存在 |
| `existing_real_file` / `existing_real_directory` | 链接位置被真实文件／目录占用 |
| `unsupported_file_type` | 链接位置类型不支持，或源类型与记录不符 |

- `aglink clean [--broken|--missing-source] [--dry-run]`：筛选互斥，仅处理清单。默认选 `missing/broken_symlink`；`--broken` 只选 `broken_symlink`；`--missing-source` 按记录源不存在筛选，即使链接指向别处。所选路径不存在时只删记录，否则删符号链接及记录；选中真实文件／目录或不支持类型时报错。
- `aglink doctor`：检查数据库、schema、Harness 记录、清单及后端；Windows 临时创建／删除链接探测 Broker。缺清单按空清单通过；不修复项目链接，Harness 查询可能初始化／迁移数据库。

## 链接与解除链接

```text
aglink link <item> [--as <链接名>] [--target-dir <目录>] [--force] [--dry-run]
aglink link --group <group> [--force] [--dry-run]
aglink unlink <selector> [--dry-run]
aglink unlink --group <group> [--dry-run]
aglink unlink --all [--dry-run]
```

- `link` 校验源存在、Skill 内容及源类型；`--target-dir` 仅 Resource 可用。分组用成员默认路径，不接受 `--as/--target-dir`；逐项执行，失败前成功的链接保留。
- `unlink <selector>` **查询清单，不查询 Registry**：接受清单 ID、条目 ID、清单条目名、项目相对链接路径或末段名。须恰好匹配一条；多条时用清单 ID 或完整相对路径消歧。Registry 改名／注销后仍可操作。
- selector、`--group`、`--all` 必选且互斥。`--group` 按**当前成员 ID**选全部对应链接；不记录链接时所属组，移出组者不选。`--all` 包括 `init` 的 Harness 链接。
- 删除所选正确／错误／损坏链接及记录，路径缺失时只删记录；先检查全部所选项，真实文件／目录或不支持类型会阻止操作。保留源及父目录；单项和 `--all` 不依赖数据库。

## Skill 与 Resource 注册

```text
aglink skill add <path> [--name <名称>] [--alias <链接名>]
aglink resource add <path> --target-dir <目录> [--name <名称>] [--alias <链接名>]
aglink <type> list
aglink <type> show <item>
aglink <type> rename <item> <新注册名>
aglink <type> remove <item>
aglink <type> refresh <item>
```

`<type>` 为 `skill` 或 `resource`。`add` 只注册本地源，不创建链接；同类型同注册名或同默认链接名报错，重复注册不更新。

`list/show` 查看注册；`rename` 保留 ID、别名及分组引用，不改项目链接／清单；无别名 Skill 改名影响以后默认链接名。`remove` 注销条目及分组成员关系，保留源和链接。`refresh` 校验原路径并更新规范化路径／时间，源丢失或类型变化时报错，不寻找移动后的源或修复链接。修改源路径、别名、Resource 默认目录需注销后重注册（ID 改变）；已有链接应先 `unlink`。

## Group 分组

```text
aglink group create <名称>
aglink group list
aglink group show <group>
aglink group rename <group> <新名称>
aglink group delete <group>
aglink group add <group> <item>...
aglink group remove <group> <item>...
aglink group link <group> [--force] [--dry-run]
aglink group unlink <group> [--dry-run]
```

`create` 建空组，重名报错；`list/show` 查看组及成员。`add/remove` 至少一个成员；重复输入、添加已有成员、移除非成员报错。`rename/delete` 和成员增删不改项目链接；删组保留条目。`group link/unlink` 分别等价于 `link/unlink --group`。

## 配置与数据库

```text
aglink config path
aglink config list
aglink config get <key>
aglink config set <key> <value>
aglink config unset <key>
aglink db path
aglink db migrate
aglink db backup [文件路径]
aglink db check
```

- `config path` 与 `db path` 等价：显示路径及原因，不打开数据库。其余 Config、Registry、Group 定义、Harness 查询／修改自动创建／迁移数据库。
- Config 存取任意字符串键值，**尚无接入 CLI 行为的键或默认值**。`set` 新增／覆盖，key 不得全空白；`get` 不存在输出 `unset`；`unset` 删除值，不存在也成功；`list` 列已保存值。
- `db migrate` 创建／升级数据库，补齐内置 Harness 数据。`db backup` 先迁移再备份；省略路径时在数据库同目录生成 `agent-linker-backup-<纳秒时间戳>.db`；显式为相对工作目录或绝对**文件路径**，自动建父目录，目标存在时报错。
- `db check` 检查存在性、可写性、schema、Harness／映射记录，不做 SQLite 完整性扫描或迁移。需读取输出状态；需失败退出码用 `doctor`。

数据库按优先级选择，文件名固定为 `agent-linker.db`：

1. 非空环境变量 `AGLINK_DATA_DIR` 指定目录（相对值相对于工作目录）。
2. 可执行文件同目录**已存在**的 `agent-linker.db`（portable 模式）。
3. 平台默认目录：Windows `%LOCALAPPDATA%/agent-linker/`；macOS `~/Library/Application Support/agent-linker/`；Linux `$XDG_DATA_HOME/agent-linker/`，未设置时 `~/.local/share/agent-linker/`。

## Harness 兼容适配

Harness 指运行 Agent 的软件系统，如 Claude Code。本组命令管理兼容路径与适配设置，不启动／停止该工具；旧 `framework` 命令已移除。

```text
aglink harness list
aglink harness show <Harness>
aglink harness enable <Harness>
aglink harness disable <Harness>
aglink harness mapping list [Harness]
aglink harness mapping add <Harness> <source> <link> --kind <file|dir>
aglink harness mapping remove <Harness> <link>
```

`<Harness>` 为已有 Harness 名或 ID；内置 `claude` 指 Claude Code，无新增 Harness 命令。`list/show` 查看定义，`enable/disable` 修改**全局**启用标记。mapping 的 source/link 为项目相对路径，添加时不校验源存在；kind 为文件／目录（`dir` 也接受 `directory`）。`mapping list` 省略 Harness 列全部；同 Harness 重复 link 报错，required 内置映射不可删。

**当前仅维护数据库，不修改项目链接；`init` 固定使用内置 Claude Code 两条映射，不读取数据库启用标记／自定义 mapping。** 禁用适配不阻止 `init` 创建兼容链接，新增 mapping 不生效。

## 示例：名称覆盖与最终路径

在项目根执行；假设 `../skills/writer/SKILL.md` 有非空内容，`../guide.md` 存在，名称未注册且目标无冲突：

```text
aglink init
aglink skill add ../skills/writer --name writer --alias writing
aglink link writer --as draft
aglink unlink .agents/skills/draft
aglink resource add ../guide.md --target-dir docs
aglink link guide --target-dir references --as handbook.md --dry-run
aglink link guide --target-dir references --as handbook.md
aglink unlink references/handbook.md
```

Skill 创建 `.agents/skills/draft` → writer 源目录，注册默认仍为 `writing`。Resource 注册名为 `guide`，默认路径 `docs/guide.md`，本次创建 `references/handbook.md` → guide.md 源文件。`unlink` 删除链接和记录，保留源及注册信息。
