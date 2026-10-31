# 13 · 安装和打包的显式初始化

返回 [TODO](README.md)。前置：[06](06-store.md)、[09](09-schema-v3.md)、[11](11-db-check.md)、[12](12-init-harness.md)。依据：第八章阶段二第 5 项。条件文档：[04](../modules/04-global-store-registry.md)、[06](../modules/06-cli-command-surface.md)。分发细节实施时仅核对[分发契约](../release-distribution.md)与[现有安装布局交接](../handoff-windows-install-data-layout.md)中与初始化相关的部分。

## 本步结果与边界

保证源码外的现有分发形态在安装或构建期显式完成迁移，普通查询和写命令不再承担初始化。只补齐或核对 migrate/check 链路，不重做安装布局、ACL、Broker 生命周期或发行流程。

## 代码入口

| 当前文件 | 核对位置 |
| --- | --- |
| [Windows 安装](../../packaging/windows/install.ps1#L123) | `AGLINK_DATA_DIR` 的设置/恢复，`db migrate` 与 `db check` |
| [Windows Portable 打包](../../packaging/windows/package-windows.ps1#L75) | 使用 stage 数据目录运行 `db migrate` |
| [Unix 安装](../../packaging/unix/install.sh#L56) | 安装后显式 migrate/check |
| [Release workflow](../../.github/workflows/release.yml#L20) | 分发形态、构建、打包调用链 |
| [Windows 安装 fixture](../../packaging/windows/test-install.ps1#L1) | 正常安装、Portable、升级和初始化失败场景 |
| [Unix 安装 fixture](../../packaging/unix/test-install.sh#L1) | 离线安装和显式初始化场景 |

## 修改清单

1. 逐一核对现有正常安装、Portable 打包和 Unix 安装的实际调用链；已满足显式 migrate 的部分保留，不为重构而重写脚本。
2. 安装/升级使用目标分发的 aglink 在正确数据目录执行 `db migrate`，并检查退出码；`db check` 非健康返回非零时，现有失败路径能够停止安装成功流程。
3. Portable 在构建/打包时创建随包数据库并迁移到 v3，初次普通查询不依赖写库。正常安装在安装期迁移，源码调试首跑明确显式执行一次 `db migrate`。
4. 更新既有离线 fixture，使其验证 v3、普通首个查询可用及失败退出码；保持既有环境覆盖恢复语义。
5. 仅同步 [README](../../README.md)、[命令参考](../../CLI_REFERENCE.md)、[中文命令参考](../../CLI_REFERENCE_zh.md)、[发布清单](../../RELEASE_CHECKLIST.md)及分发文档中与显式初始化、schema 和移除 config 直接相关的当前用法。

## 验收

- Windows 使用已构建的测试二进制运行 `powershell -NoProfile -File packaging/windows/test-install.ps1 -AglinkExe <绝对路径>` 的默认临时 fixture。
- Unix 在对应平台运行 `sh packaging/unix/test-install.sh <已构建二进制绝对路径>`。命令和参数来自现有脚本，不另造测试入口。
- 正常安装、升级、Portable 初次查询均使用已迁移库；注入 migrate/check 失败不能报告安装成功。
- 脚本验证限于临时安装 fixture，不对本机在役安装和数据执行升级；无法运行的平台记录未验收项。

## 执行记录

进度、验证结果与未解决问题见[本步进度](../重构编排/steps/13-progress.md)。
