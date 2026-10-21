# ZXS 验证体系

详细目录与来源约束见[测试结构](architecture/test-layout.md)，迁移核对见[测试分离记录](reports/test-layout-20261021.md)。

测试实现统一位于仓库根 `test/`，按原组件和测试类型组织；通过原 Cargo 包的测试入口访问业务实现，不建立重复业务规则，也不为了外置单元测试扩大生产 API。

| 位置 | 范围 |
| --- | --- |
| `test/contract/unit/encoding/` | 严格 JSON、JCS、二进制校验清单及公开固定向量 |
| `test/naming/unit/` | 固定 Unicode、跨平台名称和路径预算 |
| `test/auth/unit/` | 两端共用的密码、令牌和 CSRF 原语及准入规则 |
| `test/client/unit/` | 真实本机 HTTP 下的流式校验下载、无覆盖发布和错误语义 |
| `test/platform/unit/` | 目录句柄、链接拒绝、原子落盘、SQLite 和凭据基础 |
| `test/engine/unit/` | 随模块维护真实文件、三成员日志、断点恢复、授权、独立历史包及容量拒绝回归 |
| `test/server/integration/` | 实际存储核心和 HTTP 请求的贯通验证 |
| `test/server/unit/smb_rpc/` | 本机套接字、FD 转交、句柄、读租约及保存边界 |
| `test/smb/unit/` | 受限帧协议和 SCM_RIGHTS 实测 |
| `test/smb-vfs/unit/asynchronous/` | 真实 SDK 事件库下的异步完成和取消生命期 |
| `test/interface/integration/` | CSR 状态与生命周期 |
| `tools/contract-check/` | schema、OpenAPI、固定样例和补充语义检查 |
| `evidence/` | 入库的验证说明与检查代码；执行生成的日志、JSON 和截图仅在本机保留，不保存凭据 |
| `../test/tools/contract-check/gitignore.rs` | 临时 Git 仓库中的目录忽略策略回归，保护源码和固定样例，排除运行生成物 |

执行命令、环境和当前限制集中在 [实现报告](reports/implementation.md)。核心的故障注入只能由可信进程调用，不提供远程故障开关。临时目录模拟盘不作为三个物理设备或断电持久性证明。

`evidence/` 中 README 的执行结论及测试程序保留在 Git；原始日志、截图、请求记录、构建收据与结果 JSON 均由测试生成，可通过独立归档共享。固定的测试输入与所属测试一起放在根 `test/` 的组件子目录，不能因文件较小就把生成结果当作固定样例。文档检查单独报告未生成的验证附件，缺少源码、检查程序或公开契约仍报错。

第 22 章是完整产品验收清单。当前测试没有逐一覆盖所有用例；真实 Btrfs 故障、SMB 跨平台应用完整矩阵和平台设备测试须分别补齐。已执行的 SDK、官方客户端及浏览器用例与这些待验收项分别记录。

2026-10-19 全项目维护性优化的阅读范围、具体问题、源文件摘要和集成验证见[检查记录](validation/evidence/maintainability.md)及[逻辑与模块梳理](reports/code-review-20261019.md)。浏览器检查使用当前候选的实际 WASM，额外改变共享几何 token 验证多页同步，不用逐页重复声明尺寸；登录保持独立。

2026-10-19 最新分支与批量文件操作的核心恢复、HTTP、Unix 描述符、Web 和本地服务证据见[分支实现记录](reports/branch-system-implementation.md)及[验收材料](validation/evidence/branch-operations.md)。文档图示检查与这些业务验证分别记录。

历史 JavaScript 探针的完整分类、显式夹具配置及静态 CI 门禁见[证据索引](validation/evidence.md)。当前生产浏览器验收由 [browser-smoke](tools/browser-smoke.md) 单独维护；历史脚本语法通过不替代实际浏览器运行。
