# 15 API、操作结果与错误契约

[文档目录](README.md) · [上一章：身份、权限与运行隔离](14-security-and-permissions.md) · [下一章：App、网页与命令行界面](16-client-interfaces.md)

<a id="s15"></a>

业务接口、幂等、分类型结果、错误及能力协商。

## 本章目录

- [15.1 设计原则](#s15-01)
- [15.2 拟定接口](#s15-02)
- [15.3 提交请求示例](#s15-03)
- [15.4 错误码基线](#s15-04)
- [15.5 协议兼容与能力协商](#s15-05)
- [15.6 不同操作的结果类型](#s15-06)
- [15.7 请求摘要、幂等与结果保留](#s15-07)
- [15.8 上传代际与续传](#s15-08)
- [15.9 分页、事件和校验清单](#s15-09)
- [15.10 传输状态与契约交付](#s15-10)

<a id="s15-01"></a>

## 15.1 设计原则

App、Web 和远程 CLI 使用统一 HTTPS 业务契约；SMB 基础操作由桥接映射到同一核心规则，SMB 文件请求仅承担基础操作；自研 App 的文件读写和高级功能均调用业务 API。两种入口不串接。内容流、元数据和管理操作可以使用不同路由或连接，但不能各自实现一套提交或权限规则。

所有变更请求绑定账户、目标作用域和 operation_id。内容保存携带分支、文件版本及相应前置条件；配置变更携带对应代际。旧 `MergeLink` 只固定所选节点，不要求当前 HEAD 或文件内容；文件分支合并 `FileBranchMerge` 则固定每个参与head、结果和归属，并验证所选正文。业务文件名称通过结构化字段传输，内部对象路径不对外开放。

<a id="s15-02"></a>

## 15.2 文件管理接口与高级接口基线

| 接口 | 用途 |
| --- | --- |
| `GET /v1/file-manager/disks` | 可见磁盘目录入口及只读状态 |
| `GET /v1/file-manager/disks/{slot}/entries?path={path}` | 当前目录列表、后代文件数与稳定目录分支的冻结组成 |
| `POST /v1/file-manager/disks/{slot}/namespace` | 准备并提交当前目录的创建、重命名或删除 |
| `POST /v1/file-manager/disks/{slot}/namespace-batch` | 显式源分支→目标分支映射；移动、复制、删除、重命名整批检查并发布 |
| `GET /v1/repositories/{repo}/directory-branches?path={path}` | 只读查询冻结分支和条目；省略 path 返回所有作用域，不切换默认分支 |
| `GET/PUT /v1/repositories/{repo}/default-branch` | 查询或选择 Web/App/SMB 共享默认根目录分支 |
| `PATCH /v1/repositories/{repo}/directory-branches/{id}` | 修改分支标签，身份、子树和内容不变 |
| `POST /v1/file-manager/disks/{slot}/write-sessions` | 取得该盘目录当前文件分支的写会话；随后通过通用工作区接口上传 |
| `POST /v1/file-manager/disks/{slot}/directory-branches` | `{path,name,operation_id}`：登记命名目录分支，直接引用父子树；不提前创建文件分支、不复制正文、不创建占位文件、不自动选择 |
| `POST /v1/file-manager/disks/{slot}/directory-select` | 同结构请求；选择完整冻结目录子树，恢复成员、名称、层级及精确节点；不要求后代有同名文件分支 |
| `GET /v1/file-manager/files/{file}/history` | 文件容器备注、所选分支及历史图；versions仅正文，nodes包含正文和复用正文的重命名节点；可成对传入 repository_id/path 区分别名 |
| `POST /v1/file-manager/files/{file}/branches` | 从选定版本登记公共文件分支；可传 source_node_id 固定路径节点来源，不复制正文或新增节点 |
| `PATCH /v1/file-manager/files/{file}/nodes/{node}` | 设置路径节点独立备注，最多128 UTF-8字节；不改正文备注或新增节点 |
| `PATCH /v1/file-manager/files/{file}/branches/{branch}` | 修改公共文件分支标签，最多128 UTF-8字节；同文件内规范化名称唯一，不增加版本或目录子树 |
| `POST /v1/file-manager/files/{file}/select` | 选择此文件的分支；传入 repository_id/path 时只改变该逻辑别名，不改变其他路径 |
| `POST /v1/file-manager/files/{file}/metadata` | 设置或清空文件容器备注，最多 128 UTF-8 字节，禁止换行及控制字符 |
| `POST /v1/file-manager/versions/{version}/metadata` | 设置或清空版本备注，最多 128 UTF-8 字节，禁止换行及控制字符 |
| `GET /v1/file-manager/history` | 获授权文件实际操作记录，当前最多最近 200 项 |
| `GET /v1/system/status` | 管理员读取真实 CPU、内存、磁盘容量及 CPU/磁盘功耗；不可用值为 null，功耗另含原因代码 |
| `GET /v1/system/history?range={range}` | 管理员读取后台监测历史；1min/30min/1h/24h/1month（最近30天），最多361个真实聚合点，与文件操作历史分离 |
| `GET /v1/capabilities` | 协议、命名策略、上传范围、全量能力及入口限制 |
| `GET /v1/repositories` | 已授权仓库列表 |
| `GET /v1/repositories/{repo}/branches` | 分支及归档状态 |
| `POST /v1/repositories/{repo}/branches` | 兼容目录快照入口：从指定提交登记各文件引用，不复制来源正文 |
| `POST /v1/branches/{branch}/archive` | 显式归档 |
| `GET /v1/branches/{branch}/tree` | 固定提交下的目录列表 |
| `POST /v1/branches/{branch}/resolve` | 把路径解析为条目与确定版本 |
| `POST /v1/branches/{branch}/write-sessions` | 登记会话、文件写入占用和 fencing token，不独占整分支日常写入 |
| `POST /v1/workspaces` | 创建写工作区和基础 HEAD 绑定 |
| `PUT /v1/workspaces/{workspace}/files/{item}/content` | 上传完整文件内容 |
| `GET /v1/workspaces/{workspace}/files/{item}/upload-status` | 查询上传代际、已持久化范围、预期长度及封存状态 |
| `GET /v1/workspaces/{workspace}/files/{item}/content` | 仅授权所属会话读取覆盖后的工作文件及 Range；返回 no-store，不使用不可变版本缓存 |
| `PUT /v1/workspaces/{workspace}/files/{item}/ranges/{offset}` | 可选脏范围协议，只有能力已声明才可使用 |
| `POST /v1/workspaces/{workspace}/namespace-operations` | 准备创建、改名、移动、删除等操作 |
| `POST /v1/workspaces/{workspace}/commit` | 发起封存与提交 |
| `GET /v1/operations/{operation}` | 查询确定结果或处理中状态 |
| `POST /v1/workspaces/{workspace}/abort` | 仅在安全阶段中止 |
| `GET /v1/versions/{version}/content` | 受授权的固定版本内容与 Range；可传 repository_id/path 按具体逻辑别名生成下载名称 |
| `GET /v1/versions/{version}` | 返回获授权版本描述及可信校验清单摘要 |
| `GET /v1/versions/{version}/checksums` | 返回该版本完整二进制校验清单，按当前权限及内容状态核验 |
| `GET /v1/files/{file}/history` | 文件和替换事件历史 |
| `GET /v1/commits/{commit}` | 提交及目录树信息 |
| `POST /v1/compare` | 比较版本或提交，不产生写入 |
| `POST /v1/history-trees/{history}/historical-save-plans` | 从历史元数据和工作区实际写入创建新分支计划，未写文件仍引用父版本 |
| `POST /v1/history-trees/{history}/merge-links` | 直接记录 source_branch_id/source_commit_id 到 target_branch_id/target_commit_id 的合并连线，不准备文件内容或冲突决议 |
| `POST /v1/history-trees/{history}/purge-plans` | 预览历史内容清除、受保护引用及实际释放量 |
| `POST /v1/history-trees/{history}/split-plans` | 预览切点后代闭包、ID映射、独立性及空间预算 |
| `POST /v1/plans/{plan}/commit` | 以固定计划摘要和代际执行，长任务通过operation查询 |
| `POST /v1/plans/{plan}/cancel` | 仅在未进入不可撤销决定时取消 |
| `GET /v1/history-trees/{history}/graph` | 分别包含版本演进父边、合并标记线、PURGED 节点及拆分来源的历史图 |
| `PUT /v1/policies/{policy}` | 条件更新保护策略 |
| `GET /v1/events?after={cursor}` | 可续读事件流或分页事件 |
| `GET /v1/backup-topologies/{topology}` | 已发布层级、各盘位直接来源和物理绑定；按权限展示 |
| `POST /v1/backup-topology-plans` | 用户指定一级/二级/后续来源映射，预检路由和数据迁移 |
| `POST /v1/storage-slots/{slot}/replacement-plans` | 用户为同一逻辑角色指定替换物理盘，保留来源映射 |
| `PUT /v1/primary-placement-rules/{rule}` | 条件设置仓库/目录的一级盘规则，只影响新版本；旧版另迁移 |
| `GET /v1/versions/{version}/placement` | 该版本一级归属、冻结路由、每级健康和迁移残留 |
| `GET /v1/admin/disks` | 磁盘状态，仅授权管理员 |
| `POST /v1/admin/disks/{disk}/drain` | 排空前检查和迁移任务 |
| `POST /v1/admin/disks/{disk}/eject` | 检查后安全卸载 |

文件分支登记回执固定创建时的可见父 ID、`inherited:true` 与 `own_version_id:null`；目录分支回执固定目录分支身份和父子树引用，不表示预先创建文件分支。之后首写不会改写原回执。当前内容状态通过读取接口查询，不能以当前状态回填旧操作结果。

上述文件管理接口已实现，具体字段以服务端路由及机器契约为准。其后的高级接口表包含设计基线，不能推断每项均已开放。旧整树创建、历史保存与拆分接口在已包含文件分支的历史域中明确拒绝，避免把文件快照当作完整目录树。目录分支具有稳定的 directory_branch_id，其每次实际组成变化形成新的 head_tree_id；这两个身份不能与文件分支和文件版本混用。文件列表的下载入口通过 Content-Disposition 使用该视图中的逻辑名称；内部不可变 storage_name 不随下载或容器改名而改变。

普通 workspace commit 只针对明确当前分支，SMB 的目标由共享默认根目录分支及其冻结子目录组成决定；历史节点保存通过 historical-save-plans 登记新分支，仅实际 staged 的文件产生 FULL 版本，不能沿用普通提交直接覆盖主分支。普通内容提交最多一个父提交，不能由用户任填父列表；这不限制统一文件节点图的多父 `parent_node_ids`。旧 `MergeLink` 不改写提交、正文或HEAD，新文件分支合并按独立请求推进参与引用并保存来源，原正文证明不变。树拆分使用独立结构迁移接口，不能把 rebase/强制重置作为隐藏入口。

<a id="s15-03"></a>

## 15.3 提交请求示例

```json
{
  "operation_id": "op-example-0007",
  "workspace_id": "ws-example-0007",
  "repository_id": "repo-example",
  "branch_id": "br-main",
  "expected_head": "commit-example-0006",
  "fencing_token": "17",
  "message": "更新设计文档",
  "requested_protection": "strict",
  "file_placement": {"item-report": {"primary_slot_id": "P01", "target_copies": 3}},
  "expected_topology_generation": "1",
  "sealed_manifest_hash": "<已封存工作清单的哈希>",
  "request_digest": "<请求规范表示的哈希>"
}
```

请求先通过验证并登记操作，长任务可返回 `202` 和操作 ID，明确表示尚未完成。内容提交的最终成功结果包含 `commit_id`、分支新代际、逐版本r(v)、一级归属、required_slots及每级确认状态、拓扑/绑定代际、成员代际和业务序号。操作查询分别返回 `commit_state`、`protection_at_publish`、`current_health`；已成功提交后来掉盘时保持原提交结果，只更新当前健康状态。

收到超时后只能查询或用**同一个 operation_id 和相同请求**重试。相同 ID 携带不同请求必须返回 `IDEMPOTENCY_KEY_REUSED`，不能覆盖原操作。

<a id="s15-04"></a>

## 15.4 错误码基线

| 错误码 | 含义 | 客户端必须做什么 |
| --- | --- | --- |
| `NAME_INVALID_ENCODING` | 非法 Unicode 输入 | 显示错误，不替换乱码继续 |
| `NAME_INVALID_CHARACTER` | 禁止字符或不在允许集 | 展示原名称和原因 |
| `NAME_RESERVED` | 保留设备名或系统保留名 | 要求用户明确选择新名称 |
| `NAME_NOT_NORMALIZED` | 新名称不是规定 NFC 形式 | 展示候选，确认后另发规范请求 |
| `NAME_COLLISION` | 大小写/规范等价重名 | 不自动加后缀 |
| `NAME_TOO_LONG` / `PATH_TOO_LONG` | 名称或路径超限 | 展示超限组件和预算 |
| `DIRECTORY_BRANCH_CONFLICT` | 依据的目录子树或写会话中的选择已过期 | 整批不发布；保留暂存内容，刷新视图后由用户重新决定 |
| `BATCH_MAPPING_CONFLICT` / `BATCH_SCOPE_CONFLICT` | 分支映射、物理归属或作用域不一致 | 修正明确映射，不自动覆盖或跳过 |
| `DIRECTORY_HISTORY_SPLIT_UNSUPPORTED` / `FILE_NODE_HISTORY_SPLIT_UNSUPPORTED` | 当前拆分协议不能保留目录组成或文件路径节点闭包 | HTTP 400 受控拒绝，不执行会丢失引用的拆分 |
| `DIRECTORY_SCOPE_REQUIRED` | 旧盘根包含多个独立命名空间，无法组成单事务 | 进入一个具体文件夹后操作 |
| `FILE_BRANCH_CONFLICT` | 工作区打开后文件选择已改变，含两个引用分支共用来源的情况，HTTP 409 | 保留修改并重新选择目标、获取工作区，不按相同来源ID继续写入 |
| `HEAD_CONFLICT` | 分支基础头已变化 | 保留工作区，不覆盖或自动合并 |
| `BRANCH_WRITE_BUSY` | 分支正在拆分或维护 | 保留工作区，等待维护完成 |
| `FILE_WRITE_BUSY` / `FILE_CONFLICT` | 同文件写入占用或基础内容已变化 | 保留修改，不静默覆盖 |
| `BRANCH_SWITCH_BUSY` | 有脏工作区或不安全句柄 | 不强制切换 |
| `WORKSPACE_DEPENDENCY_BUSY` | 屏障依赖其他尚未封存的文件或目录事件 | 保留脏数据，先处理所指依赖或显式批量提交 |
| `LEASE_EXPIRED` / `FENCED` | 写会话已失效 | 停止提交，重新获取授权 |
| `REPLICA_TARGET_UNAVAILABLE` | 固定路由中某级缺少健康完整副本 | 定位slot/source/level，不偷降、不换无关目标 |
| `NO_SPACE` / `QUOTA_EXCEEDED` | 空间或配额不足 | 保留待处理状态和已知错误 |
| `CONTENT_UNAVAILABLE` | 请求版本无可读健康副本 | 不替换成其他版本 |
| `CHECKSUM_MISMATCH` | 内容校验失败 | 隔离并切换相同版本副本 |
| `COMMIT_IN_DOUBT` | 无法判定提交传播结果 | 查询原操作，禁止自动清理 |
| `METADATA_UNAVAILABLE` | 元数据保护或恢复条件不足 | 阻止危险写入 |
| `RECOVERY_REQUIRED` | 可信成员、日志或配置证据不足 | 只开放可验证读取与恢复，不自动重置池 |
| `POLICY_CHANGED` | 准备期间保护或权限代际改变 | 按新条件重新准备，不能修改已有候选 |
| `FORMAT_UNSUPPORTED` | 必需盘格式或协议特性未知 | 停止写入/自动重放，使用受支持读取器 |
| `SAVE_MODE_UNSUPPORTED` | 请求不支持的保存或分支类型 | 明确拒绝；实际内容只接受 FULL，分支创建先记录与来源引用 |
| `HISTORY_NODE_NOT_FOUND` | 所选连线端点不存在或不在请求历史域中 | 重新选择有效节点，不以分支当前 HEAD 偷换端点 |
| `CONTENT_PURGE_PENDING` | 清除屏障已建立，新的内容读取或固定暂时被阻止 | 等待计划完成或安全取消，不改用其他版本，不提前视为已清除 |
| `CONTENT_PURGED` | 请求版本payload已按规则清除 | 显示节点和清除事件；内容GET可映射HTTP 410 |
| `PURGE_BLOCKED_BY_REFERENCE` / `CONTENT_PINNED` | 活跃引用、工作区或租约阻止删除 | 展示保护来源，不强删仍需内容 |
| `TREE_MOVED` / `SPLIT_PLAN_STALE` | 拆分已迁移绑定或计划代际过期 | 重新授权共享或工作区，重新预览 |
| `CROSS_TREE_DEPENDENCY_FORBIDDEN` | 请求将两独立树建立活动依赖 | 改为独立全量导入，不静默连接历史 |
| `PRIMARY_PLACEMENT_REQUIRED` | 无用户指定/继承的一级盘规则 | 选择一级盘，不随机存入其他盘 |
| `BACKUP_ROUTE_INCOMPLETE` / `ROUTE_TARGET_UNAVAILABLE` | 指定链深不足或所需级别盘位不可用 | 补全映射、恢复/显式替换目标；不跳级或自由回退 |
| `PLACEMENT_CHANGED` / `TOPOLOGY_INVALID` | 路由、来源、绑定代际变化或映射不合法 | 重新预览，拒绝环、跨级、重复后继和同盘多级 |
| `UNSUPPORTED_FILE_SEMANTICS` | mmap、链接等未支持操作 | 显式报错，不空实现成功 |
| `IDEMPOTENCY_KEY_REUSED` / `REQUEST_DIGEST_MISMATCH` | 操作 ID 对应不同输入或摘要不匹配 | 查询原操作，不能修改原输入后继续使用同一 ID |
| `RESOURCE_LIMIT_EXCEEDED` | 工作区、上传、句柄或队列达到准入上限 | 停止新增工作，查询或处理已有工作，不删除脏数据腾空间 |
| `UPLOAD_GENERATION_CONFLICT` / `UPLOAD_RANGE_CONFLICT` | 上传代际变化或重叠范围内容不同 | 查询原上传状态，保留本地内容，显式创建新代际 |
| `UPLOAD_INCOMPLETE` | 长度或有效范围未满足封存条件 | 补齐缺失范围，不能补零冒充完整上传 |
| `CURSOR_EXPIRED` / `PAGE_SNAPSHOT_EXPIRED` | 事件窗口或分页快照失效 | 重新授权并取得一致快照，按返回的新游标继续 |
| `CONFIG_CONFLICT` / `CONFIG_INVARIANT_VIOLATION` | 配置与权威状态或协议常量冲突 | 进入配置核查，不自动改盘上状态或降低保护 |

业务 API 向 App 返回结构化错误，自研 App 展示错误码、请求 ID、原因和待处理状态。SMB 入口单独返回对应协议状态，详细原因记录在服务器活动中心，按权限供管理入口查询。

<a id="s15-05"></a>

## 15.5 协议兼容与能力协商

客户端首次连接读取 `protocol_version`、`disk_format_version`、`name_policy_version`、`unicode_data_version`、路径预算、支持的操作、最大上传范围和任务状态机版本；同时获取存储池份数上限、每个一级盘的route_max_copies及拓扑/物理绑定代际、默认校验段大小、可读段大小集合及服务端验证/客户端独立验证模式。

遇到不能正确执行的规则版本应拒绝相关写入并说明升级要求，不用旧规则静默近似。服务器不能因为客户端缺少某个按钮就省去权限检查。

当前能力声明 `full_only:true`、`full_branches:false`、`branch_creation:"record-until-write"`、`branch_content:"inherit-parent-until-first-write"`，`directory_branch_creation:"inherit-directory-tree"`、`namespace_batch:true`，区分内容必须 FULL 与登记分支只保存冻结引用。历史另存、合并关系标记、清除、独立树拆分和用户备份拓扑按各自能力及已开放入口报告。SMB 暴露用户所选默认根目录分支的基础操作，管理 API 能力不能被解释为 SMB 也支持。画线请求固定节点，不要求固定当前 HEAD、解决冲突或提供文件内容。客户端请求不支持的保存方式或操作时明确拒绝，不能近似执行。

当前另开放文件合并、节点标签与scope备份配置，具体字段见[机器契约](contracts/openapi.json)和[合并、标签、备份规则](design/file-merge-tags-backup.md)。`merge_behavior:"explicit-file-node-adoption"` 区分文件结果采用与旧 `merge-links` 纯关系登记；`directory_branch_merge:false`、`directory_merge_status:"design-pending"` 明确目录分支合并待用户确认，暂不实现且无执行入口。

| 已实现入口 | 写入基线与结果 |
| --- | --- |
| `POST /v1/file-manager/files/{file}/merge` | 每个公共文件分支 `expected_node_id`、结果 `chosen_node_id`、归属 `surviving_branch_id` 与 `confirmed:true`；只推进参与引用，被合并分支成为只读历史别名 |
| `POST /v1/file-manager/files/{file}/tags` | 每个显式 `source_node_id` 与所关联分支当前 `expected_node_id`，按 `add/remove` 增量创建继承标签的元数据子节点；最多16标签、每项128 UTF-8字节；历史源不推进分支head |
| `PATCH /v1/file-manager/files/{file}/nodes/{node}` | 路径、标签与合并节点的独立128字节备注；不修改正文备注或正文proof |
| `GET/PUT /v1/repositories/{repo}/backup-configuration` | GET返回自身与有效策略代数、scope身份和manage权限；PUT固定 `expected_scope_id`/`expected_generation`、含主盘总份数和每个备份slot的用户逻辑前缀，并回填可用历史版本 |

节点标签和合并复用已有正文，`versions`仍只包含真实正文版本。统一 `nodes` 增加 `tags`/`merge` 类型与 `tags` 快照；独立 `merges` 保存全部输入来源，图边略去会形成环的关系。只读别名可通过历史节点下载和派生新分支，不能直接选择为可写分支。条件冲突返回409，标签限制/无效prefix返回400；不能因客户端先筛选了分支就省略服务端校验。

备份盘的 `entries` 按用户为该slot配置的逻辑路径浏览，`backup:true`、只读且不提供目录分支操作。条目的 `path` 是备份路径，`repository_path` 是真实主命名空间路径；主目录改名不改变既定备份路径。该槽没有当前正文时显示其最新可用旧正文，预览和下载必须固定条目的 `version_id`；已无live alias时返回 `historical_only:true`、`repository_path:null`，历史查询省略路径上下文。规范化路径冲突返回 `BACKUP_PATH_CONFLICT`，授权过滤先于冲突检查。

备份配置查询以 `backup_routes` 列出该scope所有实际历史来源链，`backup_slot_ids` 为各链备份slot的并集，`max_copies` 取链长度最小值。选择n份时应对每条链前n项取备份并集并收集独立prefix，不能只按当前默认链填写；旧 `available_route_slot_ids` 仅保留默认来源链作兼容展示。

对移动端、桌面及浏览器分别报告可用能力，不能发一个笼统的 `all_platforms_supported=true` 代替真实能力矩阵。

<a id="s15-06"></a>

## 15.6 不同操作的结果类型

`operation_id` 标识用户请求，`commit_id` 标识目录状态提交，两者不得等同。所有操作查询至少返回操作类型、处理状态、确定结果或明确的结果未知状态；只有会产生目录提交的操作才返回相应 commit_id。

| 操作类型 | 结果必须包含 | 不得隐含执行的动作 |
| --- | --- | --- |
| 内容保存 | commit_id、目标分支、新版本、发布时保护情况和当前健康情况 | 覆盖旧版本或把本地暂存当成已保护 |
| 分支创建 | 文件/目录分支稳定 ID、冻结来源节点、`copied_files:"0"`、`placeholder_files:"0"`及待写入状态 | 复制来源正文、虚增版本、将冻结来源引用报告为新增内容已保护或改写原主分支 |
| 历史保存 | 新 branch_id、起点、实际写入的 FULL 版本及未写入来源引用清单 | 隐式复制工作区未写文件，或改写原主分支 |
| 旧关系标记 `MergeLink` | merge_link_id、固定的来源与目标节点、关系记录保存结果 | 新建内容版本、目录快照、额外副本或推进 HEAD |
| 文件分支合并 `FileBranchMerge` | merge_id、全部输入head、chosen/result节点、归属分支、复用标志及受影响目录分支 | 自动拼接内容、复制正文或修改未参与引用 |
| 历史内容清除 | 逻辑墓碑结果、物理清理进度、离线残留及实际释放量 | 将墓碑发布或任务受理视为所有介质已清理 |
| 树拆分 | 原树与新树 ID、迁移映射、各树活动状态、主分支及共享处理结果 | 只发布一侧，或保持活动跨树内容依赖 |
| 拓扑或盘位变更 | 已生效代际、受影响路由、迁移结果和残留状态 | 只改配置标签便显示全部数据合规 |

HTTP 202 或任务已受理只表示操作登记，不表示正常保存成功。对取消请求，结果必须区分“已安全取消”和“已进入不可撤销决定、继续完成”；不得将客户端停止等待视为服务端已回滚。

<a id="s15-07"></a>

## 15.7 请求摘要、幂等与结果保留

核心以 `(pool_id, principal_id, operation_id)` 登记操作；目标作用域属于请求摘要，改变目标不能另建同 ID 操作。客户端只能查询当前获授权的操作，幂等索引命中不绕过授权。登记作为独立的短控制事务，按第 10 章完成三成员保护、发布及操作账本更新后再返回 202；该事务完成即释放槽位，不能在整个上传期间占住未收敛槽位，也不表示稍后保存内容的业务候选已 ACCEPT。登记失败或响应未知时，以相同 ID 查询或重试；明确未登记才可报告未受理。

JSON 变更请求摘要为 `BLAKE3("zxs:request:v2" + NUL + JCS(envelope))`。envelope 固定包含 `method`、`route`、`path_parameters`、`query` 和 `body`；method 大写，route 使用契约中的路径模板，路径参数单独传入，query 仅包含 schema 定义的有业务意义参数。body 排除唯一的顶层 `request_digest`；operation_id、目标、前置条件、fencing_token、计划摘要、工作清单摘要及全部已接纳扩展都参与计算。身份由已认证通道绑定；认证令牌、请求跟踪 ID、网络超时和重试次数不进入摘要。拒绝未知字段和重复字段后再计算，不自行填充会随环境变化的默认值。

内容提交必须绑定不可变 `sealed_manifest_hash`，覆盖每项最终长度、内容哈希、上传代际和命名空间操作顺序。相同工作区 ID 下字节变化必须产生新清单和新操作，不能沿用已登记请求指向可变工作区。流式内容的摘要绑定目标、上传代际、偏移、长度和内容哈希，不将二进制流强行表示成 JSON。

操作账本保存受理、阶段进展和固定终态。未决操作保留全部输入及恢复依据；已结束操作可把过程日志压缩进检查点，但必须在池生命周期内保留 ID、账户、请求摘要、终态及结果引用。内容清除不删除操作去重记录。配额耗尽时拒绝新登记，不通过遗忘旧 ID 恢复容量。工作区查询、取消和恢复通道须保留处理额度；销毁整个池是独立高权限流程。

<a id="s15-08"></a>

## 15.8 上传代际与续传

完整上传与可选范围上传都绑定 workspace、item、upload_generation、最终逻辑长度和预期内容哈希。第一版可以只公布完整上传能力；未实现范围协议时返回不支持，不近似接收。

| 情况 | 规定行为 |
| --- | --- |
| 完整 PUT 中断 | 保留已接收工作数据并标注未完成；完整上传模式的重试在同代际以相同内容重新传输，不把部分文件当作完成 |
| 范围上传重试 | 同代际、同偏移、同长度、同哈希返回原接收状态；其他重叠范围一律拒绝，变更内容须显式建立新代际 |
| 截短或改变最终长度 | 创建新上传代际并使旧写 token 失效；旧唯一脏副本在新内容可恢复或明确放弃前保留 |
| 查询已接收范围 | 区分内存接收与已持久化范围，只有后者可用于崩溃后的续传依据；两者都不表示已完成业务提交 |
| 新文件封存 | 范围必须完整覆盖 `[0, logical_size)`；空文件长度零且无范围，仍校验整文件哈希 |
| 基于旧版本的范围修改 | 清单固定 base_version_id 及基线哈希；未上传范围只从获授权且受引用保护的该版本补齐，再验证最终完整文件 |
| 封存与继续编辑 | 已封存清单不可变；后续写入进入新代际，旧操作继续引用旧清单 |

范围使用无符号 64 位偏移与长度，检查相加溢出、越界及请求上限。接收前申请逐盘和账户暂存预算；额度不足返回明确错误并保留已接收数据。最终 ACCEPT 前仍重新核对权限和业务前置条件。

<a id="s15-09"></a>

## 15.9 分页、事件和校验清单

目录第一页返回固定 commit_id、作用域、排序和不透明 next_cursor；后续页固定相同快照，即使 HEAD 前进也不混入新目录。每页重新核对当前授权。游标绑定账户、作用域、查询参数、快照及有效期，由服务端防篡改；越权游标不泄露快照信息。过期返回 PAGE_SNAPSHOT_EXPIRED，不静默换到最新页。

事件流按已发布序号及同事务内 ordinal 排序，允许至少一次投递，客户端按二者去重。返回的 next_cursor 表示已检查到的位置，不因过滤无权限事件而停滞；不得暴露被过滤事件内容。重新授权、树迁移或事件窗口过期时要求重新列举，并返回一致快照对应的续读游标，防止重列举与订阅之间漏事件。事件通知只帮助失效，读请求仍执行当前权限和墓碑检查。

独立校验流程为：取得获授权版本描述 → 获取完整 checksums.bin → 验证清单哈希、头部、段数与长度 → 按段获取内容并核对段哈希。清单接口与内容接口执行相同的授权、PURGE_PENDING/PURGED 屏障，不由公开静态目录提供。HTTP Range 仅定义单范围基线；多范围支持须单独声明，未支持时返回明确错误。HEAD、304、206 也先授权；416 应返回当前表示长度。示例与测试需覆盖空文件、尾段、越界及条件失败。

<a id="s15-10"></a>

## 15.10 传输状态与契约交付

| 结果类别 | HTTP 基线 | SMB 桥接要求 |
| --- | --- | --- |
| 任务受理 / 查询成功 | 202 / 200，查询响应体仍可能是处理中或失败 | 不映射成 FLUSH 已持久化成功 |
| 输入、编码或摘要错误 | 400；明确不支持的语义用 422 | 映射相应无效参数或不支持状态 |
| 未认证 / 无权限 | 401 / 403；隐藏对象存在性时统一使用 404 策略 | 使用协议认证或访问拒绝状态，不返回空目录 |
| 条件、代际或幂等冲突 | 409；HTTP If-Match 失败用 412 | 返回占用、冲突或前置条件对应状态 |
| 内容已清除 / 范围越界 | 410 / 416 | 返回相应不可访问或范围错误，并保留活动中心原因 |
| 准入限流 / 工作资源上限 | 429，可给 Retry-After | 新请求返回资源不足；已受理请求保持原操作关联 |
| 空间或配额不足 | 507 | 映射磁盘满或配额状态，不自动降低保护 |
| 服务或元数据不可用 | 503 | 返回服务不可用；已发 ACCEPT 的操作保留未知状态，不改写成确定中止 |

HTTP 状态描述本次调用，operation 的终态描述原业务请求，二者不可互换。SMB 的精确 NTSTATUS 值与回调必须在锁定版本的映射表和样机记录中冻结；以上分类不代替真实协议错误测试。

[契约附件](contracts/README.md)提供核心保存请求、操作结果、版本描述及配置约束的机器可读草案和固定样例。OpenAPI 明确列出当前已描述端点；本章其他拟定接口在补齐 schema、错误映射与验收前不能标记为已冻结。规范、schema 和样例出现差异时阻止发布并修订，不由某个客户端自行选择宽松解释。

---

[文档目录](README.md) · [上一章：身份、权限与运行隔离](14-security-and-permissions.md) · [下一章：App、网页与命令行界面](16-client-interfaces.md)
