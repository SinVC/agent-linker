# CLI Command Surface

## 设计目标

提供“类型注册 + 统一链接”的命令体系，让全局 Registry 管理和当前项目链接状态管理保持分离。

## 职责边界

- CLI 层负责参数解析、调用 command 和格式化输出。Skill/Resource 共用子命令与转换，Resource add 保留必填 target-dir；互斥与必填由 clap 负责。
- `group link/unlink` 和 `link/unlink --group` 解析后分别归一为相同 Command，用同一个分发入口。
- command 层负责路径定位、默认 Provider/ProjectCtx 组装及一次 Store 打开，再调用 core 用例；status/普通 unlink/clean 不依赖全局库。
- 底层存储、manifest 和 symlink 操作必须通过 core 能力完成。

## 架构约束

- 不提供 `skill link` 或 `resource link` 作为主路径。
- 链接行为统一走 `aglink link`。
- 会修改链接状态的命令除已声明 MVP 例外外，应支持 `--dry-run`。
- `test` 不作为正式顶层命令。

## 顶层命令

```text
aglink init
aglink init defaults list
aglink init defaults add <item>
aglink init defaults remove <item>
aglink db ...
aglink harness ...
aglink skill ...
aglink resource ...
aglink group ...
aglink link ...
aglink unlink ...
aglink status
aglink clean
aglink doctor
```

## 命令契约

全局输出选项：

```text
aglink --quiet <command>
aglink --verbose <command>
```

数据库命令：

```text
aglink db path
aglink db migrate
aglink db backup [path]
aglink db check
```

Harness 适配命令（旧 `framework` 命令已移除，无兼容别名）：

```text
aglink harness list
aglink harness show <name>
aglink harness enable <name>
aglink harness disable <name>
aglink harness mapping list [name]
aglink harness mapping add <name> <source> <link> --kind <file|dir>
aglink harness mapping remove <name> <link>
```

Skill 命令：

```text
aglink skill add <path> [--name <name>] [--alias <link-name>]
aglink skill list
aglink skill show <name>
aglink skill rename <old> <new>
aglink skill remove <name>
aglink skill refresh <name>
```

Resource 命令：

```text
aglink resource add <path> --target-dir <project-relative-dir> [--name <name>] [--alias <link-name>]
aglink resource list
aglink resource show <name>
aglink resource rename <old> <new>
aglink resource remove <name>
aglink resource refresh <name>
```

Group 命令：

```text
aglink group create <name>
aglink group list
aglink group show <name>
aglink group rename <old> <new>
aglink group delete <name>
aglink group add <group> <item>...
aglink group remove <group> <item>...
aglink group link <group>
aglink group link <group> --dry-run
aglink group unlink <group>
aglink group unlink <group> --dry-run
```

链接命令：

```text
aglink link <name>
aglink link <name> --as <link-name>
aglink link <name> --target-dir <dir>
aglink link <name> --force
aglink link <name> --dry-run
aglink link --group <group>
aglink link --group <group> --dry-run
```

取消链接命令：

```text
aglink unlink <name>
aglink unlink --group <group>
aglink unlink --all
aglink unlink <name> --dry-run
aglink unlink --group <group> --dry-run
aglink unlink --all --dry-run
```

状态、清理、诊断：

```text
aglink status
aglink status --json
aglink clean
aglink clean --broken
aglink clean --missing-source
aglink clean --dry-run
aglink doctor
```

## 关键决策

- Harness 指 Claude Code 等 Agent 运行系统，`harness` 管理其兼容路径与全局适配设置，不启停工具进程。
- `init` 只读消费已启用 Harness 映射；缺库使用内置 Claude 默认映射，不隐式初始化全局库。其他存储错误正常返回。
- `link` 根据 Registry 中的 Linkable Item 类型决定链接目标。
- `init` 依次建立基础结构、Harness 兼容链接、默认注册项链接。`init defaults` 管理全局 Skill/Resource 默认项，不操作当前项目；接受注册名称或 ID，配置按稳定 ID 关联。
- `--as` 只覆盖本次链接名，不修改 Registry alias。
- `--target-dir` 只用于 Resource 的本次链接目标覆盖。
- `--force` 以有效且类型匹配的新 source 替换错误目标或旧目标已缺失的 symlink；由 Provider 替换，不先删后建，不允许删除真实文件或目录。
- `--dry-run` 覆盖 `link`、`unlink`、`clean`、`group link` 和 `group unlink`；`init` 在 MVP 中暂不支持 dry run。
- `status --json` 的 `links[].status` 使用稳定 snake_case 机器值：`missing`、`correct_symlink`、`wrong_symlink_target`、`broken_symlink`、`existing_real_file`、`existing_real_directory`、`unsupported_file_type`。
- `unlink` 和 `clean` 只处理 manifest 中由 `aglink` 管理的 symlink。
- Manifest 来源为 harness 或 item，旧 registry 哨兵兼容推断为 item；旧 scope 不再写出。`unlink --all` 只解绑 item，group unlink 按当前成员 item_id 选择其全部 item 链接，保留 Harness。
- alias 不参与注册项标识符查询；rename 保留 item_id、已有项目 Manifest、路径和 source。`init` 的 gitignore 托管项为 `.claude/`、`.agents/`，保留块外内容。
- 项目操作统一使用受控路径入口；`.agents` 或链接、相对 source 的已有父目录为 symlink 时失败，不访问其外部目标。
- `doctor` 由 `core/doctor.rs` 独立编排数据库、migration、manifest、Harness Adapter、Windows Broker 和 symlink 后端诊断。
- 临时验证场景由 `aglink link <name>` 和后续可能的 `aglink link --local <path>` 承担，`--local` 暂不进入正式命令集。

## 验收口径

- 全局 Registry 命令不直接改变当前项目链接状态。
- 当前项目链接命令必须写入或读取 `.agents/links.toml`。
- `link`、`unlink`、`status`、`clean`、`doctor` 及 group 链接命令从当前目录向上定位最近的 `.agents/links.toml`，遇文件系统根或 symlink 目录停止；未找到时提示先运行 `init`。`init` 严格使用当前目录，不上查。
- 用户输出默认简洁，`--verbose` 遍历错误 source 链并展示路径、系统码及 Broker 稳定码，首层原因与顶层错误文案完全相同时省略该重复行，继续展示后续原因；`--quiet` 保留必要错误。NotFound、Conflict、DatabaseBusy、PermissionDenied 和含路径的 Io 保持结构化分类。
- 成功操作输出创建、跳过或更新等摘要。link/group link/unlink/group unlink/clean 共用 Plan → Validate → Execute；整批预检失败无目录/链接/清单副作用，dry-run 返回相同计划，成功后清单只原子保存一次。
- 普通全局读写要求显式 db migrate，缺库/旧 schema 提示迁移，新 schema 提示升级。config 命令已移除，路径诊断使用 db path。
- SQLite 写命令整体事务提交；group add/remove 输入按 item_id 去重，add 已有成员跳过并提示，未知项使整次事务失败。group list 只输出名称和成员数，show 保留完整成员。
- db check 不建库/迁移/Seed，检查当前 schema 的五张核心表及 quick_check，行数不决定健康。Missing、SchemaMismatch、ReadOnly、LockedBusy、Invalid 返回非零，HealthyWritable 返回 0；Doctor 直接消费该状态。
- 时间戳为 i64 Unix 秒，verbose 格式化为可读 UTC。
