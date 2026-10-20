# ZXS SMB 桥接

本目录包含 `zxs-smb` 本机协议与 `zxs-smb-vfs` Rust 模块。已用官方 Samba 4.23.0 实际编译 SDK、构建 release 模块、加载 `smbd`，并通过官方客户端的 SMB3.11 文件往返。完整证据及未测范围见[实测记录](../../tests/evidence/smb/README.md)。这不是跨平台应用、生产认证配置或故障恢复矩阵全部通过的声明。

## 目录与职责

| 路径 | 职责 |
| --- | --- |
| `src/protocol.rs` | 有上限的严格 JSON 帧、SCM_RIGHTS 文件描述符转交 |
| `src/client.rs` | 同步本机 RPC 客户端；同一连接上的请求串行化 |
| `vfs/src/module.rs` | Rust 编写的 Samba VFS 回调，不包含自写 C 桥接 |
| `vfs/src/asynchronous.rs` | 两个 Rust 工作线程、有界队列及 eventfd/tevent 完成与取消处理 |
| `vfs/build.rs` | 校验固定 SDK 版本，使用 bindgen 生成 C ABI 绑定 |
| `../../crates/zxs-server/src/smb_rpc.rs` | 身份映射、句柄、私有工作文件、核心提交和读租约 |

客户端的同一连接在全部 clone 之间串行使用。请求发送、响应接收或帧解析失败后，立即关闭并移除共享连接；后续调用明确返回 `NotConnected`，不会把残留响应当作另一请求的结果。完整远端业务错误保留连接。VFS 从句柄表移除文件后立即建立 FD 所有权守卫，锁失败或保存失败也会关闭本地描述符。

`zxs-smb` 是独立 workspace 的默认协议包；根服务端直接依赖它。VFS 位于单独的 `zxs-smb-vfs` 包，默认要求 `smb-sdk` 特性。缺 SDK 时构建模块会失败，不产生可以误认为完整插件的空实现。

## 已实现的桥接语义

- 只允许受信任的本机 Samba 进程 UID 连接，使用 Linux `SO_PEERCRED` 校验；JSON 中没有远程客户端可以自行声明的 ACL、管理员权限或任意核心命令。
- Samba 用户名通过服务端显式配置映射为 ZXS 身份；映射不允许 `admin=true`。服务端的授权包装器在每次操作时重新读取账户状态和权限。
- 分享范围固定到一个仓库的主分支。读打开取得具体版本的文件描述符，服务端保留核心读租约。后续提交不会改变旧句柄读到的版本。每次读取与句柄状态查询仍复核当前账户及 ACL，拒绝以既有 FD 绕过撤权。
- 写打开使用服务端私有目录中的完整独立工作文件，FD 通过 SCM_RIGHTS 转交；RPC 不返回磁盘内部路径。
- 选中的未写入分支继承父版本的内容和大小。只读打开取得父版本的只读描述符，写打开从父内容建立私有工作副本；仅打开后关闭不创建版本，实际写入或显式截断并成功发布后才形成自己的分支版本，父版本不变。目录分支中各个文件分别保留继承或自己的版本。
- `pwrite` 与 `truncate` 修改工作文件后调用 `Dirty`；write-through 的 `Dirty` 同步触发保存。VFS 的 `FLUSH` 和最后 `CLOSE` 经完整文件上传及核心事务提交后才返回保存成功。
- 写锁按分支和规范化路径绑定。不同文件允许并发写入，最终由核心检查各自文件版本并串行发布；同文件跨会话写打开明确拒绝。目录改名、删除和替换目标只与路径覆盖范围内的活动写句柄冲突，读句柄仍固定原版本。
- 重试先查询原 `operation_id`，已发布结果直接返回；未决结果要求恢复，不重新生成操作或再次推进上传代际。
- 保存失败保留工作文件、核心工作区及 `*.work.json` 恢复记录；断开连接不会把脏数据自动发布为历史，也不删除这些文件。丢失发布回执后再次改动的字节不会冒充先前发布的成功结果，须核对原操作并创建新工作区继续保存。
- 目录创建、删除、改名进入核心命名空间事务。允许替换的改名在同一个工作区中执行删除目标及改名，避免发布中间状态；不支持的文件特性明确失败。
- 模块源码拦截链接、复制卸载、FSCTL 写入、权限与扩展属性等可能绕过核心的修改路径，禁止由下层 VFS 修改不可变版本文件。
- 异步读取、写入与同步在有界工作池中完成；工作线程只拥有 FD、缓冲区和 RPC 客户端，不持有宿主指针。取消请求后，不再访问已释放的事件状态。sendfile/recvfile 明确拒绝，避免绕过读取授权与保存边界。
- 文件名规范由核心负责；POSIX stat 区分实际拼写，文件名查找返回核心的规范名，以支持 SMB 不区分大小写打开与保留大小写改名。查找失败时保留调用者原指针，避免破坏宿主后续创建路径。

## 本机 RPC 部署

先建立服务进程拥有的 `0700` 目录，例如 `/var/lib/zxs/runtime/smb`；整个路径不能包含符号链接。服务端绑定其中的 socket，并设置 `0600` 权限。其下 `smb-spool` 和 `empty-directory` 也要求 `0700`。

服务端启动配置需要：

1. socket 的绝对路径。
2. Samba 可信进程连接时的 UID；这个身份是进程代理的信任边界。
3. `SMB 用户名 → ZXS 用户名` 显式映射；ZXS 用户必须已建立并有相应仓库授权。

可信 Samba 进程必须在模拟远程用户前建立受保护连接。目标环境需要验证其实际回调时机与 UID；不能为了让连接成功而把 socket 或目录改成所有用户可读写。

socket 已存在时服务端拒绝覆盖，启动时不会删除未知对象。正常退出或任务取消时，会关闭所有活跃 RPC socket，让阻塞读取退出；只在路径仍对应自身绑定的 device/inode 时移除 socket。服务崩溃遗留的 socket 须由管理员确认后清理。

帧格式为 `u32` 大端 JSON 长度、严格 UTF-8 JSON；最大 8 MiB，分配 payload 前检查长度。可选单个 FD 通过四字节帧头的 SCM_RIGHTS 附带；接收端立即设置 CLOEXEC，拒绝额外或截断的 FD。客户端请求禁止附带 FD。64 位字段使用规范十进制字符串。

仓库在拆分、恢复后默认禁止 SMB，新连接明确返回 `SMB_REBIND_REQUIRED`；管理员须通过受权绑定接口核对历史代际后启用，原共享不自动跳到新历史。

命令为 `hello`、`open`、`authorize_read`、`dirty`、`flush`、`close`、`abort`、`stat`、`list` 和 `namespace`。一个连接只能 `hello` 一次，此后固定身份与仓库。错误带 `code`、`retryable`，保存错误还附原 `operation_id` 供核对。

## SDK 与模块构建

固定目标为 Samba **4.23.0**、VFS 接口 **51**。换版本需要重新生成绑定、编译并运行互操作测试；单独的协议测试不能证明 ABI 兼容。

使用官方源码树并完成该目标平台的 Samba configure，保留生成的 `bin/default/include/config.h`、NDR 头文件及所有依赖头。这里不维护手写的 Samba 结构布局。

```sh
export ZXS_SMB_SDK_SOURCE=/absolute/path/to/samba-4.23.0
export ZXS_SMB_SDK_BUILD=/absolute/path/to/samba-4.23.0/bin/default
# 按目标 Samba 配置补充 clang 参数；JSON 数组形式，避免 shell 转义歧义。
export ZXS_SMB_SDK_CLANG_ARGS_JSON='[]'
CARGO_INCREMENTAL=0 cargo build --locked --offline --manifest-path integrations/smb/Cargo.toml \
  --package zxs-smb-vfs --features smb-sdk --release --target-dir target/native
```

构建需要 libclang。`build.rs` 检查源码 `VERSION` 和 `source3/include/vfs.h` 的接口版本，然后从 `includes.h`、`smbd/smbd.h`、`smbd/globals.h` 生成实际 ABI。不同 configure 的额外 include/define 参数必须来自对应 SDK；缺失时构建失败，不能换成猜测的类型定义。

模块文件生成在 `target/native/release/libzxs_smb_vfs.so`，按目标 SDK 的模块路径提供 `zxs.so`。已验证 release 文件 SHA-256：`606ad728c0db2ac2daa2f6fe742ebdfe2c0474e799721163cafb956c7b5b3425`。SDK 测试还需要链接配置对应的 tevent/talloc 库。

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

`path` 只能是专用空目录，不能指向版本目录、数据盘根或工作区。实际测试中该目录保持为空，所有内容来自核心 FD。完整实测配置已通过 `testparm`；其 loopback、guest Unix UID 为 root 的测试设置用于验证受限非管理员 ZXS 映射，不是生产认证模板，也不能证明普通 Unix 用户切换时的连接权限。生产用户认证、SMB 最低版本、签名、加密及可信 UID 的完整链路仍须单独验收。

## 验证范围

已执行：

- 协议与客户端测试 6 项：实际 Unix socket SCM_RIGHTS 转交及 CLOEXEC、超长帧头拒绝、远端错误保留 code/operation_id 并映射文件系统 errno，以及非法帧后拒绝遗留成功响应、发送失败使所有 clone 断开、业务错误后连接仍可用。
- 服务端桥接测试 9 项：真实开发模式三盘核心的完整保存与旧版本读、实际发布后丢回执并重试、同文件跨会话锁与 close 保存、不同文件交错发布、命名空间局部锁与仅大小写替换、丢回执后的新字节保留、外部改动拒绝覆盖并保留暂存、任务取消清理，以及已开读句柄的实时撤权。
- SDK 真实库测试 5 项：共享根绝对路径隔离、失败查询保留调用者文件名、异步等待不阻塞宿主事件循环、取消后不触碰已释放宿主状态，以及锁失败或保存失败仍关闭已移除句柄。历史 SMB 服务实测记录中的 4 项是当时的测试快照；本轮只重新执行 SDK 单元测试，未重启 `smbd`。
- 真实官方客户端：`SMB3_11`、模块加载、新文件 83 字节保存并成功 CLOSE、3407872 字节往返比对、43 字节原子覆盖、仅大小写改名及不区分大小写再打开。子目录创建与子文件另在扩展测试执行；最终脱敏原始批次见证据目录。
- 测试命令：`cargo test --manifest-path integrations/smb/Cargo.toml -p zxs-smb`；`cargo test -p zxs-server smb_rpc::tests`。环境隔离禁止 Unix socket 时需在具备对应能力的测试环境运行，不能将该限制写成测试通过。

尚未执行：真实客户端 write-through/FLUSH、Windows/macOS/移动端应用保存与临时文件多句柄矩阵、断网重连、故障注入、真实设备掉电及多账户认证部署。文件存在活动写句柄时，覆盖该路径的命名空间变更会拒绝，不能将已测的关闭后改名解释为所有编辑器保存模式均可用。当前未完整建模文件系统时间戳，剩余空间报告仍来自宿主文件系统，不能当作保护路由可用容量。

`testparm` 的测试进程以私有目录 `0700` 运行，产生建议 `0755` 的浏览警告；实际共享列目录成功。没有为消除警告放宽核心私有 socket 或目录权限。本轮仅启动并停止本地隔离测试进程，未安装系统服务或公开网络端口。

恢复时先查询 `*.work.json` 记录的 `operation_id`：若已发布，保留原发布结果；若未决，先恢复核心事务。`published` 仅描述最后核对的操作结果，不保证暂存 FD 当前所有字节都已发布。`descriptor_changes_unconfirmed: true` 表示可写 FD 交出后，仍可能存在尚未收到 `Dirty` 通知的修改；即使 `published: true` 且 `dirty: false`，也必须保留并核对暂存文件，不能据此删除或自动提交后续字节。管理员可先导出完整暂存文件，再决定如何恢复或放弃对应工作区。当前没有自动重新挂接旧 SMB 句柄的能力。

断连时无法证明可写 FD 未发生修改，因此普通无写打开、以及成功 FLUSH 后的断连，也会保守保留暂存和恢复记录，不创建新历史版本；这会占用额外空间，应按恢复流程核对后处理。正常收到并完成显式 CLOSE（或显式放弃）仍清理该句柄的暂存文件与恢复记录。

## 依赖与许可

协议和服务端桥接是本项目 Rust 代码。ABI 绑定依赖官方 Samba SDK；Samba 是 GPL-3.0-or-later 项目，本目录的 VFS 模块包使用相应许可标识。第三方依赖保留其原包名与许可，不修改其身份。

- 官方目标源码：<https://github.com/samba-team/samba/tree/samba-4.23.0>
- VFS 头：<https://github.com/samba-team/samba/blob/samba-4.23.0/source3/include/vfs.h>
- 本次核查的该头 SHA-256：`854fcd3d14cabc918de6aea755d1af8935ab8d2a4cf5a4d4c17a1e9f06744356`。
- tevent 接口：<https://github.com/samba-team/samba/blob/samba-4.23.0/lib/tevent/tevent.h>
- bindgen：<https://github.com/rust-lang/rust-bindgen>。
