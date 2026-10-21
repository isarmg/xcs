# 项目目录与忽略规则整理

> 后续范围说明：独立分支演示及其图示、构建和素材采集已于 2026-10-20 移除。本文涉及这些内容的路径、命令、计数及产物仅作历史记录；业务分支规则与存储验收分别保留。见[演示清理记录](../../reports/demo-removal-20261020.md)。

2026-10-19，范围为本项目目录、引用、独立演示工具的入口与 Git 忽略策略。源码工作区、产品 API、真实运行池与文件内容的行为不在本次修改范围。

此处保留上一轮整理时的执行结论；用户随后确认排除验证生成物，最新忽略策略见[补充记录](gitignore.md)。本记录中的 JSON、截图与日志链接指向本机或独立归档材料，不再要求入库。

- 25 章规范手册保留原编号路径；专题规则移入 `docs/design/`，实现和检查报告移入 `docs/reports/`，模板移入 `docs/templates/`。新增[文档导航](../../README.md)，40 份 Markdown 的引用同步调整。
- 演示构建、只读服务与静态图生成统一为 `demos/branch-system/src/bin/` 下三个显式 Cargo 目标。静态图生成根据 manifest 定位目录，支持从其他当前目录执行。
- 上一轮根 `.gitignore` 以 `target/` 覆盖所有工作区，并保留正式验收日志。该日志保留策略现已按用户确认改为排除全部验证生成物，最新规则及复查见上方补充记录；不全局忽略图片、数据库或压缩包。
- 运行目录、客户端待提交数据、已有构建缓存与历史证据均保留。迁移清单及移动前 SHA-256 见[清单](../../../test/evidence/project-layout/moves.json)。

验证：

| 材料 | 检查范围 |
| --- | --- |
| [契约与规则测试](../../../test/evidence/project-layout/contracts.log) | 11 项契约测试、1 项忽略策略测试；后者使用 85 个正反路径；文档契约校验 |
| [目录扫描](../../../test/evidence/project-layout/layout-check.json) | 本项目文档链接存在性、真实 Rust/锁文件/字体/图标未被错误忽略 |
| [演示测试](../../../test/evidence/project-layout/demo-tests.log) | 14 项内存模型测试、1 项只读路径测试，三个工具入口均编译 |
| [图形生成](../../../test/evidence/project-layout/demo-reference.log) | 用新的 Cargo 目标实际重建 27 个参考实例和页面 |
| [跨工作目录生成](../../../test/evidence/project-layout/demo-reference-other-cwd.log) | 从 `/tmp` 使用绝对 manifest 路径运行图示生成工具，输出仍落在演示目录 |
| [演示构建](../../../test/evidence/project-layout/demo-build.log) | 从新目录构建独立 WASM 发行包并更新现有预览的 `/demo/` |
| 浏览器复查（原路径 `../../../examples/branch-system/test/evidence/browser-result.json`，演示材料已移除） | 8 个过程、14 组自由操作、三种屏宽及 27 个参考实例切换；没有业务请求 |
| 独立服务复查（原路径 `../../../examples/branch-system/test/evidence/server-result.json`，演示材料已移除） | 新 `src/bin/serve.rs` 实际构建的只读 HTTP 服务、路径限制和资源类型 |
| 发行摘要核对（原路径 `../../../examples/branch-system/test/evidence/delivery.json`，演示材料已移除） | 演示输入、产物及预览副本摘要一致；原 Web 文件继续匹配原产物清单 |

目录边界及后续放置约定见[工程规范](../../20-engineering.md)。此处记录组织和构建验证，不替代存储及多平台验收。
