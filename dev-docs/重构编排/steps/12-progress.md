# 12 · init 消费已启用 Harness 映射

状态：完成，阶段二范围内当前平台验收通过（2026-10-31）；平台限制与前置差异见未解决问题。

[修改范围与契约](../../重构计划/12-init-harness.md)

## 完成内容

- init 只作用于当前目录；已初始化库通过只读Store消费启用Harness mappings。
- 仅缺库回退到相同内置Claude定义，不创建全局目录或库；旧schema等其他错误正常返回。
- 动态映射复用既有受控路径、Symlink Core和Manifest保存入口；不自动创建任意source；启停设置不反向改项目。
- 测试使用显式临时库/mappings入口，与本机全局库隔离。

涉及：src/core/manifest.rs、harness.rs、相关模块契约。

## 隔离验证

独立 GPT-6.1 Sol high 子代理，仅获得目标路径/签名、类型、业务验收标准，不读取业务实现。旧内联测试机械提取为独立 *_tests.rs，便于遵守隔离；无效旧 config 测试移除，其通用覆盖保留。

- 缺库默认映射/不建库、关闭Claude/重新启用、自定义mapping、无启用Harness、旧schema报错通过。
- 临时Manifest/mock Provider覆盖Harness名称记录、幂等、真实路径冲突和路径逃逸；写锁期间只读mapping消费通过。

最终隔离验证全部退出 0：

- cargo fmt --check
- cargo check --locked
- cargo test --locked --quiet：80 单元测试通过、28 CLI 黑盒测试通过、2 项目发现测试通过；3 项既有真实 Broker 测试忽略；doc tests 0。
- cargo run --locked -- --help
- 文档相对链接目标存在性检查：0 缺失。

所有测试的编写、适配和执行均由独立 GPT-6.1 Sol high 子代理完成；未向其提供业务实现或允许其读取业务源码。

## 测试隔离修正

- 首轮旧 Manifest 测试沿用默认入口，遇本机旧 schema 报错；测试代理将这些测试改用显式临时 mappings/数据库后通过。最终测试不依赖本机全局库，不迁移或修改在役数据。

## 未解决问题

- 前置状态差异：TODO 阶段一 02/03 已勾选，但当前基线的 LinkRecord 仍使用 scope/harness_name，save_manifest 仍直接写文件，未见计划要求的 origin 字段和原子保存。阶段二仅复用当前记录/保存入口，不擅自重做阶段一；本轮“来源”验证仅确认 Harness 名称及映射记录，不等同于阶段一 origin 隔离验收。
- 无需要用户决策的阻塞。保留已有修改，不改在役数据。
