# ZXS 原生 App

App 使用 Tauri 2 宿主和共享的 Rust/WASM 界面。`bridge/` 负责会话、文件传输和缓存，`host/` 负责窗口、文件选择与系统打开。文件业务通过 ZXS API 访问，与 SMB 入口分开。

## 使用与实现

- 登录信息由 Rust 桥接层管理。注销成功或服务器确认会话失效后清除凭据；网络失败时保留重试注销的能力。切换会话后，旧请求不会覆盖新会话状态。
- 界面、主题和公共组件来自 `web/runtime`，与浏览器版本共用。远程服务器使用 HTTPS；HTTP 仅用于本机开发。
- 文件通过原生句柄流式上传和下载，不经过 WebView IPC 传递正文。取消或失败的导入会清理临时文件，下载完成后核对大小与摘要。
- 下载缓存按版本保存，系统打开前重新校验。外部编辑后的内容需要重新上传并提交，不会自动回写服务器。
- 文件选择与系统打开由官方 Tauri 插件衔接；本项目的业务与适配逻辑采用 Rust。普通路径和 `file://` 保留真实 UTF-8 文件名；只有 `content://` 文档 ID、无法取得可靠显示文件名的提供方会明确拒绝，请改用网页（单文件上限 64 MiB）或桌面 App 上传。不能将此源码状态视为 Android 文件提供方完整支持。
- 选择结果只向界面返回一次性 ID、名称和大小，上传从已打开的描述符读取。失败后须重新选择；保存结果不明时仍查询原操作，不重复上传。详见[原生上传边界](../docs/design/native-upload-boundary.md)。

会话代次实现在 `bridge/src/connection.rs`，缓存和导入生命周期实现在 `bridge/src/cache.rs`。缓存只接纳普通文件，拒绝符号链接和特殊设备；Unix 缓存目录和文件分别使用 `0700`、`0600`。

## 构建与运行

以下命令在仓库根目录执行。先安装仓库指定的 Rust 工具链及平台依赖，再构建共享界面和图标：

```sh
rustup target add wasm32-unknown-unknown
bash .github/scripts/check.sh web-production
cargo run --locked --manifest-path tools/web-build/Cargo.toml -- icons --product-root .
cargo test --locked --manifest-path apps/Cargo.toml
cargo check --locked --manifest-path apps/host/Cargo.toml
cargo run --locked --manifest-path apps/host/Cargo.toml
```

首次构建需要联网获取锁定依赖；依赖齐全后，Cargo 命令可加 `--offline`。正式 Web 构建脚本会提前获取依赖并安装匹配的 wasm-bindgen CLI。

- Linux：GTK 3、WebKitGTK 4.1、libsoup 3 等开发库及相应运行库。
- Windows：Rust MSVC 工具链、Visual Studio C++ 构建工具和 WebView2。
- macOS：Xcode 命令行工具；签名和公证另需开发者配置。
- Android：Android SDK/NDK，通过 Tauri CLI 生成并构建 Android 宿主。
- iOS/iPadOS：Xcode，通过 Tauri CLI 生成 iOS 宿主，并在目标设备上验证权限、签名与文件交接。

平台依赖见 [Tauri 官方说明](https://v2.tauri.app/start/prerequisites/)，安装包构建见[发布说明](../docs/release.md)。

## 验证状态

[2026-10-19 的 CI](https://github.com/isarmg/zxs/actions/runs/39276866908) 已通过 Linux、Windows、macOS 的桥接测试与宿主编译检查。共享界面已在真实 Chrome 中完成登录、上传、版本详情、下载比对和注销测试。[5bf569e 的发布演练](https://github.com/isarmg/zxs/actions/runs/39276920875) 已完成 Linux、Windows、macOS 打包和产物校验，未公开 Release。

| 范围 | 当前状态 | 下一步 |
| --- | --- | --- |
| Linux、Windows、macOS | 桥接测试、宿主 `cargo check` 与上述提交的发行打包 | 安装、原生 WebView 交互和系统文件交接 |
| Android | 文件选择源码；缺少可靠文件名的 content URI 明确拒绝 | 提供方 DISPLAY_NAME 适配、SDK/NDK 构建、后台生命周期及真机 |
| iOS/iPadOS | 文件 URL 命名与安全范围资源清理源码（含打开失败） | Xcode 构建、签名、后台生命周期、iPad 多窗口及真机 |

编译检查与浏览器测试分别验证构建和共享界面；原生宿主交互、移动端及实际安装按平台单独验收。历史 Linux 局部 SDK 构建记录见[实现报告](../docs/reports/implementation.md)。

本轮上传边界改造已通过桥接与共享界面测试；当前环境缺少 GLib/GTK 开发库，新的宿主编译和真实系统文件选择/打开仍待平台 CI 与设备验收。上述历史 CI 结果不代表这次改动已通过原生宿主验证。
