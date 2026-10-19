# XCS 浏览器基础层

本 crate 提供 CSR 挂载、主题与语言选择、共享组件和 CSS 驱动的轮廓控制器。业务请求、身份验证及会话状态由应用通过 props 和回调传入组件。

`mount(root_id, app) -> Result<(), JsValue>` 同步检查根节点并安排异步启动。`i18n::initialize` 与 `theme::initialize` 先应用语言和主题；随后加载 `XcsUI` 常规、粗体两个字重并等待字体集合就绪，再挂载应用、启动轮廓控制器，最后发布 `xcs-startup-ready`。同步错误返回调用者，异步错误发布 `xcs-startup-failed`。

启动状态为 `data-xcs-startup="pending|ready|failed"`。字体等待最多 30 秒，收到失败事件或离开页面即取消；失败后迟到的加载结果不能挂载应用。构建工具生成的资源引导器也有 30 秒上限，负责在 WASM 无法加载时用系统字体显示错误页和重新加载按钮。它在动态导入绑定之前安装失败监听器，成功或失败后移除这些临时监听器。`styles/startup.css` 单独提前加载，因此失败提示不依赖应用字体。

应用保留 CSR 卸载句柄和轮廓控制器。非 bfcache 的 `pagehide` 会释放 owner、主题监听器、轮廓观察器、动画帧及 Blob URL；进入 bfcache 时保留页面状态。几何与绘制规则及启动状态转换具有纯 Rust 回归；实际字体资源、浏览器事件和 CSP 仍须通过浏览器验收。

轮廓内部名为 `xcs-g4`。`contours/profile.rs` 按[《给矩形倒圆角 (2)》的 G4 公式](https://zhuanlan.zhihu.com/p/713026877)构造曲率：`κ(u)=Cf(6u⁵−15u⁴+10u³)`，其中 `u=s/L`，再对切向积分求坐标；完整一角由缓和段、中央圆弧及镜像缓和段组成。本项目选取圆弧转角比例 `p=0.4`，归一化最大曲率 `K=1.486024529088757`，物理圆角范围为 `E` 时半径为 `E/K`。这是原文方案的项目参数，不是原文给出的 Apple 默认参数。

坐标使用 512 格 Simpson 积分表与残余区间积分，截断误差单独记录于 `coordinateIntegrationErrorBound`。曲率及其前两阶弧长导数在直线和圆弧接点连续；采样采用平行曲线二阶导数上界控制弦误差，实际 SVG 仍是误差受限的折线。浏览器使用 0.02 CSS px 总预算；四角变换、边框、阴影、裁切及 CSP 绘制沿用统一控制器。页面比例和字体规则见[样式说明](../public/styles/README.md)。

公共接口：

- `i18n::{t, tf, locale, valid_locale, switch_language}`：支持中文与英文，语言切换使用当前 URL 的 `lang` 参数；有效参数写入 `xcs-language`。
- `theme::{current, toggle}`：主题属性为 `data-xcs-theme`，偏好键为 `xcs-theme`；无手动选择时跟随系统主题。
- `contours::{start, ContourController}`：浏览器控制器；纯几何规则位于 `contours::geometry`。
- `components::*`：壳、登录、账户、导航、对话框和反馈组件，详见 [组件契约](src/components/README.md)。
