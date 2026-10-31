# 13 · 安装和打包的显式初始化

状态：完成，阶段二范围内当前平台验收通过（2026-10-31）；平台限制与前置差异见未解决问题。

[修改范围与契约](../../重构计划/13-distribution.md)

## 完成内容

- 核对正常安装/升级在目标数据目录显式migrate/check，Windows逐项检查退出码并finally恢复AGLINK_DATA_DIR；Unix set -eu传播失败。
- Portable打包在stage目录显式migrate，随包库为v3，首次普通查询可用；不改安装布局/ACL/Broker生命周期。
- 离线fixture补schema3和首次查询，临时转发exe分别注入migrate/check失败；源码手动使用说明要求显式migrate。

涉及：既有 packaging/windows/install.ps1、package-windows.ps1、packaging/unix/install.sh 调用链保留；测试fixture与当前用户文档同步。

## 隔离验证

独立 GPT-6.1 Sol high 子代理，仅获得目标路径/签名、类型、业务验收标准，不读取业务实现。旧内联测试机械提取为独立 *_tests.rs，便于遵守隔离；无效旧 config 测试移除，其通用覆盖保留。

- Windows默认临时fixture通过：Portable v3/首查、旧库导入/原库保留、升级/备份、override恢复、receipt路径及损坏目标保护。
- migrate/check分别注入exit47，均停止成功流程，marker证明到达被测命令。
- 本机PowerShell策略拦截原命令；实际通过命令：powershell -NoProfile -ExecutionPolicy Bypass -File packaging/windows/test-install.ps1 -AglinkExe F:\00_Workspace\AI_Agents\agent_linker\target\debug\aglink.exe。未改在役安装/数据库/Broker。
- Unix fixture 已补临时 HOME/cwd、v3/健康/普通首查以及分别注入 migrate/check exit17（marker 证明命令到达）；当前平台未运行。

最终隔离验证全部退出 0：

- cargo fmt --check
- cargo check --locked
- cargo test --locked --quiet：80 单元测试通过、28 CLI 黑盒测试通过、2 项目发现测试通过；3 项既有真实 Broker 测试忽略；doc tests 0。
- cargo run --locked -- --help
- 文档相对链接目标存在性检查：0 缺失。

所有测试的编写、适配和执行均由独立 GPT-6.1 Sol high 子代理完成；未向其提供业务实现或允许其读取业务源码。

## 未解决问题

- 覆盖界限：本轮只运行默认临时 Windows 安装 fixture；真实 Broker、ProgramFiles 及在役跨入口测试未运行。3 个既有 Broker 用例按原平台条件忽略。
- Unix 安装fixture须在Linux平台执行；当前Windows环境未验收，不能声称通过。
- 无需要用户决策的阻塞。保留已有修改，不改在役数据。

## 提交范围

用户于 2026-10-31 授权提交。阶段二代码、隔离测试、契约和进度一并提交；Windows 安装/卸载、维护脚本及配套文档包含实施开始前已有、且本次完整安装验收依赖的修改，以保留受验调用链。独立的 01-paths.md 历史记录删除保留在工作区，不纳入本次提交。提交消息：refactor: complete stage two store and schema v3。
