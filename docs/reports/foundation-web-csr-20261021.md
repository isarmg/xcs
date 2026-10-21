# Foundation 后续收窄：Web 绑定、CSR 与实际 API（2026-10-21）

接续 [SQLite 与消费者整合报告](foundation-sqlite-cloud-20261021.md)。第一阶段三个提交以 a04b7815 为基线；其最终提交 46a76842fe8275732971e3893c2025bd888eb953 的 CI [39537193525](https://github.com/isarmg/zxs/actions/runs/39537193525) 已 completed/success，13/13 项成功。此前阶段成果不是本次全部范围，以下继续落实剩余已授权批次。

## 第四批：撤销未支持的 parser 终端追踪表面

winnow/debug 在 a04 基线就引用了未声明的 anstream、terminal_size，独立启用会编译失败。当前产品未选择它，因此撤销这项能力：删除 debug feature、可选 anstyle 边、终端查询/日志实现、公开 combinator::trace 和内部 trace_result/DisplayDebug；没有新增依赖恢复其未用功能，也没有留下空 debug feature。

先迁移真实消费者，再撤销 API。winnow 内 107 处、toml_edit 内 22 处 trace 包装改为原 parser；移除 toml_edit/unstable-debug 到 winnow?/debug 的休眠转发。来源门禁曾准确拒绝该转发遗漏，补齐后恢复通过，没有放松规则。seq/dispatch/unordered_seq 宏同样直接生成原有 parser 闭包。保留 Stream::trace 的真实输入诊断格式化及其测试，它不依赖已移除终端追踪能力。活跃教程和 TOML 示例同步到当前表面，原始上游材料不变。

真实消费者新增 Unicode 赋值解析、TOML 原格式往返、二进制读取、Partial 输入未消费和 Incomplete 分类验证：literal 对字符串与字节切片都保留 Unknown，any 在空输入保留 Size(1)。初稿误把 literal 的未知需求写成 Size(1)，阅读既有实现和文档后纠正测试预期，未改变该解析契约。额外负向构建确认旧 trace 入口、winnow/debug 和 toml_edit/unstable-debug 被拒绝。

来源 verify --metadata、静态门禁、契约/发布工具测试、相关 Clippy、生产 release WASM 构建均通过。当前活动来源仍为 326，根 Cargo 包数仍为 344；foundation 为 10644 文件 / 148678139 字节。源码计数不证明运行时性能改善。浏览器验收随本批结束记录。

第四批真实 Chrome/G4、字体、六视口、owner/计时器/延迟及已释放异步请求生命周期验收全部通过；截图位于云端 `/workspace/zxs-evidence/winnow-browser.png` 和 `winnow-viewports/`。不保留此前已确认不可用的 winnow/debug 表面。
