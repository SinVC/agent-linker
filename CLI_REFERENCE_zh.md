# Agent Linker CLI 参考手册

`aglink` 通过全局 SQLite 数据库管理注册表（Registry）、分组（Group）与宿主适配（Harness），通过项目内的 `.agents/links.toml`（项目清单）管理符号链接状态。

## 公共约定

- **作用域与解耦**：
  - **`init` 严格使用当前目录；项目命令向上查找最近的 `.agents/links.toml`**，遇文件系统根或符号链接目录停止。`init`、`link`、`unlink`、`status`、`clean` 操作当前项目；`skill`、`resource`、`group`、`db`、`harness` 操作全局数据库；`doctor` 同时检查两者。
  - **全局与项目状态解耦**：修改全局数据库（改名、注销、增删分组或修改 Harness）绝不自动改动项目中已有的符号链接或清单记录。
- **路径与命名约束**：
  - 注册源路径相对于当前工作目录解析，保存为规范化绝对路径。
  - 项目目标目录与 Harness 映射路径均相对于项目根目录，禁止绝对路径或 `..`。
  - 注册名、别名（`--alias`）、分组名及 `--as` 必须为不含路径分隔符的非空单段名称，禁止包含 `/\<>:"|?*` 或单独的 `.`、`..`。
- **清单与非事务性**：项目命令要求已有 `.agents/links.toml`；未找到时提示先运行 `aglink init`，清单损坏则报错。`link` 会自动创建缺失的真实父目录并更新清单（父路径含符号链接则报错），但不补齐 `init` 的其余结构。`--dry-run` 仅做预览检查，不修改项目文件或清单。所有文件操作非事务性，遇错即停，已执行的更改不回滚。
- **文件安全边界**：
  - 绝不复制、移动或删除源文件；绝不覆盖或删除真实文件与目录。
  - 目标位置已是正确符号链接时，重复 `link` 直接将其纳入清单；`--force` **仅允许替换指向错误目标的符号链接**，不可覆盖真实文件/目录或抢占清单中其他条目的路径。
  - `unlink` 与 `clean` 仅移除符号链接及清单记录（链接已缺失时仅删记录）；若选中项包含真实文件/目录或不受支持的文件类型，操作报错中止。
- **Windows 后端**：Windows 下创建链接依赖 `WinSymlinksBroker` 服务，不可用时直接报错，不自动回退（可用 `aglink doctor --verbose` 诊断）。
- **输出与退出码**：`-q`/`--quiet` 隐藏常规输出，`--verbose` 增加诊断细节（可置于子命令前后，同时指定时 `--quiet` 优先；不影响 `status --json`）。错误输出至 `stderr`。成功或帮助返回 `0`；参数错误、执行失败、`doctor` 或 `db check` 未通过返回非零（`status` 链接异常仍返回 `0`）。

## 名称解析与默认路径

`<item>` 接受注册名或 Registry ID；`<group>` 接受分组名或 ID。

- **唯一性**：注册名跨 Skill/Resource 全局大小写无关唯一。Skill 默认链接名大小写无关唯一；Resource 按归一目标目录及链接名联合唯一，允许不同目录中的同名文件。alias 只决定默认链接名，不参与 ID/name 查询。

| 规则 | Skill | Resource |
| --- | --- | --- |
| **源要求** | 目录，含可读、非空白 UTF-8 `SKILL.md` | 任意文件或目录 |
| **默认注册名**（省略 `--name`） | 源路径末段去除最后一个扩展名 | 同左（如 `guide.md` → `guide`） |
| **默认链接名** | `--alias`，未设置则取注册名 | `--alias`，未设置则取源路径完整末段（如 `guide.md`） |
| **默认项目链接路径** | `.agents/skills/<链接名>` | `<目标目录>/<链接名>` |

`link --as <链接名>` 临时覆盖默认链接名（不自动补扩展名），Resource 的 `link --target-dir <目录>` 临时覆盖目标目录。同一条目可链接到项目内的多个不同路径。

## 初始化、状态与清理

```text
aglink init
aglink status [--json]
aglink clean [--broken | --missing-source] [--dry-run]
aglink doctor
```

- **`init`**：幂等初始化项目（不支持 `--dry-run`/`--force`）。创建缺失的空文件 `AGENTS.md` 及 `.agents/`、`.agents/skills/`（保留已有内容）；只读加载全局库中已启用 Harness 的映射并写入清单；缺库时使用同一内置 Claude 默认映射，不创建全局库（已是正确链接则跳过，源类型不符或目标冲突则报错）；在 `.gitignore` 中维护受管区块（写入 `.claude/`、`.agents/`）。
- **`status [--json]`**：仅检查清单中的记录，不扫描未纳管链接，也不执行修复。`--json` 顶层含 `project_root`、`manifest_path`、`links`（每项含清单 `id`、`item_id`、`item_name`、`link_path`、`source_path`、`link_kind`［`file`/`directory`］、`provider_backend`、`status`）。

| `links[].status` | 含义 |
| --- | --- |
| `missing` | 链接路径不存在 |
| `correct_symlink` | 指向记录的源路径，且源存在、类型匹配 |
| `wrong_symlink_target` | 指向其他目标（无论该目标是否存在） |
| `broken_symlink` | 指向记录的源路径，但源已不存在 |
| `existing_real_file` / `existing_real_directory` | 链接位置被真实文件或目录占用 |
| `unsupported_file_type` | 链接位置类型不受支持，或源当前类型与记录不符 |

- **`clean`**：按互斥条件清理清单失效项。默认清理 `missing` 和 `broken_symlink`；`--broken` 仅清理 `broken_symlink`；`--missing-source` 清理记录源已不存在的所有项（即使当前链接指向别处）。
- **`doctor`**：检查全局数据库、表结构、内置 Harness 记录、项目清单及链接后端（Windows 下会临时创建并删除链接以探测 Broker）。不修复项目链接。

## 链接与解除链接

```text
aglink link <item> [--as <链接名>] [--target-dir <目录>] [--force] [--dry-run]
aglink link --group <group> [--force] [--dry-run]
aglink unlink <selector> [--dry-run]
aglink unlink --group <group> [--dry-run]
aglink unlink --all [--dry-run]
```

- **`link`**：校验源存在性、类型及 `SKILL.md` 有效性。`--target-dir` 仅适用于 Resource。`--group` 使用成员各自的默认路径（不支持 `--as`/`--target-dir`），按成员逐项执行。
- **`unlink`**：`<selector>`、`--group`、`--all` 三选一。执行前先校验全部选中项，遇真实文件/目录则整体中止，不做部分删除。
  - `<selector>` **仅查项目清单（不访问数据库）**：接受清单 ID、`item_id`、清单记录名、项目相对链接路径或链接末段名。须唯一匹配一条记录，多条匹配时用清单 ID 或完整相对路径消歧。
  - `--group <group>`：按分组**当前成员 ID** 匹配 item 来源链接（已移出组的成员不选），保留 Harness 链接。
  - `--all`：解除所有 item 来源链接，保留 Harness 链接（不访问数据库）。

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

`<type>` 为 `skill` 或 `resource`：

- **`add`**：注册本地源到全局数据库（不创建项目链接）。注册名全局冲突或默认目的地违反上方唯一性规则时报错，不覆盖更新。
- **`list` / `show`**：列出该类型所有条目或查看单项详情。
- **`rename`**：修改注册名，保留 ID、别名与分组关系（未设别名的 Skill 改名后会改变其后续默认链接名）。
- **`remove`**：注销条目并移出所有分组。
- **`refresh`**：重新校验原路径并更新规范化路径与时间戳；源丢失或类型改变时报错（不自动追踪移动后的路径）。
- 修改源路径、别名或默认目标目录需先 `remove` 再 `add`（ID 会改变；建议先 `unlink` 已有项目链接）。

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

- **`create`**：创建空组（重名报错）。**`list`**：聚合名称与成员数量；**`show`** 加载完整成员。**`rename` / `delete`**：改名或删除分组（删组不注销成员条目）。
- **`add` / `remove`**：整次事务增删成员；输入按 item_id 去重，已有成员在 add 时跳过并提示。未知条目或移除非成员时报错，整次回滚。
- **`group link` / `group unlink`**：分别等价于 `link --group` 与 `unlink --group`。

## 数据库

```text
aglink db path
aglink db migrate
aglink db backup [文件路径]
aglink db check
```

- **`db path`**：仅输出解析后的数据库路径及依据，不打开数据库。查询使用只读连接，不创建、迁移或补齐内置数据；缺库或旧 schema 提示运行 `aglink db migrate`，新 schema 提示升级程序。
- **`db migrate`**：显式创建或升级到 Schema v3，迁移链和 Seed 整体事务提交，保留存量 ID 与引用。内置数据 INSERT OR IGNORE，不覆盖用户状态或时间。普通读写不负责初始化或迁移。
- **`db backup [文件路径]`**：只读打开已存在的源库，生成含已提交 WAL 数据的一致快照，保留源 schema。省略路径时仅 Windows 正常安装写入 `<installRoot>/backups/`，其他形态在数据库同目录生成 `agent-linker-backup-<纳秒时间戳>.db`；指定文件路径时自动创建父目录，目标已存在则报错。
- **`db check`**：检查存在性、版本、五张核心表及 SQLite quick_check，再以不带 CREATE 的连接探测可写性；Harness 表为空也可健康。missing/schema_mismatch/read_only/locked_busy/invalid 返回非零，healthy_writable 返回 0；不建库、不迁移、不 Seed。

**数据库路径优先级**（文件名固定为 `agent-linker.db`）：
1. 非空环境变量 `AGLINK_DATA_DIR` 指定目录（相对路径基于当前工作目录）。
2. Windows 的 `bin/aglink.exe` 对应安装根有有效的本产品普通安装 receipt：使用 `<installRoot>/data/`，不受 `LOCALAPPDATA` 或数据库是否存在影响。receipt 损坏或无法读取时明确报错。
3. 可执行文件同目录**已存在**的 `agent-linker.db`（便携模式）。
4. 平台默认目录：Windows `%LOCALAPPDATA%/agent-linker/`；macOS `~/Library/Application Support/agent-linker/`；Linux `$XDG_DATA_HOME/agent-linker/`（未设置时为 `~/.local/share/agent-linker/`）。

源码或手工解压后先运行 `aglink db migrate`。日常使用普通用户身份；Agent 持久化写操作默认经宿主授权在沙箱外执行，已初始化库的只读查询与链接 dry-run 可在沙箱内执行。CLI 权限不足明确失败，不切换数据库或自行解除沙箱。显式设置不同 `AGLINK_DATA_DIR` 是主动切换数据配置；非正常安装形态跨入口使用时应显式统一该目录。

## Harness 兼容适配

管理 AI Agent 宿主系统（如 Claude Code）的兼容路径映射，不负责启停宿主软件（旧 `framework` 命令已移除）。

```text
aglink harness list
aglink harness show <harness>
aglink harness enable <harness>
aglink harness disable <harness>
aglink harness mapping list [harness]
aglink harness mapping add <harness> <source> <link> --kind <file|dir>
aglink harness mapping remove <harness> <link>
```

- `<harness>` 为已有名称或 ID（仅内置 `claude`，不支持新增）。`enable`/`disable` 修改全局启用标记。
- **`mapping`**：`<source>` 与 `<link>` 为项目相对路径（`add` 不校验 `<source>` 是否存在，`--kind` 接受 `file`、`dir` 或 `directory`）。`mapping list` 省略 `<harness>` 时列出全部。同 Harness 下 `<link>` 不可重复；标记为 `required` 的内置映射不可删。
- 全局设置供后续 `init` 只读消费，不反向改变已有项目。无启用 Harness 时不创建兼容链接；缺库回退内置 Claude 默认映射且不建库。旧/新 schema 等其他存储错误正常返回。

## 综合示例

假设在项目根目录执行，`../skills/writer/SKILL.md` 非空，`../guide.md` 存在：

```bash
aglink db migrate
aglink init
aglink skill add ../skills/writer --name writer --alias writing # 默认链接名为 writing
aglink link writer --as draft                                   # 临时覆盖：创建 .agents/skills/draft -> ../skills/writer
aglink unlink .agents/skills/draft                              # 删除链接与清单记录，保留源与注册项

aglink resource add ../guide.md --target-dir docs               # 推导注册名 guide，默认路径 docs/guide.md
aglink link guide --target-dir references --as handbook.md      # 临时覆盖：创建 references/handbook.md -> ../guide.md
aglink unlink references/handbook.md
```

SQLite 写命令以整次命令为事务边界；group add/remove 对输入按 item_id 去重，add 已有成员跳过并提示。group list 仅聚合名称和成员数；show 保留完整成员。新 Item/Group ID 为 itm_<16hex>/grp_<16hex>，rename 保持 ID，不更改 source 或既有项目。verbose 时间为可读 UTC。
