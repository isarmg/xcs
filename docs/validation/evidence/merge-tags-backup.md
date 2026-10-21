# 文件合并、节点标签与备份前缀验证

> 后续范围说明：独立分支演示及其图示、构建和素材采集已于 2026-10-20 移除。本文涉及这些内容的路径、命令、计数及产物仅作历史记录；业务分支规则与存储验收分别保留。见[演示清理记录](../../reports/demo-removal-20261020.md)。

2026-10-19，依据[用户确认规则](../../design/file-merge-tags-backup.md)新增核心、持久化、接口、Web 与图示。目录分支合并保持待确认，不提供写接口。

最终冻结源码的 Rust 主工作区 **299 项**、Web **111 项**、独立 SMB／SDK 工作区 **11 项**、机器契约 **11 项**均通过；Linux 服务程序构建和格式检查完成。主工作区已包含下述核心／API／存储子集，不能将这些子集再次相加。

## 证据范围

| 材料 | 范围 |
| --- | --- |
| [Rust 工作区最终回归](../../../test/evidence/merge-tags-backup/native-workspace-final.log) | 299 项；真实临时存储池、节点／目录引用、标签继承、合并复用和只读别名、备份回填、历史包、恢复、HTTP 与本机描述符 |
| [Linux 二进制构建](../../../test/evidence/merge-tags-backup/native-build.log) | 当前 CLI／Web 服务程序构建 |
| [API、SDK 与契约](merge-tags-backup/api.md) | 请求基线、归属确认、历史路径、批量标签、稳定备份范围、权限与状态码 |
| [SMB／SDK 工作区](../../../test/evidence/merge-tags-backup/smb-sdk.log) | 协议 6、真实 SDK 模块 5；未启动实际 smbd。本次补齐 SDK 头文件和共享库搜索路径后的最终运行通过 |
| [存储与恢复](merge-tags-backup/storage.md) | 任意固定链份数、全历史回填、保留子策略、满容量复用、故障重放、原证明不变及含标签／合并／备份的历史包恢复 |
| [Web](merge-tags-backup/web.md) | Rust 测试、实际生产 WASM、中英文桌面／手机隔离交互和既有功能回归 |
| 文档图示（原路径 `../../../examples/branch-system/tests/history/merge-tags-backup/diagrams-result.json`，演示材料已移除） | 27 张 SVG／PNG，父下子上、无碰撞等粗正交箭头、文字边界及三种宽度的 81 次切换 |

最后的备份浏览修复另验证：用户前缀直接映射只读目录、主盘改名后查询上下文更新而备份路径不变、主盘已有较新正文时仍读取本槽旧副本、删除后的历史容器下载、先授权后检查路径冲突，以及同文件历史来自多个主盘链时的前缀并集和权限可见性。Web 在最终 WASM 上完成 12 个语言／尺寸场景，共 72 组行为与几何回归。

业务写入均使用临时池；浏览器加载真实候选 WASM，业务请求由夹具拦截。浏览器结果证明界面和请求，真实发布与恢复由 Rust／HTTP 测试验证。临时目录不是独立物理盘的断电验收，也不代表已部署 smbd、六平台设备验收或正式发行。

## 本地预览核验

最终预览已就绪并返回 HTTP 200：[http://127.0.0.1:7400](http://127.0.0.1:7400)。[候选摘要](../../../test/evidence/merge-tags-backup/candidate-receipt.json)、[正常替换记录](../../../test/evidence/merge-tags-backup/preview-restart.json)及[实际资源核验](../../../test/evidence/merge-tags-backup/served-assets.json)确认最终 Linux 二进制、21 个静态资源、严格 CSP 与 152 个 Web 构建输入匹配；270 个源码／契约文件摘要一致。进程使用原有池，业务测试不写入该池。

[真实浏览器检查](../../../test/evidence/merge-tags-backup/served-browser.json)以无凭据会话加载登录页，核对实际 WASM 摘要与运行标记，拦截全部写请求；新功能业务交互仍由上述隔离测试验证。当前程序与页面的替换前备份位于 `runtime/web-preview/rollback-before-merge-tags-20261019T115453Z/`。`interim-*` 记录保留了备份浏览修正前的一次中间预览，不作为最终候选记录。
