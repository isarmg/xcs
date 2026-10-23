# ZXS SMB 桥接

本目录包含 `smb-protocol` 本机协议与 `smb-vfs` Rust 模块。已重新使用官方 Samba 4.23.0 编译 SDK 并验证 VFS。2026-10-18 的文档另报告过加载 `smbd` 和官方客户端 SMB3.11 文件往返，但其原始日志不在当前 checkout，本轮未重新验证该运行时结果。当前可复核记录、历史声明与未测范围见[验证记录](../validation/evidence/smb.md)。这不是跨平台应用、生产认证配置或故障恢复矩阵全部通过的声明。

## 目录与职责

| 路径 | 职责 |
| --- | --- |
| `src/smb/src/protocol.rs` | 有上限的严格 JSON 帧、SCM_RIGHTS 文件描述符转交 |
| `src/smb/src/client.rs` | 同步本机 RPC 客户端；同一连接上的请求串行化 |
| `src/smb-vfs/src/module.rs` | Rust 编写的 Samba VFS 回调，不包含自写 C 桥接 |
| `src/smb-vfs/src/asynchronous.rs` | 两个 Rust 工作线程、有界队列及 eventfd/tevent 完成与取消处理 |
| `src/smb-vfs/build.rs` | 校验固定 SDK 版本，使用 bindgen 生成 C ABI 绑定 |
| `src/server/src/smb_rpc.rs` | 身份映射、句柄、私有工作文件、核心提交和读租约 |

客户端的同一连接在全部 clone 之间串行使用。请求发送、响应接收或帧解析失败后，立即关闭并移除共享连接；后续调用明确返回 `NotConnected`，不会把残留响应当作另一请求的结果。完整远端业务错误保留连接。VFS 从句柄表移除文件后立即建立 FD 所有权守卫，锁失败或保存失败也会关闭本地描述符；本机关闭策略与协议回归共用 `client::close_descriptor`，不依赖手写 Samba ABI 测试替身。

`smb-protocol` 是根工作区中的协议包，服务端直接依赖它；没有独立工作区或锁文件。VFS 位于单独的 `smb-vfs` 包，默认要求 `smb-sdk` 特性。缺 SDK 时构建模块会失败，不产生可以误认为完整插件的空实现。

## 已实现的桥接语义

- 只允许受信任的本机 Samba 进程 UID 连接，使用 Linux `SO_PEERCRED` 校验；JSON 中没有远程客户端可以自行声明的 ACL、管理员权限或任意核心命令。
- Samba 用户名通过服务端显式配置映射为 ZXS 身份；映射不允许 `admin=true`。服务端的授权包装器在每次操作时重新读取账户状态和权限。
- 分享范围固定到一个仓库的主分支。读打开取得具体版本的文件描述符，服务端保留核心读租约。后续提交不会改变旧句柄读到的版本。每次读取与句柄状态查询仍复核当前账户及 ACL，拒绝以既有 FD 绕过撤权。
- 写打开使用服务端私有目录中的完整独立工作文件，FD 通过 SCM_RIGHTS 转交；RPC 不返回磁盘内部路径。
- 选中的未写入分支继承父版本的内容和大小。只读打开取得父版本的只读描述符，写打开从父内容建立私有工作副本；仅打开后关闭不创建版本，实际写入或显式截断并成功发布后才形成自己的分支版本，父版本不变。目录分支中各个文件分别保留继承或自己的版本。
- `pwrite` 与 `truncate` 修改前调用 `PrepareWrite` 检查已预留的文件上限与当前恢复空间，修改工作文件后调用 `Dirty`；write-through 的 `Dirty` 同步触发保存。VFS 的 `FLUSH` 和最后 `CLOSE` 经完整文件上传及核心事务提交后才返回保存成功。
- 写锁按分支和规范化路径绑定。不同文件允许并发写入，最终由核心检查各自文件版本并串行发布；同文件跨会话写打开明确拒绝。目录改名、删除和替换目标只与路径覆盖范围内的活动写句柄冲突，读句柄仍固定原版本。
- 重试先查询原 `operation_id`，已发布结果直接返回；未决结果要求恢复，不重新生成操作或再次推进上传代际。
- `Flush` 失败保留活动句柄，可以先核对原操作再重试。`Close` 无论成功或失败都终结该句柄：保存或清理工作区失败时，将工作文件和 `*.work.json` 保留为脱离态，释放该句柄的 SMB 路径锁及句柄额度，返回原业务错误和原 `operation_id`。不因此关闭整条 RPC 连接，不影响其余句柄，不用自动 Abort 代替失败保存。
- I/O 工作线程锁中毒时，VFS 发 `Detach` 而不是尝试保存。`Detach` 只保留正文/恢复关联并释放该句柄，不提交、不放弃工作区、不删除暂存。客户端和服务端应一起升级以支持此命令。
- 断开连接不会把脏数据自动发布为历史，也不删除工作文件和恢复记录。丢失发布回执后再次改动的字节不会冒充先前发布的成功结果，须核对原操作并创建新工作区继续保存。
- 目录创建、删除、改名进入核心命名空间事务。`unlinkat` 的普通删除与 `AT_REMOVEDIR` 分别转为 `unlink` / `rmdir`，在工作区冻结树上复核目标种类；普通删除不得删除目录，目录删除不得删除文件，未知 flags 在发出 RPC 前失败。旧的不区分种类 `delete` RPC 不再接受，模块和服务端须一起升级。允许替换的改名在同一个工作区中执行删除目标及改名，避免发布中间状态；不支持的文件特性明确失败。
- 命名空间请求必须携带客户端预分配的 `operation_id`，服务端严格验证并沿用。工作区创建后、准备变更前，将原操作号、确切工作区提交请求和目标路径写入私有 `*.namespace.json`，文件及父目录同步成功后才允许提交。一旦进入提交调用，任何错误都不证明未发布，因此不自动 Abort、不删除恢复记录。只有确认核心成功，或准备阶段失败且记录同步及 Abort 均成功，才清理本次记录；记录同步失败或 Abort 失败保留已有证据并告警。不会自动用新操作号重做未知结果。
- 模块源码拦截链接、复制卸载、FSCTL 写入、权限与扩展属性等可能绕过核心的修改路径，禁止由下层 VFS 修改不可变版本文件。
- 异步读取、写入与同步在有界工作池中完成；工作线程只拥有 FD、缓冲区和 RPC 客户端，不持有宿主指针。取消请求后，不再访问已释放的事件状态。sendfile/recvfile 明确拒绝，避免绕过读取授权与保存边界。
- 文件名规范由核心负责；POSIX stat 区分实际拼写，文件名查找返回核心的规范名，以支持 SMB 不区分大小写打开与保留大小写改名。查找失败时保留调用者原指针，避免破坏宿主后续创建路径。

## 本机 RPC 部署

先建立服务进程拥有的 `0700` 目录，例如 `/var/lib/zxs/runtime/smb`；整个路径不能包含符号链接。服务端绑定其中的 socket，并设置 `0600` 权限。其下 `smb-spool` 和 `empty-directory` 也要求 `0700`。祖先只接受 root 或服务身份拥有、不可被其他用户替换的目录；标准 root-owned sticky `/tmp` 可作为安全父目录，但非 sticky 的组/全局可写目录和外部用户所有的祖先会被拒绝，既有目录不会自动 chmod。创建暂存文件及发布恢复记录使用持有的目录描述符；Flush 提交前检查目录/源文件身份，并把原可写 FD 的长度和摘要绑定到核心复制结果，校验失败不会接受工作区文件或提交。摘要和复制均使用定位读取，不改变 Samba 客户端共享的 FD 游标。

服务端启动配置需要：

1. socket 的绝对路径。
2. Samba 可信进程连接时的 UID；这个身份是进程代理的信任边界。
3. `SMB 用户名 → ZXS 用户名` 显式映射；ZXS 用户必须已建立并有相应仓库授权。

可信 Samba 进程必须在模拟远程用户前建立受保护连接。目标环境需要验证其实际回调时机与 UID；不能为了让连接成功而把 socket 或目录改成所有用户可读写。

socket 已存在时服务端拒绝覆盖，启动时不会删除未知对象。正常退出或任务取消时，会关闭所有活跃 RPC socket，让阻塞读取退出；只在路径仍对应自身绑定的 device/inode 时移除 socket。服务崩溃遗留的 socket 须由管理员确认后清理。

帧格式为 `u32` 大端 JSON 长度、严格 UTF-8 JSON；最大 8 MiB，分配 payload 前检查长度。可选单个 FD 通过四字节帧头的 SCM_RIGHTS 附带；接收端立即设置 CLOEXEC，拒绝额外或截断的 FD。客户端请求禁止附带 FD。64 位字段使用规范十进制字符串。

握手必须显式声明 `capabilities: ["durable-write-reservations"]`，服务端须确认同一能力后客户端才可请求句柄。缺少能力字段或没有这项能力的旧模块在创建会话、复制文件和转交 FD 前即被拒绝，不提供跳过 `PrepareWrite` 的兼容路径。能力名称描述写入前持久预留的语义，不改变协议名称或增加版本号。

仓库在拆分、恢复后默认禁止 SMB，新连接明确返回 `SMB_REBIND_REQUIRED`；管理员须通过受权绑定接口核对历史代际后启用，原共享不自动跳到新历史。

命令为 `hello`、`open`、`authorize_read`、`prepare_write`、`dirty`、`flush`、`close`、`detach`、`abort`、`stat`、`list` 和 `namespace`。一个连接只能 `hello` 一次，此后固定身份与仓库。错误带 `code`、`retryable`，保存及合法命名空间请求的错误还附原 `operation_id` 供核对。命名空间请求不接受缺少操作号的旧报文；客户端遇到传输错误时关闭所有克隆共用的连接，保留原 I/O 错误种类/errno，并以 `RPC_OUTCOME_UNKNOWN` 携带请求原操作号。即使核心已成功而成功 RPC 帧丢失，调用者仍可查询原操作，不能自动换新操作号重试。命名空间错误统一 `retryable: false`，不改变活动文件 `Flush` 的原位恢复流程。这不是 ACK 协议或端到端 exactly-once；同操作号的活动请求、已有权威操作或残留记录均拒绝重新执行命名空间命令。write-through `Dirty` 若已经建立新一代工作区，错误附新一代实际提交的操作号，而非上一次成功发布的操作号。`close`/`abort` 返回业务错误后句柄也已终结，再对原 handle 发请求得到 `NOT_FOUND`；这不是原核心操作不存在的证据。要重试活动保存应使用 `flush`，不能重新提交一个结果未知的原操作。

握手和客户端响应接收有 120 秒整帧期限；已认证连接等待下一帧首字节时不设空闲期限，收到首字节后仍须在 120 秒内完成整帧，不能通过逐字节发送续期。socket 写入等待上限仍为 120 秒。服务停止会关闭已登记 socket，以取消空闲阻塞读取。断连和传输错误释放会话活动锁，但不会自动重连、重放不确定写入或复用旧 handle。失败 `close` 现在在返回响应前释放自身路径锁，不再依赖空闲超时。锁只覆盖同分支、相同或祖先/后代重叠路径的 SMB 写入和相关命名空间操作，不是 HTTP 写锁，也不影响其他分支或无重叠路径。

桥接全局最多同时接受 64 条连接，握手仍受上述期限约束；每个映射后的 ZXS 账户最多保留 8 条已认证会话，跨 Samba 用户名别名和仓库共享同一额度。超额握手立即返回资源限制并关闭，不能靠单个账户的永久空闲会话占满全局连接。认证失败、断连、传输错误及服务取消均释放账户会话额度；已准入的空闲会话不因此被超时断开。

## 私有暂存字节预算与恢复

写打开在复制原内容或交付可写 FD 之前，按已有内容大小预留额度；扩展写入由 `PrepareWrite` 在操作前增加并持久保存最高获准文件末端。这个动态额度与文件静态上限不同：静态上限取核心能力与服务端 `--max-upload-bytes` 的较小值，小文件不会预占整个静态上限。每个副本另计 64 KiB 元数据额度。`--smb-spool-bytes-per-account` 默认为 100 GiB，`--smb-spool-bytes-total` 默认为 500 GiB，`--smb-spool-recovery-reserve-bytes` 默认为 1 GiB；这是独立于 HTTP spool 的 SMB 预算，不是存储池总容量配额。原文件超过允许上限时拒绝可写打开，不先复制后检查。每次新增或增长预留时，空闲空间检查保守地额外覆盖全部已获准额度，因此可能早于文件系统实际满盘拒绝新写入；每个复制块和 VFS 写入前再次检查恢复空间。不能以本机制保证外部进程永远不占用同一文件系统。

`*.budget.json` 在复制前持久记录主体与已获准额度，后续增长在回复 `PrepareWrite` 前原子更新。`Flush` 成功不释放额度；断连和重启也不能仅按当前文件长度释放，因为 Samba 仍可能持有可写 FD，已经获准的在途写入仍可能完成。预算依赖受信任 Samba 模块在所有写入前完成预检；断连后不得绕过 RPC 直接扩展 FD，也不把此逻辑额度视为针对恶意本地进程的文件系统强制配额。只有显式成功 `Close`/`Abort` 清理工作文件、恢复记录及预算记录后，才释放额度。命名空间恢复记录也预留元数据额度。失败清理保留原证据与计费，不以删除不确定数据恢复容量。

启动时重建这些预算；没有可验证所有权的旧文件或孤立文件同时计入总量和每个账户的准入，超额时保留数据并拒绝新增预留。损坏预算记录使 SMB 暂存拒绝服务，不静默修复或删除，管理 HTTP 仍独立可用。管理员应先停止 Samba 与桥接服务、确认无残留可写 FD，再按原操作号核对权威结果，导出或明确处理保留数据及配套记录，之后重启重新盘点；不要在服务运行时直接删除预算文件来释放额度。此处没有自动过期清理或替用户决定丢弃未确认写入的接口。

## SDK 与模块构建

固定目标为 Samba **4.23.0**、VFS 接口 **51**。换版本需要重新生成绑定、编译并运行互操作测试；单独的协议测试不能证明 ABI 兼容。

使用官方源码树并完成该目标平台的 Samba configure，保留生成的 `bin/default/include/config.h`、NDR 头文件及所有依赖头。这里不维护手写的 Samba 结构布局。

```sh
export ZXS_SMB_SDK_SOURCE=/absolute/path/to/samba-4.23.0
export ZXS_SMB_SDK_BUILD=/absolute/path/to/samba-4.23.0/bin/default
# 按目标 Samba 配置补充 clang 参数；JSON 数组形式，避免 shell 转义歧义。
export ZXS_SMB_SDK_CLANG_ARGS_JSON='[]'
CARGO_INCREMENTAL=0 cargo build --locked --offline \
  --package smb-vfs --features smb-sdk --release --target-dir target/native
```

构建需要 libclang。`build.rs` 检查源码 `VERSION` 和 `source3/include/vfs.h` 的接口版本，然后从 `includes.h`、`smbd/smbd.h`、`smbd/globals.h` 生成实际 ABI。不同 configure 的额外 include/define 参数必须来自对应 SDK；缺失时构建失败，不能换成猜测的类型定义。

模块文件生成在 `target/native/release/libsmb_vfs.so`，按目标 SDK 的模块路径提供 `zxs.so`。2026-10-20 前一轮 SDK 配置快照（本次关闭生命周期修改之前）的 release 文件 SHA-256：`c72636bbdb5aa31b8b217f7b503828447edeb7fa4b85663dcae9f011c8eadce2`（不是当前源码或跨平台/跨 SDK 配置的固定产物摘要）。SDK 测试还需要链接配置对应的 tevent/talloc 库。

共享配置的基本形状如下。模块参数使用独立键，不能用逗号拼进 `vfs objects`：

```ini
[zxs]
    path = /var/lib/zxs/empty-export
    vfs objects = zxs
    zxs:socket = /var/lib/zxs/runtime/smb/core.sock
    zxs:repository = repo_<32位小写十六进制>
    read only = no
    oplocks = no
    level2 oplocks = no
    kernel oplocks = no
    durable handles = no
    aio read size = 1
    aio write size = 1
    use sendfile = no
    min receivefile size = 0
    store dos attributes = no
    ea support = no
    follow symlinks = no
```

`path` 只能是专用空目录，不能指向版本目录、数据盘根或工作区。历史文档报告测试目录保持为空、配置通过 `testparm`，但当前缺少其原始日志，本轮未重跑；当时 loopback、guest Unix UID 为 root 的测试设置用于验证受限非管理员 ZXS 映射，不是生产认证模板，也不能证明普通 Unix 用户切换时的连接权限。生产用户认证、SMB 最低版本、签名、加密及可信 UID 的完整链路仍须单独验收。

## 验证范围

此前报告的测试快照（不代表本次关闭生命周期修改已重跑真实 SDK）：

- 协议与客户端测试 6 项：实际 Unix socket SCM_RIGHTS 转交及 CLOEXEC、超长帧头拒绝、远端错误保留 code/operation_id 并映射文件系统 errno，以及非法帧后拒绝遗留成功响应、发送失败使所有 clone 断开、业务错误后连接仍可用。
- 服务端桥接测试包括：真实开发模式三盘核心的完整保存与旧版本读、实际发布后丢回执并重试、同文件跨会话锁与 close 保存、不同文件交错发布、命名空间局部锁与仅大小写替换、丢回执后的新字节保留、外部改动拒绝覆盖并保留暂存、任务取消清理，以及已开读句柄的实时撤权。
- SDK 真实库测试 7 项：共享根绝对路径隔离、失败查询保留调用者文件名、异步等待不阻塞宿主事件循环、取消后不触碰已释放宿主状态、锁失败或保存失败仍关闭已移除句柄，以及输入空值/长度检查与异步结果越界拒绝。历史 SMB 服务实测记录中的 4 项是当时的测试快照；本轮只重新执行 SDK 单元测试，未重启 `smbd`。
- 历史文档报告的官方客户端（原始日志当前不可用，本轮未重跑）：`SMB3_11`、模块加载、新文件 83 字节保存并成功 CLOSE、3407872 字节往返比对、43 字节原子覆盖、仅大小写改名及不区分大小写再打开。子目录创建与子文件另在扩展测试执行；原始批次当前不在 checkout，不能当作本轮可复核证据。
- 测试命令：`cargo test --locked --offline -p smb-protocol`；`cargo test --locked --offline -p storage-server smb_rpc::tests`。环境隔离禁止 Unix socket 时需在具备对应能力的测试环境运行，不能将该限制写成测试通过。

关闭生命周期回归源码位于 `test/server/unit/smb_rpc/close_tests.rs`：实际 Unix RPC 和真实开发核心共用 VFS 的描述符关闭策略，覆盖上传拒绝、未决操作、工作区清理失败、丢失已发布回执和锁中毒；检查原字节/恢复记录、同连接及另一连接重新写打开、无关句柄继续保存、终结 handle 不再重提交。该链路不加载 Samba，不能替代固定 SDK ABI 和真实客户端验收。

尚未执行：真实客户端 write-through/FLUSH、Windows/macOS/移动端应用保存与临时文件多句柄矩阵、断网重连、故障注入、真实设备掉电及多账户认证部署。文件存在活动写句柄时，覆盖该路径的命名空间变更会拒绝，不能将已测的关闭后改名解释为所有编辑器保存模式均可用。当前未完整建模文件系统时间戳，剩余空间报告仍来自宿主文件系统，不能当作保护路由可用容量。

历史文档报告 `testparm` 在私有目录 `0700` 上产生建议 `0755` 的浏览警告，同时共享列目录成功，并称未为消除警告放宽核心私有 socket 或目录权限；这些原始日志当前不可复核。2026-10-20 本轮仅构建 SDK/模块和执行单元测试，没有启动 `smbd`、安装系统服务或公开网络端口。

恢复时先查询 `*.work.json` 记录的 `operation_id`：若已发布，保留原发布结果；若未决或查询结果未知，先恢复/核对核心事务，不重新上传或提交原请求。`published` 仅描述最后核对的操作结果，不保证暂存 FD 当前所有字节都已发布；丢回执后它也可能尚未反映实际发布。`descriptor_changes_unconfirmed: true` 表示可写 FD 交出后，仍可能存在尚未收到 `Dirty` 通知的修改；即使 `published: true` 且 `dirty: false`，也必须保留并核对暂存文件，不能据此删除或自动提交后续字节。管理员可先导出完整暂存文件，再决定如何恢复或放弃对应工作区。当前没有自动重新挂接旧 SMB 句柄的能力。

`handle_state: "detached"`、`detach_error` 和 `retained_bytes_synced` 记录失败关闭/显式脱离的原因与当时正文同步是否成功；它们不是发布成功或允许删除的标记，也不保证其他仍持有的 FD 之后没有再改动。脱离时先尝试同步正文和原子刷新记录，再释放锁。若磁盘 I/O 使刷新失败，仍保留现有正文/旧记录并记录带 `operation_id` 和 source 的告警，不以保留活动锁或删除脏文件代替恢复。`active` 或缺少新字段的旧记录不能证明句柄仍在线，尤其在断连或进程退出之后。

断连时无法证明可写 FD 未发生修改，因此普通无写打开、以及成功 FLUSH 后的断连，也会保守保留暂存和恢复记录，不创建新历史版本；这会占用额外空间，应按恢复流程核对后处理。只有成功完成显式 CLOSE（或显式放弃）才清理该句柄的暂存文件与恢复记录；失败 CLOSE、失败放弃和 Detach 不清理。

## FFI 安全与持续验证

手写 VFS 代码启用 `unsafe_op_in_unsafe_fn` 与 `clippy::undocumented_unsafe_blocks` 的 deny 检查；仅由 bindgen 生成的 SDK 代码豁免安全注释 lint，不豁免手写模块。每个不安全边界说明有效指针、长度、所有权或宿主事件循环约束。空值检查不能证明非空裸指针有效，也不能替代匹配的 ABI。同步写入在宿主边界验证空值/长度后转为借用切片，工作线程使用自己的缓冲区；不再由安全函数接收可解引用的任意裸指针。

CI 的独立 `smb-sdk` job 调用 `.github/scripts/smb-sdk.sh`，下载固定官方 Samba 4.23.0 tar（SHA-256 `b0cb963a63eb1515227f3779a46d3a7f790c1bbfeeb4b31fd43e405eefe7a3af`），编译真实 SDK、生成头文件与依赖库，再执行 VFS 全目标 Clippy、SDK 单元测试及 release 模块构建。缓存包含配置后的源码与生成头文件，键包含构建脚本摘要。运行前需要脚本对应的 Ubuntu 开发依赖；脚本不自动安装包、不启动或安装 Samba，也不改宿主服务配置。

```sh
bash .github/scripts/smb-sdk.sh
```

此 job 验证真实 ABI 的编译和事件/内存库单元测试，不等于运行 `smbd` 或真实客户端互操作测试；`smb-protocol` job 仍只验证本机 RPC 协议。源码 tar 校验固定摘要，不宣称验证了发行签名。

## 依赖与许可

协议和服务端桥接是本项目 Rust 代码。ABI 绑定依赖官方 Samba SDK；Samba 是 GPL-3.0-or-later 项目，本目录的 VFS 模块包使用相应许可标识。第三方依赖保留其原包名与许可，不修改其身份。

- 官方目标源码：<https://github.com/samba-team/samba/tree/samba-4.23.0>
- VFS 头：<https://github.com/samba-team/samba/blob/samba-4.23.0/source3/include/vfs.h>
- 本次核查的该头 SHA-256：`854fcd3d14cabc918de6aea755d1af8935ab8d2a4cf5a4d4c17a1e9f06744356`。
- tevent 接口：<https://github.com/samba-team/samba/blob/samba-4.23.0/lib/tevent/tevent.h>
- bindgen：<https://github.com/rust-lang/rust-bindgen>。

## 命名空间边界恢复记录

`*.namespace.json` 使用 `smb-protocol-namespace` 格式，包含身份、分支、操作种类、源/目标路径及 `commit_request`。`publication_outcome: "unconfirmed"` 仅表示已建立意图关联，不能证明准备变更或提交已经执行。记录可先于 Tree 检查和 Namespace staging；因此不能直接重放 `commit_request`。文件或父目录同步失败时虽保留已写证据，但不承诺掉电后的耐久性。先用原 `operation_id` 查询权威操作状态，未决或未知时走核心恢复流程，不重放该文件或另造操作号，也不把存在记录当作失败证明。确认成功后桥接会删除记录；若删除或目录同步失败，残留记录仍应按原操作查询，不重复提交。

相关回归位于 `test/server/unit/smb_rpc/namespace_tests.rs`，覆盖目标种类拒绝、旧模糊删除命令拒绝、实际发布前/后返回错误时保留原操作和工作区、不错误 Abort、成功后的记录清理、成功 RPC 帧丢弃后按客户端原操作号查询、准备/Abort 及文件/父目录同步故障，以及 write-through 新一代操作号。协议测试复用 VFS 实际调用的 flags/操作号构造函数，并用真实 Unix socketpair 验证传输错误保留原操作号和全克隆断开。socketpair 桥接测试不等于真实监听 socket 的进程/权限生命周期，也不等于固定 Samba SDK/客户端互操作；这些仍须独立验证。
