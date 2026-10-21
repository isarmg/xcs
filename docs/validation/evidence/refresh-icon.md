# 刷新按钮只显示图标

2026-10-18。直接删除按钮内的文字span及对应文字间距、移动端隐藏样式，仅保留刷新SVG，按钮通过aria-label和title提供名称。顶部菜单使用现有44px控件尺寸。

已离线构建并更新7400预览，保留原数据与凭据。构建输入逐项匹配当前源文件。真实浏览器结果见[result.json](../../../test/evidence/refresh-icon/result.json)，[构建输出](../../../test/evidence/refresh-icon/web-build.txt)及[构建清单](../../../test/evidence/refresh-icon/build-manifest.json)。不将编译链接等同于六平台运行验收。

浏览器检查期间还定位并约束了文件管理器的自动网格列，避免现有目录在320px下由最小内容宽度撑开页面；表格仍可在容器内滚动，没有隐藏页面溢出。
