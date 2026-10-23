<a id="daily-management-cli-replacement-gates"></a>

# 日常管理 CLI 替代门禁

Web 管理员页面使用 `src/web/src/admin/request.rs` 中的真实 HTTP 请求构建器。存储变更需要有效管理员会话及 `management.storage.writable()`。账户管理则使用 `/api/admin/accounts` 和作为系统权威的 AuthStore，独立于数据存储。服务端仍负责授权和存储准入。

<a id="command--endpoint--web-surface"></a>

## 命令 → 端点 → Web 界面

| 原命令 | HTTP 端点 | Web 替代入口 |
|---|---|---|
| account | GET/POST /api/admin/accounts | 账户：选择已有账户、创建/更新密码、管理员、禁用、仓库准入授权 |
| create-repo | POST /api/repositories | 仓库表单 |
| repositories | GET /api/repositories | 查看仓库 |
| disks | GET /api/admin/disks | 查看磁盘 / 现有文件管理器磁盘导航 |
| topology | GET /api/backup-topologies/{id} | 查看拓扑 |
| policy | GET/PUT /api/policies/{id} | 策略表单，显式预期代数；查看当前值 |
| rule | GET/PUT /api/primary-placement-rules/{id} | 放置规则表单，显式预期代数；查看当前值 |
| smb-binding | GET/PUT /api/repositories/{id}/smb-binding | 绑定表单，包含分支、启用状态和预期历史代数 |
| list | GET /api/branches/{id}/tree | 查看分支树 / 现有文件管理器浏览 |
| history | GET /api/files/{id}/history | 查看文件历史 / 现有文件详情 |
| graph | GET /api/history-trees/{id}/graph | 查看历史图 / 现有历史树 |
| branch | POST /api/repositories/{id}/branches | 历史分支表单，包含源提交和名称 |
| link | POST /api/history-trees/{id}/merge-links | 合并链接表单，包含精确的源/目标分支和提交 |
| namespace | file-manager namespace 和 namespace-batch 路由 | 现有文件夹创建/重命名/移动/复制/删除控件；不新增内容传输 |
| purge | POST /api/history-trees/{id}/purge-plans | 清理预览，显式版本 ID |
| split | POST /api/history-trees/{id}/split-plans | 拆分预览，显式切分提交 |
| historical-save | POST /api/history-trees/{id}/historical-save-plans | 历史保存预览；不上传内容；可选 slot/copies |
| plan-commit | GET /api/plans/{id}; POST /api/plans/{id}/commit | 完整不可变预览 + 摘要 + 显式后果确认；仅限就绪计划 |
| plan-cancel | POST /api/plans/{id}/cancel | 独立的显式取消操作 |
| operation | GET /api/operations/{id} | 查看操作；结果未知的变更保留原操作 ID |
| check | GET /api/admin/check | 查看完整性检查 |
| repository ACL（补充） | PUT /api/repositories/{id}/grants | 独立的 read/write/manage ACL 表单；不与账户的仓库准入混淆 |

<a id="other-boundaries"></a>

## 其他边界

- 配置模板及未初始化草稿中的磁盘登记由 `zxs-rescue configuration` 提供。应用本地准备的 Config 使用受保护的 Web 初始化预设预览及摘要绑定确认。初始管理员创建属于同一初始化事务，不是通用密码重置命令。只允许完成已证明发生在权威建立前的中断；已有权威标记但凭据缺失时必须闭锁拒绝。
- serve 改为直接服务入口。
- restore-system/recovery-preview/recovery-commit/offline-check/offline-export 保留为明确的离线救援操作。
- 本地历史包导出、检查和恢复由 `zxs-rescue package` 实现，保留校验及无覆盖保护。它们不是日常 Web 操作，也不会重新引入 Web 上传/下载/预览。
- historical-save 可选的已有工作区不是内容传输界面。页面接受可选的已有工作区 ID，绝不暂存内容。

<a id="verification-gates-must-record-actual-outcomes-before-retirement"></a>

## 验证门禁（停用前必须记录实际结果）

`test/server/integration/http_engine/daily_management.rs` 导入 Web 的同一组请求构建器，并通过真实 Router + 持久化 Engine 发送这些请求：

1. 创建账户、仓库准入 + 仓库 ACL、拒绝未授权账户变更、禁用账户后撤销会话。
2. 创建仓库、更新策略/规则并拒绝过时代数、SMB 绑定、创建分支、合并链接。
3. 历史保存预览/取消/提交；未确认不得提交。
4. 清理和拆分预览/取消/提交；拒绝错误摘要。
5. 正整数/代数校验、编码后的计划 ID、不可变摘要、拒绝已完成计划、预览请求绝不提交。

独立 AuthStore / 存储不可用时的账户生命周期由运行时所属模块测试覆盖，而非由此基础 Engine 夹具覆盖。原生与 WASM 编译、集成界面连接属于独立门禁。这些 HTTP 测试不宣称覆盖浏览器点击自动化或视觉验收。移除 namespace 的 CLI 路径前，现有命名空间操作集成测试也必须通过。

<a id="non-default-namespace-equivalence"></a>

## 非默认命名空间的等价能力

`src/web/src/files/view.rs` 中的文件夹分支选择器会针对选定的可写目录范围调用 directory-select。现有批量审阅 UI 暴露显式源/目标分支选择；`files/batch/request.rs` 捕获每一侧的 `directory_branch_id` 和 `expected_tree_id`，以及稳定条目 ID。这是受控的分支范围 CAS 事务，不是任意 JSON 控制台。

真实 HTTP 测试 `branch_namespace_batch_preserves_unselected_views_and_is_idempotent` 验证跨多个非默认分支的移动/复制/重命名/删除，确认未涉及的分支及 SMB 默认视图不变，并检查重试幂等和分支范围审计事件。这是对通用 namespace CLI 的相关业务能力替代，而非机械复刻其语法。
