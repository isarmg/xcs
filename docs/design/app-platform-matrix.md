# 六平台 App：实现边界与验收矩阵

本表描述项目源码与可追溯证据，不把共享页面预览、宿主编译或历史打包等同于设备验收。桌面构建基线为 [92f30df / CI 39389754286](https://github.com/isarmg/zxs/actions/runs/39389754286)，三个 desktop-host 作业均成功。本轮尚未提交的修改不在该 CI 证据内。

## 窗口与共享界面

`apps/host/tauri.conf.json` 声明一个 `main` 窗口，初始 1200 × 850、最小 360 × 560，保留默认系统窗口装饰。不引入模拟的系统标题栏，也不把浏览器视口检测当作原生平台识别。iOS/iPadOS 共用移动入口；目前没有多窗口会话管理。

共享界面的六种布局验收应覆盖：

| 平台 | 重点检查 | 必须补充的原生环境 |
| --- | --- | --- |
| Linux | 桌面侧栏、完整文件表格、最小窗口与键盘焦点 | GTK 3 / WebKitGTK 4.1 / libsoup 3，实际 Linux 桌面 |
| Windows | 窗口缩放、125% / 150% DPI、长中文文件名、键盘操作 | MSVC / Windows SDK / WebView2，实际 Windows 桌面 |
| macOS | 系统标题栏、窗口缩放、触控板、中文名称 | macOS / Xcode，发布时另需签名和公证配置 |
| Android | 360 px 窄屏、横屏、系统返回、软键盘、系统栏遮挡 | SDK / NDK / Java / Tauri Android 工程及设备 |
| iOS | 窄屏、刘海和底部安全区、横屏、键盘遮挡 | macOS / Xcode / Tauri iOS 工程、签名与 iPhone |
| iPadOS | 平板宽屏、分屏窄视口、旋转、硬件键盘 | 同上及 iPad；多窗口不能仅凭响应式布局宣称支持 |

安全区与视口尺寸属于共享布局的责任；真正的系统栏、软键盘、WebView viewport 行为必须在设备上测试。任务列表不应仅用颜色区分状态；窄屏仍应能访问文件名、状态、错误原因和恢复操作。

## 文件交接与能力限制

| 能力 | 桌面三平台 | Android | iOS / iPadOS |
| --- | --- | --- | --- |
| 原生选择文件 | 实现；实际选择器交互待验收 | 插件返回 URI，但当前 `content://` 明确拒绝 | 文件 URL 路径实现；Files/iCloud 提供方待验收 |
| 权威文件名 | 路径真实 UTF-8 basename | 尚无 DISPLAY_NAME 桥接，不能使用 document ID 猜名 | 文件 URL 解码后的真实 UTF-8 basename |
| 上传正文 | Rust 已打开描述符，单次 source ID | 有可靠名称的普通路径可走导入；不能据此称 SAF 可用 | Rust 私有临时导入，scoped-access 清理覆盖失败 |
| 下载 / 系统打开 | 缓存校验后调用 opener；设备交接待验收 | Rust 缓存链路已有，系统打开/FileProvider 待设备验收 | Rust 缓存链路已有，系统交接待设备验收 |
| 后台 / 重启续传 | 未实现持久上传队列 | 未实现后台传输服务或重启恢复 | 未实现后台传输会话或重启恢复 |
| 离线缓存打开 | 当前操作仍需会话与版本校验，不能承诺离线可用 | 同左 | 同左 |

六个平台共享上传协议，不代表各自的文件提供方行为相同。网页单文件 64 MiB 限制不应错误套用到原生描述符上传。选择取消返回空列表；来源一次性消费；失败后重新选择；提交结果不明时查询原操作，不重传。参见[原生上传边界](native-upload-boundary.md)。

## Android DISPLAY_NAME 的后续实现路线

当前锁定版本为 `tauri-plugin-dialog 2.8.3`、`tauri-plugin-fs 2.7.1`。检查其源码可见：dialog 返回 URI，fs Android Rust 接口提供 `open`，没有公开的显示名称查询方法。插件内部辅助方法不等同于可调用的公开 Rust API；不得修改 registry/vendor 源码或凭空调用 `metadata` 一类接口。

待 SDK/NDK 环境具备后，可实现项目自有的小型 Android 平台适配：

1. 仅处理本次系统选择器返回、仍在当前会话代次内的 URI。不可向 WebView 开放任意 URI 读取命令。
2. 在平台侧通过 `ContentResolver.query` 查询 `OpenableColumns.DISPLAY_NAME`，关闭 cursor；缺列、空结果、空名称、权限失效或查询失败均返回明确错误，不退回 URI 尾段或 document ID。参见 [Android 官方文件信息文档](https://developer.android.com/training/secure-file-sharing/retrieve-info)。
3. 名称经过现有服务端命名规则检查；内容仍由原生描述符流式读入私有导入，大小以实际导入结果为准，不信任提供方声称的大小。
4. 使用系统选择授予的临时访问范围，不增加全盘读取权限、不持久化 URI 授权、不增加网络或凭据权限。[Storage Access Framework](https://developer.android.com/training/data-storage/shared/documents-files) 支持用户选择范围内访问。
5. 必须用真实 SDK 编译，并以 Downloads、外部存储及至少一个云文件提供方测试中文/空格/同名文件、缺 DISPLAY_NAME、拒绝访问、未知大小、取消、切换账号、导入失败和后台切换。验证前保持现有明确拒绝行为。

这是实现计划，不是已交付的 Android SAF 支持。

## 宿主权限与交付门槛

当前 capability 仅匹配 `main`，包含 `core:default` 及八项自有业务命令；未向 WebView 授权通用 fs/dialog/opener 命令。原生文件操作从 Rust 内部调用插件。CSP 的连接目标仍限于自身和 Tauri IPC；服务端请求由 Rust 桥接发出。本轮不扩大权限范围。

每个平台验收应记录提交、工具链/OS/WebView 版本、设备、构建日志、安装包校验和及实际交互结果。至少走通：登录 → 选择（含取消）→ 上传 → 任务状态 → 版本详情 → 下载校验 → 系统打开 → 注销；再覆盖网络中断、提交结果不明与账号切换。移动平台另需验证后台恢复时不把未知提交标成成功。未取得这些证据的平台明确列为待验收，而不是“已完成”。
