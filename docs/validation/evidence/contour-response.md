# 连续轮廓缺色与监控响应修复

2026-10-18。用户截图中的横向中间背景缺失和纵向细带来自相同的连续轮廓切片问题；红字 API 错误是另一处前端响应解码缺陷。

## 原因与修复

紧凑 SVG 的两边轮廓之间只保留 1px 中心填充条。组件尺寸为奇数时，切片产生半像素值；例如 source width=68、slice=33.5，Chromium 将数值源切片取整为34，两侧合计68，中心条被完全吃掉。修复使用百分比保留源切片精度，目的边框宽仍使用原 CSS 像素，保留连续曲线和物理角框尺寸。横纵、整数/小数尺寸、两种主题与DPR矩阵见[轮廓像素记录](contour-slices.md)。

CPU 使用率响应是合法有限小数（真实复现 20.600000381469727、3.5），但 Web 原先对所有响应使用禁止浮点的存储契约解析器。后台返回200 application/json，界面却显示CPU不可用及INVALID_RESPONSE；错误包装还把实际200记录为0。新 `web/src/api/response.rs` 专门解析响应中的有限数值，继续递归拒绝重复键、BOM、异常数值、孤立代理项和尾随垃圾，原始响应正文不会进入错误信息。磁盘、写请求、整数 JCS 校验保持原限制；错误保留真实 HTTP 状态。

没有证据表明该报错由200 HTML引起：实时检查监控、磁盘、历史接口均为JSON，未知API是404。具体复现见[修复前结果](../../../test/evidence/contour-response/before-result.json)和[原错误条](../../../test/evidence/contour-response/before-error.png)。

旧文件管理浏览器脚本只检查网络层采样，没有检查实际页面CPU值与错误提示，因而遗漏此问题；旧记录已补充更正。新版验收要求实际页面显示百分比且 `role=alert` 为0。


## 验证

完整 Web workspace 48 项测试通过：运行环境 25、组件属性 3、响应式生命周期 2、应用 18。新增回归覆盖有限小数响应、初始空 CPU 值、重复键、异常 JSON、实际 HTTP 状态以及分数切片中心条。

真实服务修复前后对照确认：修复前网络返回合法 CPU 小数，但页面显示“不可用”并出现错误条；修复后页面显示实际百分比，错误提示为0。完整文件/目录分支业务流程重新通过91次API请求，并增加实际CPU页面值和无alert断言。

正式 WASM 绘制器在横向1728×63/65和纵向63/65×850、明暗主题下，读取到真实百分比切片（49.2647%/49.2857%），逐像素扫描中心十字，5380次采样无缺色；独立多DPR与分数尺寸矩阵另见前述轮廓记录。

[测试输出](../../../test/evidence/contour-response/web-tests.txt)、[业务回归](../../../test/evidence/contour-response/file-regression.txt)、[业务结果](../../../test/evidence/contour-response/file-regression-result.json)、[修复后CPU页面](../../../test/evidence/contour-response/after-result.json)、[正式渲染器像素结果](../../../test/evidence/contour-response/actual-contours.json)、[修复前页面](../../../test/evidence/contour-response/before-status.png)、[修复后页面](../../../test/evidence/contour-response/after-status.png)。


## 菜单页面标题

此节记录的是已被替代的隐藏方案；当前直接删除四页标题节点的实现及验收见[删除菜单页面标题](heading-removal.md)。

文件、状态、历史记录及账户页面不再显示大字页面标题。保留无障碍语义标题，三页刷新按钮行高44px并靠右；文件详情和指标的内容标题不变。真实 Chromium 验证了隐藏标题的1×1裁切、右对齐和1440/390/320px布局，账户外层标题也已覆盖。

[页面检查](../../../test/evidence/contour-response/menu-headings-result.json)、[文件页](../../../test/evidence/contour-response/menu-files-1440.png)、[390px文件页](../../../test/evidence/contour-response/menu-files-390.png)、[状态页](../../../test/evidence/contour-response/menu-status-1440.png)、[历史页](../../../test/evidence/contour-response/menu-history-1440.png)、[账户页](../../../test/evidence/contour-response/menu-account.png)。

最终运行的7400预览保留原有数据池与凭据，22份资源匹配当前清单，WASM MIME/magic、gzip、ETag/304、CSP与nosniff通过，源文件逐项匹配构建清单。Linux宿主重新嵌入并完成链接，未运行桌面或验收六平台发行包。

[最终构建](../../../test/evidence/contour-response/web-build.txt)、[最终资源身份](../../../test/evidence/contour-response/asset-result.json)、[响应头](../../../test/evidence/contour-response/wasm-response-headers.txt)、[构建清单](../../../test/evidence/contour-response/build-manifest.json)、[源文件核对](../../../test/evidence/contour-response/source-inputs-result.json)、[Linux宿主链接](../../../test/evidence/contour-response/linux-host-build.txt)。没有记录密码、Cookie或令牌。测试使用隔离开发目录池，不构成物理盘验收。
