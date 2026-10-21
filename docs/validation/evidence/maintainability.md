# 代码维护性重构验收

> 历史范围说明：2026-10-20 起产品仅保留 Web 与 SMB；本文 App/Tauri 的路径、命令和结果属于移除前记录，不再作为当前构建或验收要求。

本目录保存本轮全项目审查、修复和验证记录。逻辑、模块职责及具体问题见 [代码审查报告](../../reports/code-review-20261019.md)。

后续间距规则已纠正，最新记录见[装饰框间距修正](spacing-correction.md)。本目录的截图、结果和源码摘要保留的是修正前候选，旧夹具中“所有内容栈／换行均固定增加半行”的断言不再代表当前要求；当时的浏览器脚本与结果属于历史候选；当前验收及安全重跑配置见[证据索引](../evidence.md)。

## 全项目范围与验证

[source-inventory.json](../../../test/evidence/maintainability/source-inventory.json) 记录 250 个活动源码、清单、配置与模块文档的 SHA-256，其中 168 个 Rust 文件、59,127 行；另记录本地原生二进制、生产 WASM 与构建清单身份。上游依赖源码、旧验收记录、运行目录与用户数据不计入该范围，生成 Unicode 表由复现工具验证。清单说明本轮阅读和验证边界，不是外部依赖全部安全审计的证明。

| 范围 | 已执行结果 | 记录 |
| --- | --- | --- |
| 根原生 workspace | 219 项测试通过，含核心、认证、平台、服务、客户端、CLI 及 HTTP/监测集成 | [原始输出](../../../test/evidence/maintainability/native-workspace.log) |
| Web workspace | 74 项测试通过：runtime 26、app 43、集成 5 | [原始输出](../../../test/evidence/maintainability/web-workspace.log) |
| App bridge | 11 项测试通过，包含真实本机 HTTP 会话隔离及后台导入取消 | [原始输出](../../../test/evidence/maintainability/app-bridge.log)、[审读记录](maintainability/review/native.md) |
| Linux Tauri 宿主 | 锁定离线检查通过，使用本机已有 GTK/WebKit SDK | [环境与限制](maintainability/review/native.md) |
| 可移植 server | 无默认功能检查及 39 项库测试通过 | [检查输出](../../../test/evidence/maintainability/server-no-core-check.log)、[测试输出](../../../test/evidence/maintainability/server-no-core-tests.log) |
| 自维护 CSR/宏 | 4 项库测试通过；另有 2 项实际 WASM 计时生命周期回归 | [库测试输出](../../../test/evidence/maintainability/vendor-component-tests.log)、[WASM 结果](../../../test/evidence/maintainability/animated-show-result.json) |
| SMB 协议 / VFS SDK | 6 项协议测试、5 项实际 Samba 4.23 SDK 库测试通过 | [命令、环境与工具回执摘要](maintainability/review/tools-smb.md) |
| 构建及契约工具 | web-build 2 项、contract-check 6 项通过，固定 Unicode 输出复现通过 | [命令与工具回执摘要](maintainability/review/tools-smb.md) |
| Clippy | 根 workspace/all-targets 退出 0；41 项非阻断建议，未用 allow 隐藏 | [原始输出](../../../test/evidence/maintainability/clippy.log)、[分类与处理依据](maintainability/review/clippy.md) |
| 最终浏览器与实际资源 | 四场 60 组通过；17 项 CSS/WASM 响应匹配最终构建 | 下方专项、[资源结果](../../../test/evidence/maintainability/asset-result.json) |

根 workspace 的复现命令为 `cargo test --locked --offline --workspace`；Web 和 App 分别增加 `--manifest-path web/Cargo.toml --workspace`、`--manifest-path apps/Cargo.toml --workspace`。各独立 SDK 和工具命令保存在对应审读记录。部分临时 FIFO、所有权和本机套接字夹具需要主机系统调用权限；沙箱拒绝的首次结果没有被算作通过，获授权后相同检查的主机运行才是上述结果。

可移植 server、CLI 单独检查和其他重叠子集不重复累计为新增覆盖。SMB/SDK/两个工具及 Unicode 本轮输出当时仅保留在工具回执和审读摘要，没有冒用旧 SDK 测试文件作为本轮原始日志。整体读取与修复记录分为 [核心](maintainability/review/core.md)、[服务与平台](maintainability/review/server-platform.md)、[CLI 与 CSR](maintainability/review/cli-vendor.md)、[客户端与 App](maintainability/review/native.md)、[SMB 与工具](maintainability/review/tools-smb.md)、[Web](maintainability/review/web.md)。

本轮编译及本机验证不表示物理盘故障、完整 SMB 客户端矩阵或六平台签名包/实机验收。既有存储和缓存未隐式迁移，未清理用户文件；没有提交、推送或远程发布。

## 已完成的源码验证

- Web workspace：74 项 Rust 测试通过，包括 26 项 runtime、43 项 app 和 5 项集成测试；无编译警告。
- 详情／历史／状态源码 CSS：中英文 × 1440、800、390、320px，共 8 组通过。
- 账户源码 CSS：同样 8 组通过，覆盖 36px 控件、14.4px 字体、G4 E18、504px 宽度上限、桌面两列、手机分组、18px 间距、readonly/autocomplete/pending 和长错误。
- 共享令牌动态变更：4 个正常宽度，把 36→40px、44→50px 后，各页行高、字号、边框内缩、G4 及间距自动计算变化，登录几何和字号保持相同。

源码夹具是独立 HTML 的样式验证，不代替下述最终候选的真实 WASM 验收。

`source-table-alignment-probe.mjs` 另将真实浏览器脚本中的 `fileRows` 检查原样用于本地静态 DOM，对磁盘／文件表 × 36／40px 各做一次正、负对照：[8 项均通过](../../../test/evidence/maintainability/source-table-alignment-result.json)。4 个负对照保留浏览器默认的 th 居中，均被新增 computed text-align 断言拒绝；4 个正对照在 cell 层继承表格对齐，首字 Range 起点符合左内缩，名称列计入前置图标和图标间距。夹具不访问业务服务、不登录，也不修改生产 CSS。

## AnimatedShow 独立 WASM 验证

`animated-show-probe.mjs` 与 `animated-show-result.json` 记录两项已通过的组件生命周期回归：隐藏后再次通知 false，在第 102ms 重新显示，第 352ms 子内容仍存在；隐藏并重复通知后解除挂载，等待至第 305ms 容器仍为空，无异步 panic、pageerror 或 console error。实际加载的 fixture WASM 为 `application/wasm`，SHA-256 记录于结果中。

该夹具直接编译当前项目维护的组件，在独立临时本地 HTTP 运行，结束后关闭；没有新增产品路由，也不访问现有业务服务。当前业务页面没有使用 AnimatedShow，因此这是公共组件回归，不能计作业务页面覆盖。

## 最终候选的真实浏览器验证

统一构建并切换至最终 native 二进制及 WASM 后，`browser-probe.mjs` 已在 `http://127.0.0.1:7400` 完成全部四场验证，**60／60 组通过**。完整结果见 [result.json](../../../test/evidence/maintainability/result.json)，汇总见 [summary.json](../../../test/evidence/maintainability/summary.json)；分场结果、DOM／曲线清单及截图均保存在本目录。首轮目视发现表头受浏览器 th 默认居中影响，修正 cell 的对齐继承并增加字形守卫后，最终候选已完整重跑四场；本目录的实际页面结果及截图全部来自这次重跑，没有复用首轮结果。

四场组合为中文／浅色 1440px、中文／深色 390px、英文／浅色 800px、英文／深色 320px。每场 15 组检查，覆盖：

1. 登录既有字号、物理轮廓和内容边界。
2. 磁盘列表、文件列表、工具栏、文件夹／文件详情、历史、状态、账户的共享 row/stack/flow/frame 角色完整性；默认 36px／.4 字号比例／18px 间距及 G4。
3. 普通输入和顶部图标使用 44px 基准；账户／搜索／下拉框显式 compact，原位输入显式 underline。
4. 版本树最新叶在上、旧父在下，真实父子关系、向上箭头、跨层避让、三行节点、超限输入／IME／Esc／失焦取消。
5. 历史长字段折行、空历史无残留灰框、无通知／错误时主栈不出现空内容槽。
6. 状态卡四列／手机一列、3:2 比例、图表尺寸与采样时间行、块间距和折行间距。
7. 账户本地校验、只读用户名、autocomplete、请求期间清密／readonly／禁用以及模拟远端错误。
8. 在浏览器路由中给候选 `geometry.css` 追加测试令牌 40px／50px，重新加载同一真实 WASM，依次检查各页与 G4，随后验证登录仍与基线相同；不修改生产源码或服务资源。
9. 挂起旧会话 GET，真实退出／重新登录后才释放模拟 401，再用真实退出确认新 TOKEN／CSRF 仍有效。
10. 动态枚举候选目录中的完整 CSS 和 WASM 请求集合，逐资源核对实际响应／候选字节 SHA-256、MIME、CSP、实际 Rust G4 曲线与运行标记，不固定资源数量。

表格验收同时检查所有 th／td 的 computed text-align 为 start／left，并使用首字 Range 验证字形起点与 cell 左侧内缩相符；名称列仅增加实际图标宽度和按钮 gap。默认与 40px 测试令牌下，磁盘表和文件表均执行此检查，避免仅继承 table 样式却被 th 的浏览器默认值覆盖。四场共检查 16 个表格状态、320 个单元格，其中 64 个表头单元格；最大字形起点误差 0.015625px，小于 0.5px 容差。

每场动态资源清单均为 16 个 CSS 和 1 个 WASM，17 个基线资源的响应字节与候选文件一致、MIME 正确；令牌变更阶段的测试 CSS 响应单独标记，不混作原始候选字节。四场均无 pageerror、CSP 错误或意外 console error。并目视检查桌面账户和版本树、手机账户和版本树、手机与 800px 状态页，未发现重叠、全黑图表或越过页面边界；宽树在自身容器内横向浏览。

安全边界：只允许真实登录和退出两个写接口。四场共 8 次真实登录及 8 次真实退出，退出均为 HTTP 200，所有会话已撤销；4 次改密请求只使用 dummy 文本，均由浏览器本地返回模拟错误。其他业务写请求零放行，真实密码和用户文件没有被修改。截图和清单不记录输入框内容、登录响应、令牌或 Cookie。

```sh
node --check test/evidence/maintainability/browser-probe.mjs
ZXS_PLAYWRIGHT_MODULE=/path/to/playwright/index.mjs node test/evidence/maintainability/browser-probe.mjs
```

项目根默认由脚本相对位置解析，也可用 `ZXS_PRODUCT_ROOT` 指定；服务默认 `http://127.0.0.1:7400`，`ZXS_PREVIEW_URL` 可指定本地候选地址。演示记录默认 `/tmp/zxs-demo-seed/result.json`，可用 `ZXS_BROWSER_SEED` 指定，前置是已有只读可访问的演示目录和文件；脚本不会生成或修改演示文件。`ZXS_MAINT_SCENES` 可指定需要复测的场次。Playwright 默认按已安装包 `playwright` 导入，或通过 `ZXS_PLAYWRIGHT_MODULE` 指定工具模块；不是产品构建或运行依赖。

历史执行通过该变量指定已安装的工具模块；以下为可移植路径占位示例：`ZXS_PLAYWRIGHT_MODULE=/path/to/node_modules/playwright/index.mjs`，其余路径与服务使用上述默认值。脚本本身不依赖该工作区位置。每次运行会将 `result.json`／`summary.json` 写为本次选择的场次；分批执行时须按对应分场结果合并，不能把单批汇总当成完整四场。

同目录的三套源码夹具也采用项目根推断及 `ZXS_PRODUCT_ROOT`／`ZXS_PLAYWRIGHT_MODULE` 参数。归档的 20 场通过结果来自相同 DOM／CSS 逻辑，之后仅参数化路径，没有因此声明额外运行次数。

不覆盖浏览器过大缩放，也没有为此添加产品补偿逻辑。浏览器验收不表示六平台 App 实机验收或物理盘故障验收已经完成。
