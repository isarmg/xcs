# 最新分支系统验收记录

2026-10-19，依据[确认后的分支规则](../../design/branch-system-logic.md)完成核心、持久化、接口、SMB 桥接和 Web 适配。[实现说明](../../reports/branch-system-implementation.md)列出行为及模块。

完整 Rust 工作区 **277 项**、Web **101 项**、独立 SMB SDK **6 项**、机器契约 **10 项**均通过；Linux 二进制构建完成。各日志中的测试范围分别计数，避免重复相加。

## 自动验证

| 材料 | 实际范围 |
| --- | --- |
| [完整 Rust 工作区](../../../test/evidence/branch-operations/native-workspace.log) | 临时存储池、工作区、保护与恢复、按路径的别名、HTTP、权限和 Unix 描述符 |
| [Linux 构建](../../../test/evidence/branch-operations/native-build.log) | `cargo build --locked --offline -p storage-admin` |
| [公共核心集成](../../../test/evidence/branch-operations/api/core-alias-tests.txt) | 共享文件夹复制、别名规范位置改名、无操作不加子树、双路径上下文、非当前根分支局部删除、128 字节公开分支改名、冲突回滚 |
| [API 与 SMB](branch-operations/api.md) | 真实临时核心、HTTP 与本机桥接协议；含合成操作回执 |
| [机器契约](../../../test/evidence/branch-operations/api/contract-tests.txt) | schema、OpenAPI、引用链接与补充语义 |
| [Web 验收](branch-operations/web-README.md) | 101 项 Rust 测试、实际生产 WASM；中文桌面／英文手机各 9 组交互，另有目录和版本树回归 |

核心新增回归还覆盖：首次同字节写入的独立节点、完整历史目录恢复、显式同节点选择、UTF-8 唯一命名、移动前后断点恢复、未选分支冻结、根组合切回、导出／导入／重启、首次继承保存后的同一写句柄继续操作，以及新目录选择对旧工作区的隔离。

所有业务写入测试使用临时存储池。浏览器加载真实候选 WASM，业务网络由本地夹具拦截，不写入正在使用的预览存储池。浏览器结果证明界面及请求；核心、HTTP、SMB 和恢复由对应真实实现测试证明。

这不是三块物理设备的断电验收、已部署 smbd 的客户端矩阵、六平台设备验收或正式发行记录。

新增改名路径节点验证：文件／文件夹改名复用原正文，文件夹内每个文件分别有节点；后续真实写入接在路径节点后。路径备注、来源分支、混合节点 DAG、历史正文清除后元数据保留、导出恢复及原正文 inode／证明不变由核心和 HTTP 用例验证；[浏览器路径节点记录](../../../test/evidence/branch-operations/web-path-nodes/browser-result.json)使用真实生产 WASM，检查节点与正文数量、独立备注、来源节点及下载版本。

## 本地预览

[http://127.0.0.1:7400](http://127.0.0.1:7400) 已恢复并返回 HTTP 200，使用原有存储目录，业务测试没有写入该存储池。

- [候选程序与源码摘要](../../../test/evidence/branch-operations/candidate-receipt.json)
- [正常停止、备份及原目录重启记录](../../../test/evidence/branch-operations/preview-restart.json)
- [实际服务与资源核验](../../../test/evidence/branch-operations/served-assets.json)

核验确认新进程的 Linux 二进制匹配候选；首页、16 个 CSS、2 个 JavaScript、WASM 和两份字体全部与发布文件一致，严格 CSP 生效，146 个 Web 构建输入与当前源码一致。此记录为本地预览更新，不是正式发行。
