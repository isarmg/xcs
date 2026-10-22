# ZXS Web 与 SMB 发布

当前业务入口为 Web 和 SMB。服务端使用 `zxs` 提供业务 API、Web 静态资源和本地 Samba 桥接；Samba VFS 模块单独按固定 SDK 构建。

## 触发与验证

`.github/workflows/release.yml` 在推送 `v*` 标签时运行；标签须为 `v` 加合法 SemVer，并与根 workspace 中第一方产品包版本一致。先提交版本与锁文件更新，再创建标签。`workflow_dispatch` 只演练构建，保留 `release-complete` artifact，不创建标签或公开 Release。

发布流程执行源码、格式、严格 Clippy 与安全检查、业务测试、契约检查、正式 WASM 构建和真实浏览器冒烟。质量门禁直接属于发布的 `verify` 依赖，失败会阻止公开版本。全部成功后校验产物集合，创建草稿、上传、下载核对 SHA-256，再公开。失败保留草稿；重试仅续传同一源码的本流程草稿，拒绝覆盖已公开版本。发布前后均核对标签提交。只有发布任务持有 `contents: write`。

预发布标签标为 prerelease 且不更新 Latest；正式版由 GitHub 默认规则决定 Latest。

## 发布文件

- `zxs-VERSION-x86_64-unknown-linux-gnu.tar.gz`：Linux x86_64 的直接守护进程 `zxs` 和最小离线救援工具 `zxs-rescue`。迁移分支中的旧管理入口不进入发布包；发布前须通过 Web 初始化和日常管理替代验收。
- `web-interface-VERSION.tar.gz`：正式 Web 静态目录，包括 WASM、绑定文件及 `build-manifest.json`。
- `INSTALL.md`、`BUILD.txt`、`SHA256SUMS`：安装与重建说明、源码提交及构建工具记录、全部其他资产的摘要。

两个归档均携带完整 `docs/`、根说明和中央 `compliance/`，包括操作手册及 SMB 集成说明。独立 `INSTALL.md` 从[发布安装模板](templates/release-install.md)生成，不依赖从源码文档目录复制后的相对链接。源码与构建脚本引用须在 `BUILD.txt` 指定的仓库提交中查阅。

此流程不发布 Samba VFS 二进制。VFS 必须使用项目固定的 Samba 4.23.0 SDK 构建并核对 ABI，步骤见 [SMB 集成](components/smb.md)。真实 Samba 操作、物理盘故障和恢复仍需专项验收，构建通过不能代替这些验证。

## 下载与安装

从同一版本的 GitHub Release 下载所需文件和 `SHA256SUMS`，确认对应标签及提交，再校验每个实际安装的文件。Linux 示例：

```sh
VERSION=0.1.0
sha256sum --check --ignore-missing SHA256SUMS
tar -xzf "zxs-$VERSION-x86_64-unknown-linux-gnu.tar.gz"
./zxs --help
./zxs-rescue --help
```

服务端在 Ubuntu 24.04 构建，要求兼容的 GNU libc 及系统运行库。根据 `zxs --help` 和运维文档准备数据目录、设备与认证配置。将 Web 归档解压到独立静态目录，按服务选项关联该目录；不要覆盖解压到已有业务数据目录。

## 离线重建 Rust 源码

检出 `BUILD.txt` 记录的精确提交，准备 `rust-toolchain.toml` 指定的 Rust 工具链及 WASM 标准库。第三方 Rust 源码由仓库提供，首次正式业务构建也使用 `--locked --offline`；CI 和 Release 使用空的独立 Cargo home，不缓存 registry/git 源码。

```sh
rustup show active-toolchain
rustup target add wasm32-unknown-unknown
python3 tools/rust-sources/manage.py verify --metadata
cargo run --locked --offline -p release-tools -- v0.1.0
bash .github/scripts/check.sh web-production
python3 .github/scripts/check-rust-diagnostics.py -- \
  cargo build --locked --offline --release --package storage-server --package storage-rescue --message-format=json
```

正式 Web 绑定工具来自 [绑定生成工具](tools/web-bindings.md)：从 wasm-bindgen-cli 0.2.129 提取 Web 命令，删除测试运行器、其他目标命令及网络/TLS 依赖。其版本字符串保持 `wasm-bindgen 0.2.129`，与 Web schema 匹配；原始来源、许可证及修改记录纳入[统一源码清单](THIRD_PARTY_RUST_SOURCES.md)。

Python 3 校验源码清单；安全扫描独立管理 cargo-audit，并在线读取漏洞公告与 yanked 状态，所有发现及扫描不完整均阻止通过。Rust 工具链、SQLite、Samba、Linux 系统库及必要 SDK 继续按既有方式准备。托管 runner 和原生依赖可能更新，因此以上流程保证可追溯重建，不承诺位级一致。

## 发布故障回归

`bash .github/scripts/tests/release-publish.sh` 在临时仓库使用假的 `gh` 离线检查新建、草稿续传、503/403 拒绝创建、已公开版本拒绝覆盖、下载损坏拒绝公开。它纳入 `build-tools` suite，不会写入 GitHub。
