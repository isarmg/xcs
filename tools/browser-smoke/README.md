# 真实浏览器冒烟测试

独立 Rust 工具，直接调用本机 ChromeDriver 的 W3C WebDriver / Chrome DevTools 协议；不引入 Python、Node 或业务 JavaScript，不模拟 API、不替换页面资源。少量浏览器端脚本观察错误、已渲染内容和 computed style；独立无障碍回归还临时注入并移除一条装饰故障 CSS，验证真实轮廓控制器的降级行为。

## 依赖与运行

需要 Linux、项目固定 Rust 工具链、匹配的官方 Chrome / ChromeDriver、PATH 中的 `kill`。Chrome 保持默认沙箱，不添加 `--no-sandbox`。GitHub Ubuntu runner 使用已安装的匹配版本；其他环境从官方 Chrome for Testing 下载同版本浏览器与驱动。浏览器或驱动缺失、沙箱禁止启动、页面加载失败均返回非零，不跳过测试。

先按 `web/README.md` 构建真实 `web/dist`，并构建 CLI：

```sh
cargo build --locked --target-dir target/native -p zxs-cli
export CHROME_BIN="$(command -v google-chrome)"
export CHROMEDRIVER="$(command -v chromedriver)"
cargo run --locked --manifest-path tools/browser-smoke/Cargo.toml -- \
  --cli-bin "$PWD/target/native/debug/zxs" --web-dir "$PWD/web/dist"
```

`CHROME_BIN` 可省略，ChromeDriver 会自动发现浏览器；`CHROMEDRIVER` 默认为 PATH 中的 `chromedriver`。这些变量接受可执行文件完整路径，不接受命令参数。Cargo 单测只验证测试工具自身，不能替代上面的浏览器命令。

## 真实页面截图

可添加 `--screenshot artifacts/web-file-manager.png`：使用产品现有中文语言参数，在真实上传完成后保存 Chrome WebDriver 返回的原始 PNG。截图额外上传三个纯合成示例文件，不修改页面 DOM、主题或 API 响应；原有下载、历史和注销验收仍完整执行。截图中使用独立临时开发池，不包含用户文件或凭据。

CI 通过 `ZXS_SMOKE_SCREENSHOT` 传递路径，浏览器验收成功后将图片保存为 `web-screenshot` artifact。README 展示图从该 artifact 原样取用。

## 验收范围

1. 用随机临时密码创建独立三目录开发池，启动真实 ZXS 服务；浏览器登录后必须看到三个磁盘目录。
2. 在 P01 上传 UTF-8 文件，等待目录实际发布。
3. 打开文件详情与版本历史，触发真实浏览器下载并逐字节比对内容。
4. 在操作历史中确认该文件出现。
5. 注销并刷新，仍然必须呈现匿名登录页；检查未捕获异常、JavaScript 严重错误、意外 HTTP 错误以及登录、磁盘、历史、注销和 WASM 的成功网络响应。
6. 用真实主题按钮切换明暗模式，并用 CDP 检查普通及 forced-colors 模式：账户密码输入接受 WebDriver Tab 后必须具有可见焦点环；历史筛选的真实 Select 必须完成绘制并可见。普通模式下临时注入 inset shadow，使真实控制器进入不支持装饰的方形 fallback，仍必须可见。注入规则、媒体模拟和主题在结束或失败后恢复；不伪造组件状态、API 或轮廓标记。
7. 在真实 WASM 发布启动就绪事件的同一调用栈中注入 `pagehide` 或启动失败事件，验证非缓存离开和失败会同步卸载真实节点、清除 owner 的轮询定时器；缓存离开则保留节点和定时器，并继续完成真实会话恢复。定时器观察器原样调用浏览器原生方法，不替换响应或响应式状态。每次移除注入器，最后再次验证正常导航与刷新。此项覆盖资源释放契约，不声称测量全部堆内存或真实 bfcache 命中率。
8. 暂缓真实 `/api/auth/me` 原始 Response 的交付，在已挂载应用卸载后释放同一个 Response，验证迟到的会话恢复不会访问已释放 owner、重新挂载页面或遗留轮询定时器。原始状态码、响应头和正文保持不变；成功或失败均释放等待并移除注入器。

所有数据放在权限受限的临时目录，密码只经 0600 临时文件初始化、不进入命令行。ZXS 仅监听 127.0.0.1；驱动仅允许本机连接。服务、驱动与 Chrome 在同一测试进程生命周期内创建，并通过独立进程组在正常结束和失败时清理；不会修改用户的浏览器配置、现有池或真实磁盘。浏览器配置由 ChromeDriver 为新会话临时创建。失败会输出服务和驱动日志；临时数据自动删除。

这是小范围贯通测试，不等同于生产磁盘、真实 Samba、原生六平台或完整 UI 回归验收。CI 的 browser-smoke job 必须执行真实浏览器命令，不能将工具编译或单测当作 E2E 通过。

## 可选宣传素材采集

设置 `ZXS_DEMO_CAPTURE=artifacts/demo-capture` 可额外通过真实界面生成合成的 `活动方案.txt`：预算 8、10、12 万元三个正文版本，从 8 万历史节点创建“精简方案”，切换并逐字核验继承正文，再写入 6 万并切回“主方案”核验仍为 12 万。使用 1920×1080 CSS 像素的实际浏览器视口和 DPR 3（原始 PNG 为 5760×3240），逐步保存原始 PNG、控件坐标和操作时间到 `capture.json`；`viewport.json` 记录视口及像素比例，裁切原图时控件 CSS 坐标需乘 DPR；全部成功后才写 `verification.txt`。不设置此变量时不执行该流程。CI 的 `zxs-demo-capture` artifact 可含失败前的局部素材，只有 `verification.txt` 与成功日志证明完整流程通过。

## 六平台共享 UI 视口预览

设置 `ZXS_PLATFORM_PREVIEWS=artifacts/platform-previews` 可保存 Linux (1366×900)、Windows (1440×960)、macOS (1512×982)、Android (412×915)、iOS (390×844)、iPadOS (1024×1366) 六个 CSS 视口的原始 Chrome PNG，命名为 `平台-shared-ui-preview.png`。每张均为真实生产 WASM、真实临时服务上的传输任务页，不伪装操作系统窗口，不覆盖 User-Agent，也不代表对应原生 WebView、触控设备或应用商店构建已验收。手机截图保留正常滚动，不为在一屏塞入全部记录而缩小公共行高。

传输验收不依赖截图开关：通过真实文件选择器选取合成文件，验证发布成功任务、超过 64 MiB 的真实本地预检失败任务、状态筛选、六视口无横向溢出、长中文名称/目标路径自然换行，以及真实 Tab 键焦点。另一个真实上传的原始 Response 在返回给 WASM 前暂缓交付，使截图包含真实 `uploading` 状态；不修改内容字节、状态码、头或应用状态。截图后恢复交付并等待真实 `published`，失败时也释放响应并清理视口设置。所有截图来自同一会话和同一批任务。

`source.json` 记录来源 commit、工作区是否有改动、实际 Chrome 版本、实际视口、DPR、状态和非原生预览标签。只有真实浏览器任务成功才能说明验收通过；工具的编译/单测不能产生或替代这六张截图。原有文件选择器拦截、无障碍和跨会话下载隔离测试仍执行。

## 字体故障与非阻塞启动

独立回环代理只暂缓原始字体 HTTP 字节或为字体返回 404，其余资源与 API 响应原样转发。真实生产 WASM 必须在两字体仍挂起时发布 ready，登录输入可用；释放原始字体后字体状态独立完成，并解码真实 border-image SVG，核验透明圆角、可见填充和九宫格物理尺寸；紧凑图块按百分比切片与实际圆角范围核对，不错误要求其图像尺寸等于控件尺寸，也不要求不存在的装饰子节点。缺失字体必须保留可用回退界面；挂起期间非缓存 pagehide 后的迟到字体完成不得再写状态或恢复节点。代理在失败和退出时释放等待并关闭，最长等待 90 秒。截图路径仍显式等待字体加载，故障验收在正常截图与业务验收之后执行。测试工具单测只证明代理转发/故障机制，不替代真实 Chrome 验收。
