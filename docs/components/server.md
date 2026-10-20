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

从项目根目录构建 `CARGO_TARGET_DIR=target/native cargo build -p storage-admin`，先按照 [CLI 文档](admin.md) 初始化存储池，然后运行：

```sh
target/native/debug/zxs --root /srv/zxs/system serve \
  --listen 127.0.0.1:7400 --web-dir /srv/zxs/web/dist
```

HTTP 监听地址必须是回环地址。对外部署由同机反向代理终止 TLS；同时指定公开 HTTPS Origin，例如 `--external-origin https://storage.example.org`。服务只信任显式配置的 Origin，不根据任意转发头改变安全策略。`--web-dir` 可省略；省略时仅提供 API。

`--max-upload-bytes` 设置单次完整上传上限，默认 1073741824 字节；实际公开上限取它与存储池 `max_file_bytes` 的较小值。服务同时最多处理 8 个上传和 4 个密码验证。引擎工作区数量、字节数、过期时间等限制来自初始化 JSON 配置。当前引擎通过互斥锁执行同步事务，未提供吞吐量保证。

## 认证与授权

`POST /api/auth/login` 接受 `{"username":"admin","password":"..."}`，返回内存使用的 Bearer token、`csrf_token`、账号和到期时间，同时设置 HttpOnly、SameSite=Strict Cookie。会话默认 12 小时有效。密码采用 Argon2id；持久状态仅保存会话 token 的摘要。每个来源 IP 与用户名组合每分钟最多尝试登录 10 次。

Bearer 请求使用 `Authorization: Bearer ...`。Cookie 写请求必须同时提供精确匹配的 `Origin` 和 `X-CSRF-Token`；若 Bearer 请求显式提供跨源 Origin，也会拒绝。浏览器下载可直接使用同源 Cookie，不需要把 token 放进 URL。没有开放跨源 CORS。

`POST /api/auth/password` 允许当前账号提交 `current_password`、`new_password` 自助改密；新密码沿用至少 12 字节的准入要求。不接受用户名、角色或仓库范围变更。旧密码不正确返回 `403 CURRENT_PASSWORD_INVALID` 并保留会话；成功后通过账号代际撤销该账号所有会话、清除浏览器 Cookie，要求重新登录。每个来源 IP 和账号分别限制每分钟 10 次改密尝试，与登录共享密码计算并发上限。改密使用相同的三成员认证备份事务。

账号可限制允许进入的仓库，仓库还独立验证 read/write/manage 授权。直接访问版本、校验表、历史图、工作区和操作结果同样验证对象权限。拆分仓库允许自身 ID 的账号准入，也可沿用来源仓库的准入范围，最后始终验证新仓库独立的 ACL。修改或禁用账号会撤销该账号所有已有会话；不能禁用最后一个有效管理员。Samba 的每次业务调用也重新检查账号状态。

`auth.json`、会话撤销和账号状态通过正常三成员事务备份到存储池。正常启动会以恢复出的权威快照重建丢失或过期的系统盘认证文件，不会自动创建新管理员。认证备份结果不确定时停止接受认证，重启后恢复。系统目录和认证文件只允许服务账号访问，认证文件有排他锁。拥有这些本机目录访问权的系统账号属于管理员信任边界。

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
| `/api/repositories/{repo}/default-branch` | GET/PUT Web/App/SMB 共用默认根目录分支 |
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
  cargo test -p storage-server -p storage-client --offline
```

Unix socket 测试需要运行环境允许本机 socket。[集成回归](../../src/server/tests/http_engine.rs) 覆盖 HTTP 准入、封存、三盘发布、幂等、Range、ACL、历史计划和策略 CAS，也覆盖文件/目录分支、真实版本祖先、备注与重命名下载、上传池限额，以及改密的 CSRF、同源、字段和 JSON 长度边界。永久丢失后的降级发布与发布后损坏均保留原保护证据；本地认证文件丢失后从三成员权威状态恢复新密码。两种保存结果 wire 均通过原 SaveOperation schema 验证，详见 [API 验收证据](../validation/evidence/api.md)。

模块回归还验证上传断开后的占用生命周期、Range/HEAD 及读租约、静态/API 命名空间、规范化账号限流、损坏上传状态和持久化失败清理。平台与共享认证回归使用 `cargo test --locked --offline -p authentication -p platform-io -p storage-server` 一并运行。

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
