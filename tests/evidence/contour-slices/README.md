# 连续轮廓背景缺带回归

日期：2026-10-18。

根因是紧凑 SVG 九宫格的中心横、纵条只有 1px。比如高度 63px 时，物理角框 extent 被钳到 31.5px，含 2px 外围的切片为 33.5px，源图为 68×68px。Chromium 将 `border-image-slice: 33.5 fill` 的数值取整为 34；左右及上下切片各占 34px，把中心条完全吃掉。表现为横条仅剩两端，纵条仅剩上下两端；端部交界也可能出现细缝。

修复保持相同 SVG、连续轮廓曲线及目标物理角框，仅将源图切片表示为百分比：`33.5 / 68 × 100%`。`border-image-width` 保留 `33.5px`，不会把物理轮廓变成随长宽缩放的圆角。

验证使用运行中页面实际生成的 SVG，在隔离浏览器元素中对比原数值切片和与 Rust 修复一致的百分比切片，不捕获 API 内容或凭据。10 种横纵尺寸（含奇数、偶数、分数、5px 短边），浅深主题，DPR 1、1.25、2，共 60 组修复场景及 60 组原始对照。扫描中心十字线及切片交界的直边区域，排除外缘 2 CSS px 的预期抗锯齿，允许单通道偏差 2。

- 修复场景：448,564 次像素采样，60/60 无缺色。
- 原始对照：24/60 稳定出现缺色。
- 真实浏览器版本、尺寸、采样方法及计数见 [summary.json](summary.json)。
- 每组原始结果见 [matrix-results.json](matrix-results.json)。
- [横向前后对比](before-after-wide.png)；[纵向前后对比](before-after-tall.png)。

该记录验证绘制算法的浏览器像素结果；部署后的应用验证由主流程另行记录。临时探针保存在 当时的临时测试目录或日志，产品中没有新增维护的 JavaScript。

对应 Rust 回归：`contours::paint::tests::fractional_compact_slices_keep_the_centre_strip_in_both_axes`。渲染器绘制相关 9 项单测通过，Web workspace 的 wasm32 检查通过。
