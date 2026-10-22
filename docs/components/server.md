# storage-server

Axum HTTP 服务、持久会话认证及受保护的 Samba 本机桥接。业务变更经过 `storage-engine::Engine`，HTTP 层不直接修改数据盘内容。

项目尚未正式发布，业务接口统一使用无版本号的 `/api/` 前缀；客户端、Cookie 路径及路由校验共同使用该命名空间，不提供旧数字版本路由别名。

## 代码职责

| 模块 | 职责 |
| --- | --- |
| `lib.rs` | 服务配置、共享状态、路由组合、存储适配接口及通用 JSON 边界 |
| `http_auth` | 同源/CSRF 准入、登录和改密共用的限流、账号 HTTP 接口 |
| `auth` | 账号与会话持久化、备份发布、代际撤销和恢复阻断 |
| `transfer` / `uploads` | HTTP 内容流/范围/读租约；上传进度的有界读取与持久化 |
| `web_assets` | 静态资源准入和 ETag，保留 `/api` API 命名空间 |
| `engine_backend` / `file_manager` | 核心权限检查、业务动作和文件管理响应适配 |
| `telemetry` | 有界可重建的主机采样和历史，不参与存储提交真相 |
| `smb_rpc` | 本机 Samba 会话、句柄与工作区发布，不直接授予账号权限 |

密码、令牌和 CSRF 参数修改只在 [authentication](auth.md) 进行；Cookie 生命周期引用 `auth` 的同一会话有效期常量。存储格式和业务规则以核心为准。

## 启动

从项目根目录构建 `CARGO_TARGET_DIR=target/native cargo build --locked --offline -p storage-server -p storage-rescue`，直接启动服务，再按[受保护 Web 引导](../getting-started.md)完成首次初始化：

```sh
target/native/debug/zxs --root /srv/zxs/system \
  --listen 127.0.0.1:7400 --web-dir /srv/zxs/web/dist
```

HTTP 监听地址必须是回环地址。对外部署由同机反向代理终止 TLS；同时指定公开 HTTPS Origin，例如 `--external-origin https://storage.example.org`。服务只信任显式配置的 Origin，不根据任意转发头改变安全策略。`--web-dir` 可省略；省略时仅提供 API。

`--max-upload-bytes` 设置单次完整上传上限，默认 1073741824 字节；实际公开上限取它与存储池 `max_file_bytes` 的较小值。HTTP 上传和读取各自默认全局 8 个、每账户 2 个活动操作，支持显式配置，见[传输准入与期限](#http-transfer-limits)；密码验证最多并发 4 个。引擎工作区数量、字节数、过期时间等限制来自初始化 JSON 配置。发布仍经互斥锁串行执行，上传、读取及工作区提交的部分准备在锁外完成，详见[写入流程](../architecture/write-flow.md)；未提供吞吐量保证。

## 认证与授权

`POST /api/auth/login` 接受 `{"username":"admin","password":"..."}`，返回内存使用的 Bearer token、`csrf_token`、账号和到期时间，同时设置 HttpOnly、SameSite=Strict Cookie。会话默认 12 小时有效。密码采用 Argon2id；持久状态仅保存会话 token 的摘要。每个来源 IP 与用户名组合每分钟最多尝试登录 10 次。

新增会话同时受全局 4096 个和单账号 64 个有效会话的准入上限约束，达到任一上限返回 429；管理员也没有例外。已持久化且仍有效的会话不会仅因单账号数量超过 64 被撤销，但该账号必须先注销、等待到期或撤销会话，才能再次登录。该限制隔离单账号占用，不保证多账号同时耗尽全局额度时仍能登录。

登录和账号更新会回收已到期、账号不存在、账号禁用或代际不匹配的会话；账号更新还立即移除被此次变更撤销的会话。创建同名账号不会复活孤立会话。清理与新增会话或账号变更一起持久化；准入失败或持久化失败时，内存账号和会话恢复到变更前状态，且不绕过已有的发布不确定性恢复阻断。

Bearer 请求使用 `Authorization: Bearer ...`。Cookie 写请求必须同时提供精确匹配的 `Origin` 和 `X-CSRF-Token`；若 Bearer 请求显式提供跨源 Origin，也会拒绝。内容 API 可使用同源 Cookie，不需要把 token 放进 URL；这是服务端认证协议能力，当前 Web 产品不提供文件下载或预览入口。没有开放跨源 CORS。

`POST /api/auth/password` 允许当前账号提交 `current_password`、`new_password` 自助改密；新密码沿用至少 12 字节的准入要求。不接受用户名、角色或仓库范围变更。旧密码不正确返回 `403 CURRENT_PASSWORD_INVALID` 并保留会话；成功后通过账号代际撤销该账号所有会话、清除浏览器 Cookie，要求重新登录。每个来源 IP 和账号分别限制每分钟 10 次改密尝试，与登录共享密码计算并发上限。改密先耐久发布到独立系统盘认证主库并撤销旧会话；账号及授权的灾备副本异步同步到数据池，不以数据盘可用作为登录或改密的前提。

账号可限制允许进入的仓库，仓库还独立验证 read/write/manage 授权。直接访问版本、校验表、历史图、工作区和操作结果同样验证对象权限。拆分仓库允许自身 ID 的账号准入，也可沿用来源仓库的准入范围，最后始终验证新仓库独立的 ACL。修改或禁用账号会撤销该账号所有已有会话；不能禁用最后一个有效管理员。Samba 的每次业务调用也重新检查账号状态。

正常 daemon 的认证权威是系统盘上的 `management-auth.json`，不是引擎中的认证快照。HTTP、排队中的管理写入与存储准入都按当前主库重新核验会话、账号状态和权限，不能凭此前取得的身份绕过已发布的禁用、降权、范围撤销或会话注销。登录、账号管理和主机诊断不等待引擎恢复或数据盘复制；存储业务则在引擎不可用时明确拒绝。已获得的读取计划不会回收正在传输的字节，后续请求必须重新准入。

引擎侧受保护认证快照仅是账号与授权的灾备副本，不复制会话。在仍用于可信嵌入式测试的引擎认证模式中，准入投影随可变引擎操作失效并重新解析快照；它不是 daemon 管理主库的替代来源。正常托管模式直接使用当前主库的准入能力，不依靠旧引擎投影恢复权限。

`capabilities` 与 `readyz` 是不依赖账号的公开探针。仅供可信嵌入式调用的 `EngineBackend` 在未挂接受保护认证库时使用调用者明确传入的身份；这不是网络服务的默认认证模式，也不会把独立、未挂接的账号库当作引擎权限来源。

后台主机采样使用服务内部持有的只读观测能力，在引擎锁内捕获 `DiskObservationPlan`、锁外查询磁盘，不伪造管理员账号，也不发布业务状态。此能力不能从 HTTP/SMB 请求构造；公开 `MonitorDisks` 仍要求当前管理员身份，系统状态与历史接口在返回前再次检查当前账号权限。

系统盘的 `management-auth.json` 保存当前账号、授权与会话状态，数据池只接收剥离会话的灾备快照。`management-authority.json` 是权威交接围栏：交接完成后，即使主库丢失或损坏，也不能自动用旧 `auth.json` 或数据池快照恢复访问权。此时静态 Web 和公开诊断仍可访问，认证保持关闭，必须走受保护且明确的恢复流程。恢复不自动创建新管理员，也不复活旧会话。系统目录和认证文件只允许服务账号访问，认证文件有排他锁；拥有这些本机目录访问权的系统账号属于管理员信任边界。

## 已实现的路由

除健康探针、capabilities 和登录外，以下路由都需要认证；管理操作还检查管理员或仓库 manage 权限。

| 路由 | 方法与作用 |
| --- | --- |
| `/healthz`、`/readyz`、`/api/capabilities` | GET，存活、就绪、实际能力 |
| `/api/auth/login`、`/api/auth/me`、`/api/auth/logout` | POST 登录、GET 当前会话、POST 撤销 |
| `/api/auth/password` | POST 校验当前密码并更新本账号密码，撤销原有会话 |
| `/api/file-manager/disks`、`/disks/{slot}/entries` | GET 独立磁盘目录、文件列表与冻结目录分支组成 |
| `/api/file-manager/disks/{slot}/write-sessions`、`/namespace` | POST 当前文件上传会话、目录创建/重命名/删除 |
| `/api/file-manager/disks/{slot}/directory-branches`、`/directory-select` | POST 登记或选择目录分支的完整冻结子树 |
| `/api/file-manager/disks/{slot}/namespace-batch` | POST 显式源/目标目录分支映射，原子移动、复制、删除、重命名 |
| `/api/repositories/{repo}/directory-branches`、`/directory-branches/{id}` | GET 全部或指定路径作用域与冻结条目；PATCH 分支标签 |
| `/api/repositories/{repo}/default-branch` | GET/PUT Web/SMB 共用默认根目录分支 |
| `/api/file-manager/files/{file}/nodes/{node}` | PATCH 文件路径节点独立备注，最多128 UTF-8字节 |
| `/api/file-manager/files/{file}/branches/{branch}` | PATCH 公共文件分支标签，元数据修改，不生成版本或目录子树 |
| `/api/file-manager/files/{file}/history`、`/branches`、`/select`、`/metadata` | GET 文件版本/分支/备注；POST 创建、选择文件分支或更新文件备注 |
| `/api/file-manager/versions/{version}/metadata`、`/api/file-manager/history` | POST 版本备注；GET 最近实际文件操作 |
| `/api/system/status` | GET 管理员 CPU、内存与磁盘真实容量采样 |
| `/api/admin/accounts` | GET 账号列表、POST 新建或修改账号 |
| `/api/repositories`、`/api/repositories/{repo}` | GET 列表/详情，POST 建仓 |
| `/api/repositories/{repo}/grants`、`/api/repositories/{repo}/smb-binding` | PUT ACL；GET/PUT 显式 SMB 分支绑定 |
| `/api/repositories/{repo}/branches` | GET 分支、POST 登记独立待写入分支 |
| `/api/branches/{branch}`、`/tree`、`/resolve` | GET 元信息/目录树、POST 路径解析 |
| `/api/branches/{branch}/write-sessions`、`/archive` | POST 工作区写会话或归档 |
| `/api/workspaces`、`/api/workspaces/{workspace}` | POST 创建、GET 状态；可绑定历史提交 |
| `/api/workspaces/{workspace}/files/{item}/content` | PUT 完整上传、GET/HEAD 私有工作内容 |
| `/api/workspaces/{workspace}/files/{item}/upload-status` | GET 持久范围、代际、封存状态 |
| `/api/workspaces/{workspace}/namespace-operations` | POST mkdir/rename/delete/delete_tree |
| `/api/workspaces/{workspace}/seal`、`/commit`、`/abort` | POST 封存、原子发布、显式放弃 |
| `/api/versions/{version}`、`/placement`、`/checksums`、`/content` | GET 元信息/路由/校验表/内容，内容支持 HEAD 与单段 Range |
| `/api/files/{file}/history`、`/api/commits/{commit}` | GET 文件历史或提交 |
| `/api/history-trees/{history}/graph`、`/merge-links` | GET 图、POST 仅关联的合并链接 |
| `/api/history-trees/{history}/purge-plans`、`/split-plans`、`/historical-save-plans` | POST 预览，尚不执行变更 |
| `/api/plans/{plan}`、`/commit`、`/cancel` | GET 已保存预览、POST 按摘要执行或取消 |
| `/api/policies/{policy}`、`/api/primary-placement-rules/{rule}` | GET/PUT 目录保护策略、主盘规则，更新要求 generation |
| `/api/admin/disks`、`/api/backup-topologies/{topology}`、`/api/admin/check` | GET 盘状态、拓扑、完整性检查 |
| `/api/operations/{operation}`、`/api/events`、`/api/compare` | GET 结果/分页事件、POST 两次提交比较 |

表中同一行的缩写路径共享资源前缀，例如 `/api/branches/{branch}/tree` 和 `/api/branches/{branch}/resolve`。请求字段以代码及 [HTTP 契约](../15-api-and-errors.md) 为准；64 位整数使用规范十进制字符串。重复 JSON 键、浮点数、未声明的变更字段会被拒绝。失败响应包含 `error.code/message/retryable/request_id`，不会把未实现操作返回为成功。

## 上传与发布

上传 URL 使用 `?path=业务相对路径`，必需声明 `X-ZXS-Logical-Size`、`X-ZXS-Content-Hash`（完整 BLAKE3）；`X-ZXS-Upload-Generation` 为正的十进制代际，省略时为 1。服务流式写入私有暂存目录并周期 fsync。中断和失败内容保留：同代际重新发送完整文件时必须匹配已持久前缀、长度和哈希；内容变更须显式提高代际。旧代际不会被新上传静默删除。当前不支持按偏移 PATCH/Range 上传。

HTTP 断开不会取消已运行的阻塞发布或进度持久化。每个任务共享同一上传并发名额和文件写入占用，直到最终状态持久化完成；这期间同文件重试返回 `FILE_WRITE_BUSY`。Range 错误统一返回 `416` 和 `Content-Range: bytes */大小`，HEAD 响应不含正文；读租约随下载正文的消费或丢弃释放。

先调用 `seal` 冻结工作内容并取得提交模板，填入固定的 operation_id/message，再按 `storage_contract::request_digest` 计算完整请求摘要。原样保存请求后调用 commit；结果不确定时查询 operation 或原样重试。普通内容保存返回契约中的 SaveOperation/SaveResult，成功结果保留发布时保护证据，当前健康度单独计算。历史保存计划在单次核心事务中登记独立分支，只为实际写入的文件生成 FULL 版本，其余文件保留冻结父引用，原分支 HEAD 不变。

## 分支登记与首次写入

创建文件分支只持久化分支记录和冻结父引用，不复制正文、不创建占位文件、不增加文件版本。`copied_files`、`placeholder_files` 均为 `"0"`。目录分支直接引用父目录的完整子树，不提前登记后代文件分支；首次写入继承文件时才创建公共文件分支。创建不会自动选择新分支。

命名目录分支保持稳定身份；内容保存、子项分支切换及命名空间变化追加不可变目录子树。`namespace-batch` 显式列出每个作用分支及依据子树，整批检查并发布；移动保留文件身份并移交跟踪，复制产生独立文件，删除只移除选中分支条目，未选分支保留自己的冻结路径和节点。详细规则见[分支系统逻辑](../design/branch-system-logic.md)。

选中尚未写入的分支时，文件列表、预览和下载显示父版本。响应中 `version_id` 和大小指向当前可见的父文件，同时用 `materialized:false`、`inherited:true`、`own_version_id:null`、`source_version_id` 明确表示该分支尚无自己的版本；`content_state:"PENDING"` 表示此归属状态，`visible_content_state` 是父版本实际的可用状态。首次成功写入后，`version_id` 与 `own_version_id` 指向新 FULL 版本，`materialized:true`、`inherited:false`。目录里未写入的文件继续显示各自父版本，已写入的文件显示自己的版本。

未发布的工作区、上传失败和写打开后直接关闭不会产生内容版本。`versions` 只列真实正文版本；统一历史图还列复用正文的路径节点，分支登记单独保留审计事件。底层 `tree` 继续使用严格的 `Entry` 合同，以真实可见版本 ID 和大小返回继承视图，顶层 `pending_file_versions` 标明哪些文件仍继承来源。登记时的历史快照在首次写入后仍显示其父内容，并保持继承标记，不会追填一个属于该分支的旧版本。

Samba 的只读打开取得父版本的只读描述符；写打开从当前可见父内容建立私有工作副本。实际写入或显式截断并成功发布后才生成自己的版本，包括有效的零字节内容。仅打开后关闭和放弃工作区不会将私有副本变成分支版本，父版本始终不可变。

## 静态资源与 Samba

静态目录支持 `.gz` 预压缩、WASM MIME、弱 ETag/304、CSP 和 nosniff；发布新静态文件后应重启服务以重建资源 ETag 表。未知 API 不回退到网页。不要在静态目录保存认证或工作文件。

`/api` 和 `/api/` 下路径只属于 API：即便静态目录意外出现同名文件，也不返回静态正文或静态 304。未知 API 与缺失静态文件均返回 JSON `NOT_FOUND` 错误封装。

可选桥接参数为 `--smb-socket /srv/zxs/run/smb.sock --smb-peer-uid UID --smb-user SambaUser=CoreAccount`。只接受 Unix peer UID 白名单和显式账号映射，拒绝管理员账号映射；socket 父目录 0700、socket 0600。FD 传递、工作区提交和失败保留由桥接实现，Samba 模块构建与验收范围见 [集成文档](smb.md)。

尚未提供 HTTP 线上拓扑迁移、换盘/排空/弹出、偏移分片上传、多段 Range 或任意服务端路径导出。`POST /api/admin/disks` 明确返回 409，不能在线修改配置草稿代替迁移。历史独立恢复包通过本机 CLI 操作。上传与 delete/rename 路径发生祖先重叠时会保留数据并拒绝提交，尚未实现任意交错的工作区事件重放。

## 验证

```sh
CARGO_TARGET_DIR=target/native CARGO_INCREMENTAL=0 \
  cargo test --locked --offline -p storage-server -p storage-client
```

Unix socket 测试需要运行环境允许本机 socket。[集成回归](../../test/server/integration/http_engine.rs) 覆盖 HTTP 准入、封存、三盘发布、幂等、Range、ACL、历史计划和策略 CAS，也覆盖文件/目录分支、真实版本祖先、备注与重命名下载、上传池限额，以及改密的 CSRF、同源、字段和 JSON 长度边界。永久丢失后的降级发布与发布后损坏均保留原保护证据；可信嵌入式认证快照恢复测试仍保留，但不代表 daemon 在权威交接后允许自动恢复丢失的认证主库。管理运行时回归另外验证主库丢失时围栏拒绝旧快照、数据引擎阻塞时仍可登录，以及排队请求在撤权后被拒绝。两种保存结果 wire 均通过原 SaveOperation schema 验证，详见 [API 验收证据](../validation/evidence/api.md)。

模块回归还验证上传断开后的占用生命周期、Range/HEAD 及读租约、静态/API 命名空间、规范化账号限流、损坏上传状态和持久化失败清理。平台与共享认证回归使用 `cargo test --locked --offline -p authentication -p platform-io -p storage-server` 一并运行。

账号撤权回归通过双屏障固定“身份已取得、引擎尚未准入”的窗口，完成禁用、管理员降权或范围撤销后才放行普通元数据、工作区、上传、读取计划及 SMB 共用入口，检查拒绝前后状态和三成员日志不变；另验证当前范围过滤、公开探针和未挂接认证库的嵌入模式边界。

用户目录分支以稳定文件身份和精确节点组成冻结子树；登记目录分支直接引用父组成，文件首次实际写入时才登记公共文件分支。批量操作明确列出每个作用分支和基线子树，任一条件冲突整笔拒绝。已包含文件分支的历史域明确拒绝未适配的旧整树创建、历史保存和拆分。新版本正文使用真实文件容器，下载名称采用所选逻辑别名，布局及旧数据兼容见[存储格式](../08-full-storage-and-disk-format.md)。

文件历史保留真实正文的 `versions`，统一图使用 `nodes`、`node_id` 与 `parent_node_ids`。文件或文件夹重命名为受影响文件创建路径节点并复用原正文；后续真实写入沿路径节点继续连线。路径节点备注与版本备注分开保存，分支来源可明确指向路径节点，下载仍使用真实 `version_id`。

`POST /api/file-manager/files/{file}/merge` 提交已审阅的参与分支 head、任意同文件结果节点、归属分支及 `confirmed:true`。参与 head 改变返回409；被合并分支保留冻结历史，转为只读别名。结果恰为参与 head 时复用节点，否则创建复用正文的 `fmeta_` 合并节点；独立来源记录保存全部输入，图边过滤成环关系。只推进参与引用，父目录冻结的旧子树在自己的组成上派生，不从子目录新 head 带入未参与文件。

`POST /api/file-manager/files/{file}/tags` 对显式节点与分支 head 基线应用标签增量，最多16项，每项128 UTF-8字节。变化生成不可变标签子节点，保留源标签；历史源不推进分支 head，当前精确引用才推进。标签不产生正文或完成首次正文写入。路径、标签、合并节点备注统一使用 `PATCH /files/{file}/nodes/{node}`，与正文备注独立。

`GET/PUT /api/repositories/{repo}/backup-configuration` 查询/配置稳定文件或目录 scope。写请求需要manage，绑定 `expected_scope_id` 与自身 `expected_generation`；继承策略的有效代数另以 `effective_generation` 返回。份数包含主盘；每个备份slot使用用户独立指定的逻辑路径前缀，相对所选scope根。配置回填可用历史正文且保持版本及proof ID。目录分支合并待用户确认，暂不实现，能力明确为false且无写端点。完整语义见[文件合并、标签和备份](../design/file-merge-tags-backup.md)。

### 反向代理登录限流

默认忽略 `X-Forwarded-For`，按实际 TCP 对端限流。若使用同机 TLS 反向代理，可显式增加 `--trusted-proxy-ip 127.0.0.1`（IPv6 代理则为 `--trusted-proxy-ip ::1`；可重复）。必须同时设置 `--external-origin https://storage.example.org`。仅接受指定的精确回环代理地址，不接受网段或非回环代理；监听地址仍必须是回环地址。这是可选部署配置，升级不会自动启用信任。

代理必须**覆盖**客户端传入的 `X-Forwarded-For`，仅写入它实际看到的客户端 IP，例如 nginx：`proxy_set_header X-Forwarded-For $remote_addr;`。禁止使用 `$proxy_add_x_forwarded_for` 或转发客户端原始头。来自受信代理的缺失、重复、列表、带端口或其他非单一 IP 值会被拒绝；未受信 TCP 对端的该头被忽略。`Forwarded`、`X-Real-IP` 和其他转发头不参与身份、安全 Origin 或 Cookie 策略。不要将来自不可信本地进程的请求混入受信代理来源；本机进程可使用同一回环源地址，信任本机是此部署模型的前提。多层代理需要在最后一跳安全解析上游来源后覆盖此头，不能直接传递头链。

登录保留每来源 IP 每 60 秒 10 次的上限，并记录来源 IP + 规范化用户名组合；共享 NAT 的客户端仍共享来源限额。跨 IP 针对同一规范化用户名的尝试超过 10 次后，验证前逐步增加 250ms、500ms、1s、2s、4s 至最多 6s 的等待，而不是耗尽用户名额度后整分钟拒绝。账号压力在连续 60 秒无获准尝试后消退，持续攻击不能借固定窗口边界反复取得免费突发额度。每个规范化账号只允许一个正在等待或验证的请求，稳定持续猜测因而最多约 10 次/分钟；其他同账号并发请求立即返回繁忙 429。全局最多保留 64 个待登录账号，取消或完成时释放。等待不占用全局四个密码哈希槽位，单个受攻击账号不会因此独占其他账号的哈希容量。不同账号的哈希容量满时立即返回 429，不建立无限队列。

此策略限制分布式猜测，但无法保证恶意流量下的可用性：被针对的账号仍可能遇到繁忙或最长 6s 的验证前等待；公网部署还应在代理层实施总体请求/连接限制。无效、非 ASCII 或超过共享用户名策略长度上限的用户名，在进入待处理集合和计数表之前即返回 401，不保留原始输入作为键。计数与容量更新是原子的，被来源限额拒绝的请求不能消耗另一个账号的额度；计数过期后清理，内存键数最多 4096。上述配置不会修改 Origin/Cookie 安全策略，也不会更改现有账号、凭据或数据库。

<a id="http-spool-budget"></a>

## HTTP 暂存预算与恢复

`api-spool` 使用独立的 `SpoolBudget`，不再把单次上传长度或 8 个并发许可当作累计容量限制。默认每账户 100 GiB、整个 HTTP spool 500 GiB，并保留所在文件系统 1 GiB 空闲恢复余量；这些额度不包含核心工作区或 SMB 暂存，不能相加后仍宣称是统一全池预算。

服务启动先独占 `.budget.lock` 并盘点 active/retained/owner 记录和数据文件。已完成、失败、旧代际及未知孤儿文件继续计费，缺失或无法归属的记录不得获得免费容量；无法确认计费的 I/O 结果会阻止新准入，直到重启盘点。每份数据另计 64 KiB 保守元数据额度，超额的已有元数据按实际长度计入。这是文件长度和保守开销账本，不声称等于文件系统实际分配块数。

接收 body 前原子预留账户及全局额度。所有后台写入/同步/记录发布与 reservation 共享生命期，取消等待不提前归还仍在执行的容量。同代际重发只验证已有物理前缀并续写，不复制第二份正文、不覆盖未宣告为 durable 的尾部；新代际保留旧字节。请求结束按实际仍保留文件长度结算，而不是按成功与否释放。

目前没有自动删除未决材料或按 TTL 回收历史的功能；达到预算会拒绝新增上传。已结算 API 副本的安全回收/显式放弃入口仍需单独设计，不能手动删除活动记录来规避账本。操作系统的其他写入者可能消耗同一文件系统空间，余量检查并不构成内核级独占预留。

运维入口：`zxs --http-spool-bytes-per-account ... --http-spool-bytes-total ... --http-spool-recovery-reserve-bytes ...`。单文件的 `--max-upload-bytes` 与上述累计限制独立；预算参数必须为正、总额不低于账户额、账户额至少容纳元数据额度。

<a id="http-transfer-limits"></a>

## HTTP 并发公平性与传输期限

上传和读取使用两个独立的准入池，读取包含版本正文、工作区正文和校验清单。默认每池全局 8 个、同一认证账户最多 2 个活动操作；多会话和管理员身份也不能绕过账户额度。账户或全局满时立即返回 429，不建立等待队列。这是单账户隔离而不是轮转调度，不能保证多账户攻击下的可用性。

`--http-transfer-slots-total`（8）与 `--http-transfer-slots-per-account`（2）要求 `0 < 单账户 < 全局`；`--http-transfer-idle-seconds`（60）与 `--http-transfer-total-seconds`（3600）要求 `0 < idle <= total <= 604800`。四项映射至 `ServerConfig.http_transfer_limits`，仅影响 HTTP，不改变 SMB 预算或超时。

总期限从进入已认证的传输处理器开始，包含准入后的授权、打开、暂存、发布和流式读取。上传 idle 按接收非空 body 块重置；空块不能续期。下载 idle 按 HTTP 层取走非空正文块重置，不代表远端已确认接收；即使 consumer 停止轮询 body，独立计时器仍会终止旧流并释放空闲读租约。总期限不能被持续小流量重置。响应头发出前超时返回 408 `TRANSFER_TIMEOUT`；已经开始的二进制响应终止为流错误，不追加 JSON，旧 body 恢复轮询也不能继续读取。

期限取消的是 HTTP 等待，不是强制取消同步磁盘 I/O。已经启动的阻塞校验、回退读取、写入、同步或发布任务继续持有账户/全局许可及读租约、spool reservation、条目 claim，直至任务真正结束，因此卡死的底层 I/O 仍可能占用该账户的额度。不会通过超时提前释放正在使用的文件或宣称发布失败；客户端应查询上传状态后重试。空闲超时尽可能持久化已有前缀；总期限/断开可在持久化前到达，已写物理尾部及旧记录仍保留，后续重发必须验证原字节。不会删除失败上传、未决操作或未知文件，累计 spool 预算仍计费。

### 普通 HTTP 后端任务准入

普通 HTTP 元数据和文件管理后端调用使用独立的 8 槽准入，每个 router state 及其克隆共享。额度耗尽时立即返回 HTTP 429 `RESOURCE_LIMIT_EXCEEDED`，标记 `retryable:true`，不建立应用等待队列。许可归阻塞任务所有，覆盖 Tokio 队列等待及实际执行；HTTP 请求取消后仍保留，直到任务完成、报错或展开退出。这是保守的资源安全上限，不是吞吐保证，也不复用传输配置。

`/readyz` 和 `/api/capabilities` 同样受此限制，饱和时可返回 429；`/healthz` 不使用该池。上传和正文读取保持独立准入，避免嵌套获取同一许可。认证、会话任务和遥测不受本池控制；它不是进程级后端预算、跨 HTTP/SMB 的磁盘预算或 SMB 文件描述符路径限制。请求解析发生在此准入之前，因此该边界不限制所有连接或请求体，也不增加缓存。
