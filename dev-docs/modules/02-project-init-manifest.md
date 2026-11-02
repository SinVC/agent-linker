# Project Init & Manifest

## 设计目标

`aglink init` 在当前工作目录建立 Agent Linker 项目结构，并通过 manifest 记录该项目实际由 `aglink` 管理的 symlink 状态。

## 职责边界

- 创建或确认项目真实源结构。
- 创建 Harness 兼容 symlink。
- 维护 `.agents/links.toml`。
- 维护 `.gitignore` 中的 aglink managed block。
- 保证重复执行幂等且不覆盖用户真实文件或目录。

## 架构约束

- `aglink init` 只作用于当前工作目录，不支持 `aglink init <path>`。
- MVP 阶段 `init` 不提供 `--dry-run`、`--force`；支持全局 `--quiet` / `--verbose`。
- `AGENTS.md` 与 `.agents/skills/` 必须是真实源。
- `CLAUDE.md` 与 `.claude/skills/` 等 Harness 路径必须是兼容性 symlink。
- 不创建 `.agents/config.toml` 或其他项目级配置文件。

## 接口契约

`init` 至少创建或确认：

- `AGENTS.md`
- `.agents/`
- `.agents/skills/`
- `.agents/links.toml`

内置 Claude Code 映射（ID 为 `claude`）：

- `AGENTS.md` -> `CLAUDE.md`
- `.agents/skills/` -> `.claude/skills/`

manifest 固定路径：

- `.agents/links.toml`

manifest 至少记录：

- `id`
- `origin`（`harness` 或 `item`）
- `harness name`（仅 Harness 来源有值；磁盘字段为 `framework_name`）
- `item id`
- `item name`
- `source path`
- `link path`
- `link kind`
- `provider backend`
- `created by command`
- `created at`
- `updated at`

## 关键决策

- manifest 是项目链接状态清单，不是项目级配置文件。
- 使用 serde + toml 读写，保留旧来源、scope 和时间兼容，业务合法性仍由 validate 检查；不维护手写 TOML 解析/转义器。绝对 source 保留原生分隔符，Windows 相对链接路径写为 `/`，Unix 保留合法文件名中的反斜杠。
- `provider_backend` 是审计字符串，未知值及历史 `external-ln` 原样往返，不影响 status、unlink 或 clean；Provider 选择不消费该字段。
- Harness 基础设施使用 `origin = "harness"`，注册项使用 `origin = "item"`；item 不持有或写出 Harness 名称。旧 `framework_name = "registry"` 推断为 item，其余旧 Harness 条目推断为 harness；旧 `scope` 可读取但不再写出，ID、路径和时间保留。
- `init` 对已初始化全局库只读加载已启用 Harness 映射；缺库时使用同一 builtin_harnesses 定义的 Claude 默认映射，不创建目录/库、不迁移或 Seed；旧 schema 等其他错误正常返回。无启用 Harness 时不强制恢复 Claude。
- mapping 复用受控项目路径、Symlink Core 和 Manifest 保存入口；不自动创建任意 mapping source。全局启停和 mapping 变更不反向更改已有项目。
- Manifest 时间使用 i64 Unix 秒并写为整数，旧 unix:<seconds> 字段仍可读。
- manifest 只记录 `aglink` 创建或接管管理的 symlink。
- manifest 支持 `status`、`unlink`、`clean`、审计和恢复。
- manifest 已存在且合法时合并或更新；损坏时失败且不覆盖。同一 item 来源、item_id 和受控路径键的记录在重链时复用原 ID 与 created_at。
- 完整内容写入同目录 `.agents/.links.toml.tmp.<pid>` 后关闭并以 `fs::rename` 替换清单，不先删除原文件；独占创建临时文件，写入或替换失败保留原清单并清理本次临时文件。
- Manifest 的 `link_path` 必须是非空项目相对路径，不含绝对根、盘符前缀或 `..` 组件；新计算的路径剥离 `.` 组件。规范化相对路径后保持唯一，Windows 按大小写无关判重。
- 创建、读取、状态检查、解绑、清理及 Manifest 读写统一通过 `core::paths` 解析受控路径；逐级检查已有父目录为真实目录，`.agents` 也必须为真实目录。链接末级允许为 symlink，Manifest 本体必须为真实文件。
- Manifest 加载和保存同时验证记录中链接路径、项目相对 source 的祖先边界；绝对 source 可以位于项目外。路径解析本身不创建目录。
- link/group link/unlink/group unlink/clean 读取一次当前清单，在同一 LinkPlan 中完成整批路径、source、占用及组内冲突预检；待建父目录按受控路径键去重，created_dirs 使用同一口径。dry-run 返回该计划结果，不创建目录、不修改链接、不保存。校验通过后执行全部变更，最终只原子保存一次 Manifest。运行期 Provider 故障不承诺跨文件系统和 SQLite 的统一回滚。
- `.gitignore` 由 managed block 管理，不覆盖用户手写内容。
- 托管 ignore patterns 固定为 `.claude/`、`.agents/`；更新时保留块外原有内容。共享 `AGENTS.md` 不加入托管忽略。
- `unlink --all` 只选择 item；分组解绑按当前成员 item_id 选择其全部 item 链接（包括不同 `--as`），保留 Harness 与分组外记录。

## 冲突规则

- `AGENTS.md` 不存在时创建空文件；存在真实文件时保留；存在目录或 symlink 时报错。
- `.agents/` 不存在时创建；存在真实目录时保留；存在 symlink 或非目录时报错。
- `.agents/skills/` 不存在时创建真实目录；存在目录时保留；存在非目录时报错。
- Harness 兼容 link 不存在时创建；已是正确 symlink 时成功；错误 symlink、真实文件或真实目录时报错。

## 验收口径

- 重复执行 `init` 不改变用户已有真实文件内容。
- Harness 兼容链接全部记录到 manifest。
- `.gitignore` 中用户内容保持不变，managed block 可追加或更新。
- 全局数据库不可用或迁移后，项目内已管理链接仍可通过 manifest 识别。

