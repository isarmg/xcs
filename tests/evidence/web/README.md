# Web 验收记录

本页是早期技术界面的历史验收记录；当前文件管理、目录批量分支与资源身份以[新版记录](../file-manager/README.md)为准，以下旧截图和计数不代表当前运行页面。

2026-10-18，Rust 1.99.0、wasm-bindgen 0.2.129、Chromium，Linux x86_64。

浏览器加载项目构建的真实 `web/dist`，连接本地真实 xcs-server 的 `/v1/`，未拦截或伪造接口响应。服务端使用开发模拟磁盘 P01 → B01 → C01；记录不证明物理盘断电恢复或六平台原生 App 实测。

| 验收 | 结果 |
| --- | --- |
| WASM release、独立 CSR 引擎 | 构建通过；浏览器出现 `data-xcs-runtime="xcs-web"`、`data-xcs-ready="true"` |
| Rust Web workspace | 37 项测试通过：22 项运行环境（启动终态、语言和连续轮廓）、3 项组件属性、2 项响应式生命周期、10 项应用（原保存结果/迁移分支/路径约束，以及普通用户身份、壳状态稳定、改密失败保留会话和封存工作） |
| 复用页面与视觉 | 公共 AppShell、LoginPage、AccountPage、Header、通知与 Dialog 已接入；登录卡居中且比例 3:2，两字重字体实际解码，3 个连续轮廓实际绘制 |
| 登录、建仓、同工作区建目录并上传 | 真实 HTTP 通过，文件 66 字节，含中文路径 |
| 封存与规范请求摘要提交 | 服务端返回 `PUBLISHED`、`COMMITTED_AND_PROTECTED` |
| 固定版本预览、下载 | `<script>` 保持文本；下载字节相同，界面先进行 BLAKE3 校验 |
| 文件历史、全量分支、任务 | 真实 HTTP 通过；显示发布时保护和当前健康 |
| 历史树拆分与边界 | 真实计划预览及提交通过；显示 1 张独立边界审计卡，外部审计 ID 没有跳转链接；已迁移分支不再请求失效文件树 |
| 主题、语言 | 公共文案实际切换中文→英文→中文并更新缓存；无手动偏好时跟随系统，手动选择后保持并跨刷新保留 |
| 对话框与账户 | 原生 Dialog 的 Tab 边界保留焦点；用户名只读，错误当前密码保留登录，改密成功后返回登录并用新密码进入 |
| 磁盘与路由、窄屏 | 3 个盘位；390 px 视口没有页面横向溢出 |
| 退出及刷新 | 退出成功，刷新后保持未登录 |
| 浏览器错误 | 最终产物 38 次 API 请求，无页面异常、意外 HTTP 错误或 CSP 违规；有意提交错误当前密码的 403 不计意外错误 |
| 启动失败与恢复 | 独立浏览器 6 个场景全部通过：正常字体/轮廓、绑定 JS 失败、WASM 失败、字体失败、30 秒超时与迟到资源不能挂载、恢复资源后重载成功；失败页使用系统字体与 alert 语义，无伪 ready 标记 |
| 资源交付 | 运行中的 7400 全部 20 份清单资源 SHA-256 匹配；WASM magic/MIME、gzip、ETag/304、nosniff、CSP 通过 |
| 原生桥接 | 1 项凭据不进入 IPC 的测试通过 |
| Linux Tauri | 局部 GTK/WebKit SDK 的可执行文件重新链接成功，嵌入新资源；宿主图片 CSP 纳入共享 SVG 所需 data:；运行时 helper 路径限制见 `apps/README.md` |

最终 WASM 为 1069842 字节，gzip 为 339543 字节；响应内容与当前构建清单逐字节匹配。WASM SHA-256 为 `9589b3c2ea6a0807d3f1ea633b2ae3188825654557c513b8b4378f98965b31d9`，gzip SHA-256 为 `ba91c63df65dd11738b28b6c7e03578790c953d0204941de3208cf42f70e0ce4`。服务 ETag 对应未压缩内容的 BLAKE3：`91866a6aa8b895e24272af067d0ccdf9f7c5d2789e278cb9cdcc4ca08aac0792`。

降级发布和发布后损坏的提示差异由三份真实服务端 wire 数据的 Rust 回归验证；本次浏览器保存使用完整保护成功路径，没有通过浏览器制造磁盘损坏。

[业务结果](result.json)、[启动故障结果](startup-result.json)、[主题与语言结果](preferences-result.json)、[资源校验结果](asset-result.json)、[资源响应头](wasm-response-headers.txt)、[构建清单](build-manifest.json)、[Rust 测试输出](rust-tests.txt)、[WASM 检查](wasm-check.txt)、[最终构建](release-build.txt)、[Linux 宿主重链](linux-host-build.txt)。这些文件不包含密码、Cookie、会话令牌或私有认证请求。

启动探针的[首次最终产物复跑](startup-first-final-attempt.json)有 1 项断言失败，原始记录缺少失败现场，不能据此确定其精确时序。随后[受控诊断](startup-probe-setup-race.json)复现了测试前提的问题：CSS 提前请求常规字体，但 WASM 尚未开始粗体加载时就推进时钟，会使“迟到后两个字重均解码”的断言失败；现场启动终态仍为 failed，没有被迟到结果覆盖。最终探针明确等待两种字体真实请求后才快进，6 项全部通过；此过程只修正临时测试的同步，没有修改产品代码。

截图：[公共登录页](login.png)、[文件列表](files.png)、[只读预览](preview.png)、[拆分边界审计](boundary.png)、[桌面磁盘管理](desktop.png)、[390 px 磁盘管理](mobile.png)、[公共账户页](account.png)。

浏览器业务测试使用单独初始化的临时开发池；7400 用户预览池只执行就绪、资源校验和新建探测会话的登录/退出，原管理员密码和用户数据保持不变。三成员改密恢复、最大长度 JSON 转义与 CSRF 边界另见[服务端测试](../native/server-ui-tests.txt)。

重新验收时，先按 `web/README.md` 构建，再以 `xcs --root ROOT serve --listen 127.0.0.1:PORT --web-dir ABSOLUTE_DIST` 启动服务，通过浏览器完成上表操作。初始化开发池的参数见根项目文档。验收登录信息应放在仓库外的限制权限文件中；不应将运行时凭据写入证据。
