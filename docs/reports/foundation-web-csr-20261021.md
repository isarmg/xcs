# Foundation 后续收窄：Web 绑定、CSR 与实际 API（2026-10-21）

接续 [SQLite 与消费者整合报告](foundation-sqlite-cloud-20261021.md)。第一阶段三个提交以 a04b7815 为基线；其最终提交 46a76842fe8275732971e3893c2025bd888eb953 的 CI [39537193525](https://github.com/isarmg/zxs/actions/runs/39537193525) 已 completed/success，13/13 项成功。此前阶段成果不是本次全部范围，以下继续落实剩余已授权批次。

## 第四批：撤销未支持的 parser 终端追踪表面

winnow/debug 在 a04 基线就引用了未声明的 anstream、terminal_size，独立启用会编译失败。当前产品未选择它，因此撤销这项能力：删除 debug feature、可选 anstyle 边、终端查询/日志实现、公开 combinator::trace 和内部 trace_result/DisplayDebug；没有新增依赖恢复其未用功能，也没有留下空 debug feature。

先迁移真实消费者，再撤销 API。winnow 内 107 处、toml_edit 内 22 处 trace 包装改为原 parser；移除 toml_edit/unstable-debug 到 winnow?/debug 的休眠转发。来源门禁曾准确拒绝该转发遗漏，补齐后恢复通过，没有放松规则。seq/dispatch/unordered_seq 宏同样直接生成原有 parser 闭包。保留 Stream::trace 的真实输入诊断格式化及其测试，它不依赖已移除终端追踪能力。活跃教程和 TOML 示例同步到当前表面，原始上游材料不变。

真实消费者新增 Unicode 赋值解析、TOML 原格式往返、二进制读取、Partial 输入未消费和 Incomplete 分类验证：literal 对字符串与字节切片都保留 Unknown，any 在空输入保留 Size(1)。初稿误把 literal 的未知需求写成 Size(1)，阅读既有实现和文档后纠正测试预期，未改变该解析契约。额外负向构建确认旧 trace 入口、winnow/debug 和 toml_edit/unstable-debug 被拒绝。

来源 verify --metadata、静态门禁、契约/发布工具测试、相关 Clippy、生产 release WASM 构建均通过。当前活动来源仍为 326，根 Cargo 包数仍为 344；foundation 为 10644 文件 / 148678139 字节。源码计数不证明运行时性能改善。浏览器验收随本批结束记录。

第四批真实 Chrome/G4、字体、六视口、owner/计时器/延迟及已释放异步请求生命周期验收全部通过；截图位于云端 `/workspace/zxs-evidence/winnow-browser.png` 和 `winnow-viewports/`。不保留此前已确认不可用的 winnow/debug 表面。


## 第五批：底层绑定生成器只有 Web 与 JavaScript

第四批精确 SHA `b89617ace4b7aa14bdeee9ee2838ff091c37bd4d` 的 CI [39540267977](https://github.com/isarmg/zxs/actions/runs/39540267977) 已 completed/success，13/13 项成功。

此前 `tools/web-bindings` 的命令行虽已只接受 `--target web --no-typescript`，其底层仍能选 Node/CommonJS/ESM、Deno、bundler、no-modules、source-phase module 和 Emscripten，并构建 TypeScript 输出。本批在底层删除 OutputMode、多后端设置 API、对应生成/加载/写盘分支、Emscripten 库依赖图及 JSPI hooks、描述器的 invoke_* 跳板。带 Emscripten 标记的模块在执行描述器之前明确拒绝。CLI 继续要求原有两个参数，直接调用没有后端选择器的 Web 核心。

删除 TypeScript 缓冲、签名/字段/命名空间/初始化声明渲染、引用闭包、Output::ts、typed-array buffer 类型选项、wasm2es6js 模块。该模块是 CLI support 的唯一 base64 使用处，因此同时删除这一真实依赖边；本批没有为凑数合并仍被其他调用方使用的 base64 两代。shared_api 与 SCHEMA_VERSION 未改：解码器仍按原顺序完整消费并校验输入，仅不把声明专用字段存入后续状态；动态 union 仍执行变体描述器、AdapterType 数量和重复名称检查。JSDoc 类型描述与真实 JS getter/setter、字符串枚举、动态 union ABI 保留。

源码按职责整合：Web 导入生成不再经过通用多后端门面；移除仅用于分支的导入命名转发；raw/export 两种引用 helper 统一到相同的 Web 引用实现；只有一项的 FinalizedOutput 包装直接返回 String。浏览器的 WASM 加载与 MIME 回退、文本编解码、内存视图、异常、线程和生命周期分支继续保留。通用 Wasm 指令与 imported global 处理没有因曾被 Emscripten 使用就删除。同步修正前批算术示例的旧 debug 注释。

### 生成结果与真实浏览器证据

使用同一份生产原始 WASM，收窄前后生成的 JS 逐字节一致，SHA-256 为 `0d9a72d43389a2cbfb2e9d7d3349b9c320771171ad702c01fc9c0c84e2214913`。生成 WASM 均为 2,205,316 字节，除 producers 节的 Git 构建身份（a04b7815b 与 b89617ace）之外，所有节逐字节一致，包括代码、数据、导入、导出和名字节。

另建真实 wasm-bindgen 宏消费者，覆盖可选字段、getter/setter、方法、Unicode、数字枚举、字符串枚举、动态 union、自定义 TypeScript 声明。其 JS 同样逐字节一致，SHA-256 为 `c9db88a95779858b73722511743b84a66a779a3155abdfca451614ad0fd171ba`；24,414 字节 WASM 亦只差 producers 构建身份。输出只有 JS/WASM，无声明文件或声明泄漏。

Chrome 154.0.8037.57 保留浏览器沙箱，分别执行正常加载、错误 MIME、缺少 instantiateStreaming/FinalizationRegistry/TextEncoder.encodeInto 三组，每组 10 项运行时断言均成功。证据见 [同输入产物比较](evidence/foundation-web-binding-equivalence-20261021.json) 和 [真实浏览器兼容回退](evidence/foundation-web-binding-browser-20261021.json)。没有声称已经在 Safari 或 Firefox 实机运行；其浏览器兼容分支保留，并经相同输出比较。

### 门禁、数量与边界

新增 foundation CI 的真实消费者检查：构建上述 WASM 夹具并运行当前 CLI；14 组独立负向编译分别确认已删除的后端/TypeScript/输出 API 不可用；CLI 单测确认七种旧后端参数被拒绝、两种 Emscripten 标记在描述器前被拒绝。工具单元/集成测试、相关 Clippy、来源 verify --metadata、架构索引、静态门禁与生产构建已通过。正式产品 Chrome/G4、字体、六视口、会话隔离、owner/计时器/延迟及已释放请求生命周期套件全部通过；截图为云端 `web-bindgen-browser.png` 和 `web-bindgen-viewports/`。全 workspace quality（Clippy、全部主机 target、release WASM 警告门禁）同样通过。

本批 foundation 从 10,644 文件 / 148,678,139 字节降至 10,642 / 148,516,376，减少 2 文件与 161,763 字节；相对 a04 基线减少 367 文件、1,932,826 字节。根 Cargo 包仍为 344、活动来源仍为 326、退休来源仍为 76；仅移除 CLI support 到 base64 的一条边，没有改变其余多版本数字。原始 provenance、许可证、上游包名/版本/来源身份保持不变，当前来源收据与架构索引更新，门禁未放宽。生成器源码缩小不等于浏览器性能收益；同输入产物相同正说明本批目标是维护表面与构建职责收窄。

CSR/流式 HTML 和 web-sys/tachys 实际闭包仍是后续已授权批次，尚未在本批完成。浏览器 template 所需 RenderHtml/hydration 必须先分离保留，不能直接删除 RenderHtml。
