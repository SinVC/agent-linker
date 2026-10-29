# 22 · 裁决覆盖核对与故障回归验收

返回 [TODO](README.md)。前置：01—21 均完成其各自验收。依据：第八章阶段三第 4 项及第四至九章的最终行为。条件文档：[07](../modules/07-code-architecture-verification.md)，具体故障按被验证步骤引用的模块契约读取。

## 本步结果与边界

汇总已实现修改的证据，并补齐裁决明确要求但前序仍缺失的故障/边界测试。本步不新增业务设计，不重跑已经通过且没有新变化或未解决问题的同类检查；发现未完成实现时回到所属步骤处理。

## 代码入口

- [TestDir](../../src/core/test_support.rs#L1)、[Symlink 单元/平台测试](../../src/core/symlink.rs#L1007)。
- [Manifest 测试](../../src/core/manifest.rs#L674)、[Registry 测试](../../src/core/registry.rs#L747)、[项目用例测试](../../src/core/project_links.rs#L1208)。
- 原 [存储迁移测试](../../src/core/db.rs#L1025)由 06/09 迁至 Store；[Doctor 原测试](../../src/core/project_links.rs#L1842)由 18 迁至 Doctor。
- [CLI 测试](../../src/cli.rs#L1090)、[Windows 安装 fixture](../../packaging/windows/test-install.ps1#L1)、[Unix 安装 fixture](../../packaging/unix/test-install.sh#L1)。
- [已发现的 Cargo 配置](../../Cargo.toml#L1)、[现有 release 验证命令](../../.github/workflows/release.yml#L36)。

## 必须闭合的故障场景

| 场景 | 对应步骤与验收重点 |
| --- | --- |
| 项目外路径 | 01：绝对路径、盘符、`..`、重复 link_path 在临时清单中拒绝 |
| 祖先目录替换为 symlink | 01：`.agents` 和普通受控父目录指向外部时，读写/状态/解绑/清理不越界 |
| 同名和 alias 交叉 | 05/09：alias 不参与查询、name 全局唯一、Resource 不同目录同名可共存 |
| Harness 解绑隔离 | 02：unlink all 留下 Harness，group unlink 仅解绑当前成员的 item 链接 |
| force 建链失败 | 04/15：wrong/broken 替换失败仍保留旧链，真实文件目录不被替换 |
| 组内同路径冲突 | 16：dry-run 和实际执行都在完整 Plan 校验阶段检出，无提前建目录/建链 |
| 批量数据库写失败 | 07：中途故障事务回滚；group 输入去重和已在组内跳过正常 |
| v2 存量库升级 | 09：以本机在役数据相同形态构建临时 v2 fixture，含旧 ID、技能、分组、成员、Harness 状态、mapping 和旧 Manifest；升级无损且旧 item_id 可解绑 |

## 第四至九章修改覆盖表

| 裁决位置 | 实施步骤 |
| --- | --- |
| 四 P0-1：路径与祖先 symlink | 01 |
| 四 P0-2：标识符/alias、全局名称与默认路径唯一 | 05、09 |
| 四 P0-3：origin、unlink all、group unlink | 02 |
| 四 P0-4：原子替换、断链 force | 04、15 |
| 四 P0-5：外键、事务、批量 Plan、原子 Manifest | 03、06、07、16 |
| 五 1：五张表、两个部分唯一索引、字段与时间 | 09 |
| 五 2：分组聚合、约束替代预查、check/退出码 | 09、10、11 |
| 六 1：config/死字段/死枚举/scope/未知审计值 | 02、08、09、14、19、21 |
| 六 2：toml、bool 原生读写、i64 时间/UTC 输出 | 09、14 |
| 六 3：Mock/平台条件编译、删除源码检索测试 | 21 |
| 六 4：clap 重复校验、group 幂等、Windows 路径、创建选项 | 01、07、15、20 |
| 七：paths/Store/registry/harness/manifest/project_links/doctor 目标职责 | 01、06、09、12、14、16、17、18 |
| 七 1：ProjectCtx、唯一入口、命令组装/连接复用 | 17 |
| 七 2：共享纯判定与 dry-run 执行链路 | 15、16 |
| 七 3：CLI 公共结构和 group 命令归一 | 20 |
| 七 4：结构化错误与 verbose 源链 | 19 |
| 八：三阶段路线、显式 migrate 分发、故障验收 | TODO 顺序、06、12、13、本步 |
| 九 1：整体忽略 .claude/ 与 .agents/ | 02 |
| 九 2：rename 只更新全局库、旧项目依赖不可变 ID | 02、05、09 |

## 验收与交接

1. 核对上表每项实际实现和测试证据；缺失测试仅补该裁决相关场景。文件系统使用临时项目和 mock/平台测试，数据库使用临时库，不拿本机在役数据做破坏性验证。
2. 使用已发现的 `cargo fmt --check`、`cargo check --locked`、`cargo test --locked`、`cargo run --locked -- --help` 完成所需最终验证；Cargo 使用默认缓存并按根 AGENTS.md 直接提权。Windows Broker 与 Unix rename 分别记录真实平台验证，未执行不能写成通过。
3. 核对 `dev-docs/modules/01—07` 与架构导航已随所属步骤同步，不再把被移除能力描述为当前契约；仅修正文档遗漏，不重写历史审查或开发记录。
4. 检查本计划的文件/代码链接是否仍存在，源码移动后同步符号位置；根文档保留导航，详细变更和结果留在下钻文档。
5. 每个步骤填实际改动、验证命令、结果与未完成项。只有相关修改与验收都完成才勾选，不把本次“计划编写完成”等同于“重构完成”。

## 执行记录

未执行。完成时填写各故障场景证据、平台结果与最终未验收项；全部满足后勾选 TODO 22。
