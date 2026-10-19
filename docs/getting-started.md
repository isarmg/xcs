# 本地开发入门

[文档目录](README.md) · [工程结构](20-engineering.md)

以下命令在仓库根目录执行，用目录模拟数据盘来验证开发流程。生产部署见[初始化与维护](17-lifecycle-and-maintenance.md)及[配置规范](21-configuration.md)。


需要 `rust-toolchain.toml` 指定的 Rust 1.99.0。先构建服务与 CLI；密码从权限为 `0600` 的私人文件或标准输入读取，至少 12 字节。

```sh
cargo build --locked --target-dir target/native -p xcs-cli
target/native/debug/xcs --root "$PWD/runtime/pool" init \
  --development-disks "$PWD/runtime/disks/P01" "$PWD/runtime/disks/B01" "$PWD/runtime/disks/C01" \
  --username admin --password-file /secure/path/xcs-password
target/native/debug/xcs --root "$PWD/runtime/pool" serve --listen 127.0.0.1:7400
```

`--development-disks` 明确使用目录模拟三个物理设备，仅用于开发。生产配置必须使用三个独立物理设备上的单设备 Btrfs，启动时核对盘位与物理身份。服务运行时，CLI 使用 `--server http://127.0.0.1:7400` 访问同一核心；直接打开本地池的第二写者会被拒绝。默认服务仅监听本机，外部访问配置 HTTPS 反向代理和正确的外部 origin。

Web 构建、资源服务及真实浏览器验证见 [Web README](../web/README.md)。原生平台依赖及平台矩阵见 [App README](../apps/README.md)。Samba 的目标 SDK、受限用户映射和构建边界见 [Samba README](../integrations/smb/README.md)。

```sh
cargo test --locked --target-dir target/native --workspace
cargo run --locked --manifest-path tools/contract-check/Cargo.toml --target-dir target/native
cargo run --locked --manifest-path tools/unicode-build/Cargo.toml -- --check
rustup target add wasm32-unknown-unknown
bash .github/scripts/check.sh web-production
```

首次构建需要联网取得各工作区锁文件中的依赖。`web-production` 会预取 Web 依赖并安装匹配的 wasm-bindgen CLI，再调用内部离线的正式构建器；其余 Cargo 命令在对应工作区缓存齐全后可追加 `--offline`。本机 Unix 套接字和所有权隔离测试需要相应系统能力，不能把隔离环境拒绝执行当作业务测试通过。

[持续集成说明](21-ci.md)列出根工作区之外的独立测试、正式 WASM 构建、三桌面宿主检查与浏览器烟测，以及 SDK 和真机验证边界。 标签自动发布、下载校验与安装方式见[发布说明](release.md)。
