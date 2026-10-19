# xcs 原生 App

`bridge/` 是独立可测的 Rust 原生业务边界，复用本项目 `xcs-client`。`host/` 是 Tauri 2 宿主，使用与 Web 相同的 `web/dist`，提供 Linux、Windows、macOS、Android、iOS/iPadOS 的统一源码入口；平台包由各自工具链构建。

## 原生职责

- 会话令牌仅保留在 Rust 内存，登录结果传给 WebView 时移除令牌；日志不输出凭据。通用 API 请求不能绕过原生连接命令登录，恢复会话也不向 WebView 返回 CSRF 凭据。退出失败时保留凭据供锁定界面重试服务器注销；确认成功或服务器明确返回 401 后才清除，同时使在途登录失效。连接尝试与活动会话代次分别管理，失败登录保留原连接，旧登录/退出/传输/缓存校验结果不能覆盖新会话。
- 公共登录、账户、页面壳、主题和对话框均由项目内 `web/runtime` 提供。自助改密确认成功后同时清除 Rust 内存会话并返回登录；密码错误保留当前连接。
- JSON 请求严格限制到所连接服务器的 `/v1/`，共享客户端禁止远程明文 HTTP、URL 凭据和重定向。
- 原生文件选择使用官方 dialog 插件；Android content URI 与 iOS 安全范围文件通过官方 fs 插件解析句柄，Rust 流式导入私有暂存目录；iOS 句柄使用结束后释放安全范围资源。
- 文件上传、封存/JCS 提交与校验下载使用 Rust 客户端，文件字节不走 IPC。导入临时文件由 `StagedImport` 跨后台复制与异步返回持有，读取失败或调用取消会清理；只有成功接收选择结果后才保留，成功上传后再清理。
- 固定版本保存到 `versions/<version_id>/<当前文件名>` 缓存，完整保留合法 240 字节文件名，不再拼版本前缀而超长。初次下载与复用共享路径校验；只有目标已存在时才尝试复用，再次打开前重新校验。缓存子目录和现有缓存文件拒绝符号链接，现有条目必须是普通文件（拒绝目录和特殊设备，避免校验阻塞）；Unix 新子目录为 0700、文件为 0600；旧版缓存不会自动删除或迁移。外部编辑形成的本地变化需要重新上传工作区并提交。
- 系统打开仅允许应用缓存内的已验证文件；使用官方 opener 插件。主窗口的自定义命令有明确权限清单，未授权远程页面不能调用。

官方插件可包含平台 Java/Kotlin/Swift 等代码；本项目维护的宿主、适配、界面与业务逻辑全部为 Rust。

`bridge/src/connection.rs` 负责连接尝试及会话代次，`cache.rs` 负责缓存身份和导入临时文件；宿主只处理文件提供方、权限与系统打开，不重新定义这些规则。

## 构建与检查

先构建 Web，并生成应用图标：

```sh
cargo run --locked --offline --manifest-path tools/web-build/Cargo.toml -- build --product-root .
cargo run --locked --offline --manifest-path tools/web-build/Cargo.toml -- icons --product-root .
cargo test --locked --offline --manifest-path apps/Cargo.toml
cargo check --locked --offline --manifest-path apps/host/Cargo.toml
cargo run --locked --offline --manifest-path apps/host/Cargo.toml
```

Linux 需要 GTK 3、WebKitGTK 4.1、libsoup 3 等开发库。Windows 需要 WebView2 和对应 Rust/C++ 链接工具；Apple 平台需要 Xcode 与签名配置；Android 需要 Android SDK/NDK。使用 Tauri CLI 的 `android init` / `ios init` 生成对应原生工程，再在各平台运行 `android build` / `ios build`。生成的第三方宿主工程不承载自写业务逻辑。

本次环境使用项目 `target/sysroot` 局部提取 Linux SDK，未向系统安装或升级软件。对应检查可设置：

```sh
PKG_CONFIG_PATH="$PWD/target/sysroot/usr/lib/x86_64-linux-gnu/pkgconfig:$PWD/target/sysroot/usr/share/pkgconfig" cargo check --locked --offline --manifest-path apps/host/Cargo.toml
```

完整平台运行、后台持续传输、系统文件关联回传、签名与分发应在各平台 SDK 和设备上验收。源码入口和 Linux 检查不能代替六平台实测。

| 平台 | 当前验证范围 | 尚需平台验证 |
| --- | --- | --- |
| Linux | Rust 原生桥接测试；Tauri 使用局部 GTK/WebKit SDK 的检查与可执行文件链接构建通过 | 宿主交互、系统文件选择/关联编辑器、打包 |
| Windows | 统一 Rust 源码入口 | WebView2、链接、安装包与系统交接 |
| macOS | 统一 Rust 源码入口 | Xcode、签名、打包与系统交接 |
| Android | Rust 文件选择及 content URI 适配源码 | SDK/NDK、文件提供方、后台生命周期和真机 |
| iOS | Rust 文件选择及安全范围资源释放源码 | Xcode、签名、后台生命周期和真机 |
| iPadOS | 复用 iOS 宿主与窄屏/宽屏 Web 界面 | iPad 真机、多窗口及系统交接 |

局部提取的 WebKit 运行库仍把 helper 进程定位到系统绝对路径，当前环境的 Xvfb 启动探针因此未完成运行验证。此处只记录实际检查和链接结果；标准 Linux 发行环境仍应安装完整运行依赖并执行交互验收。
