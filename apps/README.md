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

## 六个平台与验证状态

[92f30df 的 CI（39389754286）](https://github.com/isarmg/zxs/actions/runs/39389754286) 已通过 Linux、Windows、macOS 的桥接测试与宿主编译检查。此证据只对应该提交，不自动覆盖之后的工作区改动，也不是原生设备交互验收。

[5bf569e 的历史发布演练](https://github.com/isarmg/zxs/actions/runs/39276920875) 已完成 Linux、Windows、macOS 打包和产物校验，未公开 Release；它不是当前提交的发行产物。

| 平台 | 已有实现 / 证据 | 尚需完成的验收 |
| --- | --- | --- |
| Linux | 共享界面、原生选择/打开、桥接测试与宿主 CI 编译 | 安装、真实 WebKitGTK 窗口、系统文件交接 |
| Windows | 共享界面、原生选择/打开、桥接测试与宿主 CI 编译 | 安装、WebView2、高 DPI、系统文件交接 |
| macOS | 共享界面、原生选择/打开、桥接测试与宿主 CI 编译 | 安装、WKWebView、签名/公证、系统文件交接 |
| Android | 共享界面和移动入口源码；content URI 明确拒绝 | 自有 DISPLAY_NAME 适配、SDK/NDK 构建、系统栏/键盘、生命周期、真机文件交接 |
| iOS | 共享界面、文件 URL 命名及 scoped-access 清理源码 | Xcode 构建、签名、Files 提供方、safe area/键盘、生命周期、真机 |
| iPadOS | 复用 iOS 宿主及共享界面，不是独立桌面宿主 | iPad 设备构建、分屏/旋转/键盘、Files 提供方；多窗口仍未实现 |

具体窗口、权限和文件交接边界见[六平台能力与验收矩阵](../docs/design/app-platform-matrix.md)。六种视口的共享 UI 页面或截图只证明布局，不能替代六个平台的原生构建及设备验收。

本地环境缺少 GLib/GTK 开发库，不重复安装或绕过平台依赖；桥接测试可独立执行。Android/iOS 生成工程未纳入当前仓库，没有移动 SDK/设备的验证结果。历史 Linux 局部 SDK 构建记录见[实现报告](../docs/reports/implementation.md)。
