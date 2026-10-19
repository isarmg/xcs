# XCS 标签发布与安装

## 触发与失败边界

`.github/workflows/release.yml` 在推送 `v*` 标签时运行。标签必须是 `v` 加合法 SemVer（如 `v0.1.0`、`v0.2.0-rc.1`），并与根 workspace、所有 `crates/*`、Web/runtime、App bridge/host 的 Cargo 版本及 Tauri 配置版本完全一致。先提交版本及对应锁文件更新，再创建标签。标签保护规则应限制可创建/移动标签的人；此工作流不配置仓库权限。

Release 自己执行锁定测试、契约检查、正式 Web 构建、浏览器冒烟及三个桌面打包任务，不依赖另一个可能尚未完成的 CI。全部成功、资产集合校验通过才进入发布。仅发布任务持有 `contents: write`；不添加持久密钥。先创建草稿并上传全部资产，下载校验 SHA-256，再公开。中途失败保持草稿；重新运行可续传同一源码的本工作流草稿，拒绝修改已经公开的 Release。标签移动会阻止公开发布。公开后如需修复，应发布新版本，不覆盖旧包。

`workflow_dispatch` 只演练同样的构建与验证，生成 Actions 的 `release-complete` artifact，永远不创建标签或 Release。可从 Actions → Release → Run workflow 选择待验证分支。首次标签发布前应先让这项演练成功。工作流失败不代表产物可安装，不能把仅做过静态检查的 YAML 当作实机验收。

预发布标签自动标为 prerelease。预发布不更新 Latest；正式版交由 GitHub 默认规则判定 Latest，不强制把旧标签指定为最新版本。

## 发布资产

将下列 `VERSION` 替换为标签去掉 `v` 的版本：

- `xcs-VERSION-x86_64-unknown-linux-gnu.tar.gz`：Linux x86_64 的 `xcs`，同时提供 CLI 与 `xcs serve` 服务端子命令。不存在另一个独立 `xcs-server` 可执行文件。
- `xcs-web-VERSION.tar.gz`：生产静态目录及 `build-manifest.json`。
- `xcs-app-VERSION-x86_64-unknown-linux-gnu.deb`：Ubuntu 24.04 构建的 Linux 桌面包。
- `xcs-app-VERSION-x86_64-pc-windows-msvc-setup.exe`：Windows x86_64 NSIS 安装包。
- `xcs-app-VERSION-aarch64-apple-darwin.app.tar.gz`：macOS Apple Silicon 的 `.app` 目录归档。当前不提供 Intel Mac 包。
- `INSTALL.md`、`BUILD.txt`、`SHA256SUMS`：安装/重建步骤、源码提交与工具版本、所有其他资产的 SHA-256。

桌面包没有开发者证书签名，macOS 未公证，系统可能阻止运行。这些是供测试验证的产物，尚不代表商店分发或签名安装体验；不建议关闭系统安全机制。Android、iOS、iPadOS 没有冒充可安装包的占位文件；它们仍需 SDK、生成宿主工程、签名和真机验收。此流程不发布 Samba VFS 二进制模块。

## 下载、校验与安装

从同一版本的 GitHub Release 下载所需包和 `SHA256SUMS`。Linux 用 `sha256sum --check --ignore-missing SHA256SUMS`；macOS 用 `shasum -a 256 文件名`，Windows PowerShell 用 `Get-FileHash 文件名 -Algorithm SHA256`，与清单相应行比较。至少校验实际要安装的每个文件；单独的校验清单不能证明发布者身份。确认下载来源和对应标签提交。

Linux CLI/服务端示例：

```sh
VERSION=0.1.0
sha256sum --check --ignore-missing SHA256SUMS
tar -xzf "xcs-$VERSION-x86_64-unknown-linux-gnu.tar.gz"
./xcs --help
./xcs serve --help
```

二进制在 Ubuntu 24.04 构建，依赖其兼容的 GNU libc/运行库，不承诺适用于旧发行版。按 `serve --help` 及仓库运维手册准备数据目录、设备与身份配置，勿直接把测试服务暴露到公网。可将 Web 归档解压到专用静态目录，按 `xcs serve --help` 的 Web 目录选项关联；不要在已有数据目录中直接覆盖解压。

Linux 桌面通过 `sudo apt install ./xcs-app-VERSION-x86_64-unknown-linux-gnu.deb` 安装并解决 WebKitGTK 等依赖。Windows 运行对应 NSIS 安装包，需要 WebView2。macOS 解压 `.app` 归档；正式面向普通用户分发前需要开发者签名和公证。桌面启动、系统文件交接和真实设备行为仍须各平台验收。

## 重建与版本固定

检出 `BUILD.txt` 记录的精确提交，使用仓库 `rust-toolchain.toml` 的 Rust 1.99.0 及提交的所有 Cargo.lock。构建脚本使用 `--locked`，Web builder 在预取依赖后离线构建；工具固定 wasm-bindgen-cli 0.2.129、tauri-cli 2.12.2，均从 crates.io 安装，依赖官方工具自身的锁文件。

```sh
rustup show active-toolchain
rustup target add wasm32-unknown-unknown
cargo run --locked --manifest-path tools/release-tools/Cargo.toml -- v0.1.0
bash .github/scripts/check.sh web-production
cargo build --locked --release --package xcs-cli
# 原生目标机器上，先安装相应官方 Tauri 系统依赖：
RELEASE_VERSION=0.1.0 RELEASE_TARGET=x86_64-unknown-linux-gnu RELEASE_BUNDLE=deb \
  bash .github/scripts/release-desktop.sh
```

Windows 使用目标 `x86_64-pc-windows-msvc` / bundle `nsis`，macOS Apple Silicon 使用 `aarch64-apple-darwin` / bundle `app`，并先用 rustup 安装该 target。桌面脚本从 Rust 图形代码生成 PNG/ICO/ICNS，复用已验证的 `web/dist`，显式设置构建输出目录与目标。包版本来自已经校验的 Cargo/Tauri 配置，不在发布途中改写源码。

版本检查工具 `tools/release-tools` 也由 Rust 实现；shell 仅编排平台工具与 GitHub CLI。GitHub 托管 runner 镜像、系统包及安装器工具会更新，所以这些是可重复执行的构建说明，不是位级可复现承诺。Actions 使用官方仓库固定提交 SHA；更新时重新核对对应官方版本。

官方参考：[Tauri CLI](https://v2.tauri.app/reference/cli/)、[Tauri 分发](https://v2.tauri.app/distribute/)、[GitHub CLI release](https://cli.github.com/manual/gh_release)、[GitHub Actions 权限](https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions#permissions)。

## 发布故障回归测试

`bash .github/scripts/tests/release-publish.sh` 使用临时 Git 仓库和假的 `gh` 命令离线验证：新建、草稿续传、503/403 拒绝创建、已公开版本拒绝覆盖、下载损坏拒绝公开。该测试已纳入 Linux CI 的 `build-tools` suite，不会连接或写入 GitHub。
