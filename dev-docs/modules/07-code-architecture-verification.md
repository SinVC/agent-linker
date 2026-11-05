# Code Architecture & Verification

## 设计目标

以单 crate 内部模块组织 MVP，使 CLI、command 和 core 的职责边界清晰，并为后续拆分或扩展保留稳定入口。

## 职责边界

- CLI 层：参数解析、调用 command、格式化输出。
- command 层：解析项目/数据库路径，构造默认 Provider 与 ProjectCtx，按用例打开一次 Store，并调用 core。init 使用当前目录，其余项目操作向上定位项目根。
- core 层：领域模型、路径、错误、symlink、manifest、SQLite、registry、harness 和 linkable item。

## 架构约束

- MVP 不采用 Rust workspace 和多 crate。
- 当前阶段不实现运行时插件系统。
- command 层不直接写 SQL。
- command 层不直接读写 `.agents/links.toml`。
- command 层不直接调用平台 symlink API。
- 平台条件编译限制在 symlink 或极少数路径解析代码中。

## 模块契约

core 模块职责：

- `error`：NotFound、Conflict、DatabaseBusy、PermissionDenied、含路径与 io::Error 的 Io 等结构化错误；数据库、解析与 Symlink 保留可遍历 source，CLI verbose 展示原因链。
- `paths`：项目根、全局数据目录、数据库路径、portable 模式、环境变量和路径转换。
- `symlink`：Provider、链接类型、状态、共享无副作用判定与原子替换；Mock 全部 cfg(test)，Windows Broker 类型与实现整体 cfg(windows)，无外部 ln 执行后端。
- `manifest`：serde + toml 清单读写、兼容读取、schema/业务校验、原子保存及 init；后端审计字符串不参与 Provider 选择。
- `project_links`：基于 ProjectCtx 的 link_item、link_group、status、unlink、unlink_group、clean 唯一用例入口，复用传入 Store；共享 LinkPlan 读取一次清单、整批预检、执行后一次保存，dry-run 使用同一计划。
- `doctor`：独立诊断编排，直接消费 Store check 结构化状态并执行 Windows Broker 探测。
- init 的 Harness 与默认注册项阶段分别复用同一 LinkPlan 执行能力；来源与 manifest 保留 Harness/Item 身份及各自路径约束，不新增 Linkable Item 类型。
- `store`：统一 Store 的只读/读写入口、显式 migration、backup 和数据库诊断；普通业务不能隐式初始化。
- `registry`：Store 上的 Linkable Item/Group 持久化访问和事务，group list 用聚合摘要。
- `harness`：Store 上的 Harness Adapter、mapping 与唯一 builtin_harnesses 内置定义；Seed 只由 Store 显式迁移消费。
- `linkable`：Linkable Item 模型、业务约束、alias、kind 检测和默认 link path。

依赖规则：

- `cli` 可以依赖 `commands`。
- `commands` 可以依赖 `core`。
- `core` 不依赖 `commands` 或 `cli`。
- `commands` 之间避免直接互相依赖，共享逻辑下沉到 `core`。
- Store 底层 connection/write 仅 core 内可见，LinkPlan 与内部辅助私有；公开用例保留集成测试所需可见性。分层不使用 include_str 搜索源码的测试约束。

## 扩展策略

当前阶段只保证代码架构可扩展。

明确不做：

- 运行时插件系统。
- 第三方动态加载命令。
- 自定义 Linkable Item 类型。

允许扩展：

- 新 Agent Harness 通过 Harness Adapter 数据扩展。
- 新 mapping 通过 Harness Adapter 数据扩展；Schema v3 使用 harness_mappings，通过事务迁移保留旧 ID。
- 不为尚无消费者的 source ownership/repo 元数据保留字段。
- 后续可在不破坏 command 层的情况下替换或拆分 core 模块。

## 验收口径

- symlink provider 使用 cfg(test) mock 做单元测试，集成黑盒测试自带 Provider；测试和生产调用同一个核心用例，不保留生产测试套壳。
- 批量预检失败验证目录/Provider/Manifest 均未修改；提交时机通过 Provider 回调观察，不能用检索源码字符串代替行为。
- SQLite 使用临时数据库测试。
- manifest 使用临时目录测试。
- `init`、`link`、`status`、`clean` 使用集成测试覆盖核心流程。
- Windows Broker 后端使用独立集成测试，不阻塞 Unix 单元测试。
- 路径解析覆盖 Windows、macOS、Linux 默认数据目录和 portable 模式。

