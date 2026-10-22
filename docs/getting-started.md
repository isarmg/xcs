# 本地开发入门

[文档目录](README.md) · [工程结构](20-engineering.md)

以下命令在仓库根目录执行，用目录模拟数据盘来验证开发流程。生产部署见[初始化与维护](17-lifecycle-and-maintenance.md)及[配置规范](21-configuration.md)。


需要 Linux amd64 构建宿主和 `rust-toolchain.toml` 指定的 Rust 1.99.0，当前验收目标为 `x86_64-unknown-linux-gnu`。Web 的 `wasm32-unknown-unknown` 目标不扩展原生平台支持范围。先构建守护进程与离线救援入口，再通过受保护 Web 引导初始化；没有默认密码，也不接受明文密码命令行参数。

```sh
cargo build --locked --offline --target-dir target/native -p storage-server -p storage-rescue
target/native/debug/zxs --root "$PWD/runtime/pool" \
  --management-root "$PWD/runtime/management" --listen 127.0.0.1:7400 \
  --web-dir /absolute/path/to/verified-web-output \
  --setup-disk "P01=$PWD/runtime/disks/P01" --allow-development-setup
```

最低配置为系统盘加一个存储盘，二级、三级、四级及后续备份均可不配置。运行前准备一个现有、私有的测试存储目录和管理目录，并替换 Web 输出路径。操作员在管理目录安全预置 `bootstrap.key`：服务 UID 所有、普通单链接文件、0600、恰好64个十六进制 ASCII 字节且无换行，mtime起30分钟有效。不要将密钥写进命令行、日志或版本库。打开 Web 的首次初始化页，输入引导密钥，选择至少一个批准的存储盘 ID，并设置首次管理员；引导授权不保存在浏览器持久存储中。缺少有效授权时拒绝初始化，不自动生成管理员或数据池。

单盘池只保存一份数据，没有磁盘冗余；该盘损坏可能导致数据永久丢失。新建仓库默认目标份数为 1（包含主盘），不要把“目标已满足”理解为有备份。可选多盘向导按选择顺序建立备份链，高级预置可以指定自定义多盘布局；已有多成员池不会因丢盘自动缩减成员或降低发布证明要求。

`--allow-development-setup` 只用于目录模拟，不证明物理盘独立性。生产必须省略该开关，使用已挂载的独立 Btrfs 数据设备，并核对盘位与物理身份。高级初始配置由本机 `--setup-config ID=/absolute/private-config.json` 预置，再在 Web 选择；浏览器不能传任意路径或配置 JSON。

服务运行时通过 Web 管理、SMB 访问文件。直接启动 `zxs`，不使用 `serve` 子命令。离线救援见[救援说明](components/rescue.md)，需要打开池的救援操作先停服。默认只监听本机，外部访问需要 HTTPS 反向代理和正确 Origin。系统盘管理凭据与数据成员分离，缺失或损坏的数据池不阻止管理 HTTP 启动。

Web 构建、资源服务及真实浏览器验证见 [Web README](components/web.md)。仅提供浏览器和 SMB 用户入口，不安装原生 App。Samba 的目标 SDK、受限用户映射和构建边界见 [Samba README](components/smb.md)。

```sh
cargo test --locked --offline --target-dir target/native
cargo run --locked --offline -p contract-check --target-dir target/native
cargo run --locked --offline -p unicode-build -- --check
rustup target add wasm32-unknown-unknown
bash .github/scripts/check.sh web-production
```

Rust 依赖源码已经纳入仓库，首次业务构建也使用根锁文件和 `--locked --offline`。须预先准备 Rust 工具链、WASM target 和所需系统 SDK；Web 构建器调用本地维护的绑定工具。工具链或 SDK 的安装不属于 Rust 依赖下载。本机 Unix 套接字和所有权隔离测试需要相应系统能力，不能把隔离环境拒绝执行当作业务测试通过。

[持续集成说明](ci.md)列出同一根工作区中按目标选择的服务端、SMB、正式 WASM 构建与浏览器烟测，以及 SDK 和真机验证边界。 标签自动发布、下载校验与安装方式见[发布说明](release.md)。
