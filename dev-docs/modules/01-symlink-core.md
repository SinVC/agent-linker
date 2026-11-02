# Symlink Core

## 设计目标

提供跨平台一致的 symlink 能力，所有会创建、删除、读取或检查链接状态的命令都必须通过本模块完成，不直接调用平台 API 或外部命令。

## 职责边界

- 定义链接类型、链接状态、链接操作和结构化错误。
- 封装 Unix、Windows Broker 与显式 Windows fallback 后端。
- 维护链接创建的幂等和冲突语义。
- 向上层返回结构化结果，不直接决定 CLI 文案。
- `evaluate_create` / `evaluate_remove` 纯函数统一状态、source 有效性和 force 的判定；项目 Plan 与实际创建/删除共用规则，不在纯函数中执行 Provider 修改、目录创建或保存。

## 架构约束

- Linux / macOS 默认使用 Rust 标准库创建 symlink。
- Windows 默认使用 `WinSymlinksBroker` IPC 能力创建 symlink。
- Windows 标准库 symlink 只能作为显式 fallback。
- fallback 后端必须显式配置或通过命令参数启用。

## Windows Broker 集成

Windows 默认 Provider 归 Symlink Core 所有，必须通过 `WinSymlinksBroker` 本地 IPC 创建真实 Windows symlink。外部集成细节参考 [win-symlinks-integration.md](../../docs/win-symlinks-integration.md)，但 Agent Linker 的架构约束以本节为准。

- Rust 实现优先通过 `win-symlinks-client` 的 broker-only 能力接入；不应先尝试直接创建再自动回退。
- 非 Rust 或兼容实现只能使用已文档化的本地 Named Pipe 协议，并必须验证连接的 pipe server 是已安装的 `WinSymlinksBroker` 服务进程。
- `create_symlink(source, link, kind)` 在 Broker 请求中映射为 target path、link path 和 target kind；Symlink Core 已知链接类型时必须显式传入 `file` 或 `directory`。
- 普通创建请求固定 `replace_existing_symlink = false`；替换请求固定为 `true`，只能在调用方明确启用 `--force` 且 `link_status` 已确认目标是 `wrong_symlink_target` 或 `broken_symlink` 时使用。替换前校验新 source 存在且 kind 合法，并再次拒绝真实文件或目录。
- Broker 后端只能创建真实 symlink，不允许退化为 junction、hardlink、文件或目录复制、`.lnk` 快捷方式。
- Broker 返回的稳定错误码必须保留在诊断信息中，并映射到 Symlink Core 的结构化错误分类；CLI 不解析 Broker 文案。

## 接口契约

Symlink Provider 必须提供以下能力：

- `validate_source(source, kind)`：校验创建所需的 source 与 kind，默认检查实际文件系统，测试 Provider 可使用自身条目；不修改链接。
- `create_symlink(source, link, kind)`：创建指定类型的 symlink。
- `replace_symlink(source, link, kind)`：校验新 source 和已有 symlink 后由 Provider 替换；上层不得先删除再创建。Unix Std 在 link 同目录创建临时 symlink，再以 `fs::rename` 原子覆盖；创建或 rename 失败保留旧链并清理本次临时链接。Windows 默认使用 Broker 原生替换，不新增后端或自动 fallback；显式 Std fallback 也采用同目录临时链接与 rename，系统不支持覆盖时返回错误并保留旧链。
- `remove_symlink(link)`：删除 symlink。
- `read_link(link)`：读取 symlink 指向。
- `link_status(source, link, kind)`：检查 link 相对 source 的状态。

链接类型：

- `file`
- `directory`

链接状态：

- `missing`
- `correct_symlink`
- `wrong_symlink_target`
- `broken_symlink`
- `existing_real_file`
- `existing_real_directory`
- `unsupported_file_type`

错误分类：

- `source_not_found`
- `link_parent_not_found`
- `link_already_exists`
- `wrong_symlink_target`
- `existing_real_file`
- `existing_real_directory`
- `permission_denied`
- `broker_unavailable`
- `broker_protocol_error`
- `unsupported_platform`
- `unsupported_link_kind`
- `io`

- `CreateSymlinkOptions` 仅含 `force` 字段，默认 false；不提供额外构造器或转换器。
- `SymlinkError` 保留底层错误作为 `std::error::Error::source`，同时保留 source/link 路径、系统码和 Broker 稳定码。
- Mock Provider 及专用辅助只在 `cfg(test)` 编译；Windows Broker 类型及完整实现只在 `cfg(windows)` 编译。历史后端名称只作为 Manifest 审计字符串保留。

## 关键决策

- 核心库依赖结构化 API 或 IPC，不依赖命令行文本协议。
- 链接创建必须幂等：正确链接视为成功，错误链接默认报错。
- `--force` 允许替换 `wrong_symlink_target` 或 `broken_symlink`，新 source 必须存在且 kind 合法；不允许删除真实文件或真实目录。Mock 支持替换故障注入，失败保留旧链。
- 断链仍按记录目标判断：目标一致但 source 缺失时报告 source 缺失，force 不绕过新 source 校验；目标不一致时按错误 symlink 处理，有效新 source 可在 force 下替换缺失的旧目标。
- link 父目录默认不隐式创建，是否创建由调用场景决定。
- Windows Broker 不可用时必须返回可诊断的结构化错误。

## 验收口径

- 所有链接相关命令均通过同一 Provider 抽象完成。
- Windows 默认路径不绕过 Broker。
- Windows 默认路径使用 Broker IPC，不使用直接创建后自动 fallback 的客户端路径。
- 真实文件或真实目录不会被 `--force` 删除。
- 错误目标及断链在无 force 时拒绝、在有效新 source 和 force 下替换；缺失或 kind 不合法的新 source 不能使旧链丢失。替换失败以故障注入验证保留旧链；Unix rename 与 Windows Broker 原生替换分别用真实平台测试验证。
- CLI 能将结构化错误转换为简洁、可执行的用户提示。
- `--verbose` 能展示后端、系统错误码和 Broker 诊断信息。
- Windows Broker 错误码映射使用 mock 或 fixture 覆盖，真实 Broker 后端使用独立集成测试。
