# 页面行格式与最新版本置顶检查

2026-10-19。本轮统一详情、文件操作历史和系统监测下半部分的真实页面行格式：36px 基准行、14.4px 字号、10.8px 内容起点（带 1px 外框的行使用 9.8px 内缩）、18px 内容块间距。允许长路径、备注与窄屏说明在同一逻辑行内自然换行。

版本图采用逆向拓扑：所有终端版本位于顶层，同层最新日期优先，旧父版本位于下方；箭头仍表示旧父版本指向新子版本，视觉向上。跨层、多父和高低不同节点继续保留真实连线及避障。

## 自动化检查

- Web 工作区 66 项单元测试通过（Runtime 26、应用 40），包含最新版本拓扑、向上箭头及现有组件回归。
- [源 CSS 夹具结果](../../../test/evidence/page-format-newest-tree/source-css-result.json)八组通过，覆盖中英文和正常桌面／手机窗口；[夹具脚本](../../../test/evidence/page-format-newest-tree/source-css-probe.mjs)保留供复核。
- `tools/web-build check` 通过，14 份 CSS 的源码与 dist 逐字节一致；契约工具验证 522 个文档链接通过。
- 最终服务 [15 份 CSS/WASM 资源验证](../../../test/evidence/page-format-newest-tree/asset-result.json)通过，包含新增 `history.css`，与最终构建逐字节一致，MIME 与 CSP 正常。

## 浏览器范围

四种组合均已完成，每场 9 组、合计 36 组通过：[汇总](../../../test/evidence/page-format-newest-tree/summary.json)与[完整记录](../../../test/evidence/page-format-newest-tree/result.json)。没有执行浏览器过度缩放测试。

| 语言／主题／宽度 | 通过组数 | 真实版本／连接 | 真实历史事件 | 测试会话注销 |
| --- | ---: | --- | ---: | --- |
| 中文／浅色／1440px | 9 | 3／2 | 55 | 完成 |
| 中文／深色／390px | 9 | 3／2 | 55 | 完成 |
| 英文／浅色／800px | 9 | 3／2 | 55 | 完成 |
| 英文／深色／320px | 9 | 3／2 | 55 | 完成 |

历史页面验收真实 `ul` 及四个事件字段、筛选控件和空态。详情逐行检查元信息、标题、说明、文件备注、分支控件及版本节点，另检查监测底部采样时间与卡片内部行。业务写入全部拦截，只有登录与注销使用真实请求；模拟数据仅用于图布局及长短历史事件展示，不修改用户数据。

## 实际测量结果

- 详情面板清除旧 14px 留白，使用 0 内边距、1px 灰框和 G4 轮廓；标题、元信息、说明、文件备注、分支标签与控件按 36px／14.4px 基准逐行检查，内边距、外边距及真实尺寸共测量 480 项。框内使用 9.8px 内缩，首个元信息文字起点距外框 10.8px。长路径换行时，标签和值的字形组保持垂直居中，最大中心误差 0.3125px；各内容块间隔 18px。
- 真实 API 的 3 个版本、2 条父子连接与页面完全相同，两条终端版本在上、旧父版本在下。明显不同日期的 mock 进一步检查 1 月 6／5／4 日版本位于顶层、1 月 1 日旧父位于底层；多父、跨层边及缺失父版本独立显示均通过。箭头仍从旧父指向新子，端点由父节点顶部指向子节点底部前 3px，视觉向上。12 个树状态中 56 条连接共采样 5656 点，无穿过节点；最大端点误差 0.00037px。
- 文件备注、版本备注与创建分支的原位输入打开及取消保持紧凑格式；129 字节无效备注使行增高后，向上箭头重新测量，仍不穿节点。没有提交任何业务请求。
- 历史真实事件和长短 mock 共测量 232 个事件、928 个字段；每字段以 36px／14.4px 为基准，框内内缩 9.8px，文字垂直居中，窄屏换行间隔 18px，末条分隔线为 0，外框为 1px 灰色 G4。空历史显示独立空态，不留下折叠的灰框。长路径允许自然换行，不截断内容。
- 监测底部采样时间采用 36px／14.4px、10.8px 内缩；英文 320px 自然换为两行，高 57.625px。卡片内部标题和副标题各以 36px 为行基准，卡片上下内边距为 0、左右为 9.8px；9 卡保持 3:2、桌面四列、手机一列及 18px 间隔。
- 32 个页面状态记录 1754 个字号及 536 个实际 G4 轮廓，未发现旧字号；最大轮廓误差 0.00934px（容差 0.02px）。60 次浏览器资源 SHA-256 比对全部匹配当前 dist；无页面异常、CSP 违规、文档横向溢出、黑色 SVG 填充或业务写请求。控制台仅记录 401 未授权响应，原始记录保留在结果中。四个测试会话全部注销，登录既定尺寸保持。

截图：[真实文件详情](../../../test/evidence/page-format-newest-tree/file-details-real-zh-light-1440.png)、[真实历史](../../../test/evidence/page-format-newest-tree/history-real-zh-light-1440.png)、[最新在上树 mock](../../../test/evidence/page-format-newest-tree/file-details-newest-tree-mock-zh-light-1440.png)、[手机长路径历史 mock](../../../test/evidence/page-format-newest-tree/history-mock-en-dark-320.png)、[真实状态](../../../test/evidence/page-format-newest-tree/status-real-en-light-800.png)、[手机底部采样时间](../../../test/evidence/page-format-newest-tree/status-footer-en-dark-320.png)。mock 截图仅证明边界布局，不代表真实用户数据。

可复核脚本：[browser-probe.mjs](../../../test/evidence/page-format-newest-tree/browser-probe.mjs)及[独立 G4 轮廓参考](../../../test/evidence/page-format-newest-tree/g4-reference.mjs)。
