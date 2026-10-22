# 本地开发入门

[文档目录](README.md) · [工程结构](20-engineering.md)

以下命令在仓库根目录执行，用目录模拟数据盘来验证开发流程。生产部署见[初始化与维护](17-lifecycle-and-maintenance.md)及[配置规范](21-configuration.md)。


需要 Linux amd64 构建宿主和 `rust-toolchain.toml` 指定的 Rust 1.99.0，当前验收目标为 `x86_64-unknown-linux-gnu`。Web 的 `wasm32-unknown-unknown` 目标不扩展原生平台支持范围。先构建服务与本机维护入口；密码从权限为 `0600` 的私人文件或标准输入读取，至少 12 字节。

```sh
cargo build --locked --offline --target-dir target/native -p storage-admin
target/native/debug/zxs --root "$PWD/runtime/pool" init \
  --development-disks "$PWD/runtime/disks/P01" "$PWD/runtime/disks/B01" "$PWD/runtime/disks/C01" \
  --username admin --password-file /secure/path/zxs-password
target/native/debug/zxs --root "$PWD/runtime/pool" serve --listen 127.0.0.1:7400
```

`--development-disks` 明确使用目录模拟三个物理设备，仅用于开发。生产配置必须使用三个独立物理设备上的单设备 Btrfs，启动时核对盘位与物理身份。服务运行时使用 Web 管理和 SMB 文件访问；远程 CLI 及普通文件传输命令已移除。必要本机维护命令必须先停服，直接打开本地池的第二写者会被拒绝。默认服务仅监听本机，外部访问配置 HTTPS 反向代理和正确的外部 origin。

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
