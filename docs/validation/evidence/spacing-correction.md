# 装饰框间距修正验证

本轮验证对应 2026-10-19 的间距纠正：无框文字行依靠原有行高和内边距；仅直接相邻、没有自然内容分隔的可见装饰框补足半行距离。之前 `maintainability/` 中断言所有 `stack/flow` 都增加半行的记录属于旧候选，不能作为本轮间距验收依据。

[source-spacing-probe.mjs](../../../test/evidence/spacing-correction/source-spacing-probe.mjs) 使用当前源码 CSS 与同结构的静态 DOM，在 Chromium 中验证中英文各 1440、800、390、320px，共 8 个场景。结果与实际读取文件的 SHA-256 位于 [source-spacing-result.json](../../../test/evidence/spacing-correction/source-spacing-result.json)。这是源码 CSS 布局验证，不是对已部署应用的业务验收；没有登录真实服务，没有读取或修改用户文件。浏览器在受限沙箱中无法启动，本轮通过已获批的本机 Chromium 执行。

验证内容：

- 面包屑、文件操作行、目录分支标签行之间不再固定增加外部间距；搜索框与下拉框之间已有标签行，不叠加半行。
- 桌面目录下拉框直接接列表边框时净距为 18px；手机说明文字独占一行后不再增加 18px。插入只读提示行时不再补外部间距。
- 状态范围控件与首行卡片、状态卡片之间保留 18px；卡片为 3:2，桌面最多四个等宽位置，手机每行一个，最后不足四个仍等宽。
- 详情普通内容、历史文字换行不再固定增加外部间距；版本节点水平、垂直间距保留 18px。
- 账户桌面输入框之间为 18px；手机已有标签行分隔，字段组不再补距；保存按钮行前没有额外间距。
- 登录 CSS SHA-256 与修改前一致；独立登录 DOM 在本轮公共间距与旧间距对照下的位置、尺寸、字体和轮廓参数一致。
- 旧规则负对照恢复 `stack/flow` 的固定半行间距，实际重新出现 18px 无框行额外间隔，并被本轮零额外间距断言拒绝。

典型截图：[桌面文件页](../../../test/evidence/spacing-correction/files-zh-1440.png)、[手机文件页](../../../test/evidence/spacing-correction/files-zh-390.png)、[桌面状态页](../../../test/evidence/spacing-correction/status-zh-1440.png)、[手机账户页](../../../test/evidence/spacing-correction/account-zh-390.png)。所有场景均保留相应截图。

执行方式（需要已安装 Chromium 的 Playwright；依赖路径可按本机配置）：

```sh
ZXS_PLAYWRIGHT_MODULE=/path/to/node_modules/playwright/index.mjs node test/evidence/spacing-correction/source-spacing-probe.mjs
```

未增加浏览器大幅缩放适配逻辑；上述均为正常视口尺寸验证。静态夹具不运行 Rust/WASM 轮廓绘制器，不能替代以下真实应用检查。

## 最终候选的真实页面验证

Web workspace 本轮 74 项测试通过，格式检查和生产构建通过；命令及源码、WASM、构建清单身份见[candidate-receipt.json](../../../test/evidence/spacing-correction/candidate-receipt.json)。74 项来自本轮工具执行回执，没有另存原始测试日志。原生后端本轮未改动。

更新本地预览后，使用实际 Rust/WASM 在中文浅色 1440px、中文深色 390px、英文浅色 800px、英文深色 320px 四场完整重跑，**60／60 组通过**：[汇总](../../../test/evidence/spacing-correction/browser/summary.json)、[完整结果](../../../test/evidence/spacing-correction/browser/result.json)。测试不是复用上轮结果；实际 CSS/WASM 响应 17 项均匹配本轮候选，见[资源摘要](../../../test/evidence/spacing-correction/asset-result.json)。[文档契约检查](../../../test/evidence/spacing-correction/contract-result.json)也通过。

- 桌面文件页的面包屑→操作行、操作行→目录标签均为 0 额外间隔，下拉框→列表 18px；改成 40px 行高后仅表面分隔变为 20px。
- 手机说明独占一行，已经形成约 58～79px 的可见框间自然距离，因此文件页不再补外部间距。
- 状态范围框→卡片为 18px，卡片→无框采样时间行不再补距；账户表单 gap 为 0，桌面输入框之间 18px，手机字段组额外间距为 0。状态卡与版本树间距、动态尺寸传播及既定登录布局均通过。
- 320 个表格单元格的字形起点对齐通过；真实版本树、原位输入取消、账户反馈和旧 401 会话隔离回归仍通过，无 pageerror、意外 console error 或 CSP 错误。

典型真实截图：[桌面文件页](../../../test/evidence/spacing-correction/browser/files-zh-light-1440.png)、[手机文件页](../../../test/evidence/spacing-correction/browser/files-zh-dark-390.png)、[手机状态页](../../../test/evidence/spacing-correction/browser/status-zh-dark-390.png)。四场实际登录与退出各 8 次，退出全部 HTTP 200，测试会话已撤销；4 次 dummy 改密请求及其他业务写请求全部拦截，没有修改真实密码或用户文件。

活动浏览器脚本仍位于 `../maintainability/browser-probe.mjs`，已纠正固定间距断言。复现本轮独立输出目录：

```sh
ZXS_PLAYWRIGHT_MODULE=/path/to/playwright/index.mjs \
ZXS_MAINT_EVIDENCE="$PWD/test/evidence/spacing-correction/browser" \
node test/evidence/maintainability/browser-probe.mjs
```

服务、演示只读数据及认证前置条件沿用原脚本；脚本不生成或修改演示文件。`ZXS_MAINT_EVIDENCE` 避免覆盖历史候选的结果。
