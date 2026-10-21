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


## 第六批：分离浏览器模板与服务端 HTML

第五批的最终补齐提交 `1f67340ce7634acce42e93a3b13fae9475e6de15` 已通过 CI [39544312360](https://github.com/isarmg/zxs/actions/runs/39544312360)，13/13 项成功。前批同输入等价 JSON 是提交前源码验证，不能单独冒充最终提交二进制证明；本批新增 CI 资产到精确 Git SHA/树、当前来源收据和实际 WASM 摘要的对应记录。

`RenderHtml` 保留为浏览器模板克隆的 hydration 契约，`ToTemplate` 继续生成模板，`ViewTemplate` 继续克隆节点、恢复动态数据与事件，并保留可重建状态。删除 RenderHtml/Attribute/AttributeValue/Class/Style/Property 等路径上的服务端字符串输出、长度估算、异步服务端解析、HTML 流和相关类型擦除回调；删除 StreamBuilder 与同步/异步/顺序/乱序输出 API。模板恢复不再带 FROM_SERVER 参数，也不再提供服务器异步恢复模式。

同步移除 islands、分支标记、仅为 islands 使用的 SerializableKey 及无业务约束的序列化 key 空实现；keyed 的 items 不再为 SSR 存 Option，保持 key 比较、保留/逐出和实际 DOM/owner 逻辑。事件、directive、property 的值直接放在 SendWrapper 中，删除仅为服务器跳过浏览器值而存在的 Option 和 feature 冲突诊断。界面 View/attribute interceptor/children 约束及宏调用方均已迁移，移除仅用于服务端输出的 view marker。46 个内部宏调用不再传无用的 dry_resolve 标志。

撤销 `ssr`、`hydrate`、`islands`、`mark_branches`、`testing`、`sledgehammer`、`reactive_stores`、`error-hook` 八个特性，以及原先只激活闲置 testing 的 default 声明。后四项分别关联已被注释掉的 mock renderer、未公开的旧 renderer、引用未声明依赖的休眠模块，或没有任何实现分支的空声明，不保留伪支持。删除对应四个源码文件及 html-escape、serde、serde_json、slotmap 四条 manifest 边；这些库仍有其他消费者，未机械删除包。async-trait 还有 JSON Schema/referencing 的异步 API 声明。本批保留它以及 tachys 原有的依赖声明：后者暂时维持当前受支持闭包的来源可达性，仍是待迁移消费者后一并消除的旧边；没有增加虚假入口或弱化 Schema 校验来凑包数。

### 浏览器模板保留过程中发现的旧缺陷

新增真实模板夹具先暴露 void 元素错误：原先 input 的模板生成连同 `()` 子占位和闭合标签一起输出，浏览器把占位解析成按钮之前的兄弟节点，游标无法恢复按钮。本云端另建精确 `1f67340c` 工作树执行同一夹具，复现相同失败。修复运行时模板与常量模板的 void 规则；原生回归同时检查两者，不删掉该场景。

扩展 class/style 断言后发现旧样式恢复沿用“服务端已写入”的假设，而浏览器模板不包含动态 style。将 14 处重复恢复实现统一到已有 build 路径，恢复实际样式。输入 value 刻意通过微任务写入的原行为保留，夹具等待微任务后验证，未为了同步断言改调度。

真实浏览器模板现覆盖 13 项：class 属性及切换、style、Unicode 动态文本、数字/字符/布尔文本、文本转义、微任务 property、事件、重建、节点与焦点保留、监听替换、卸载与清理。现有 web-app/web-interface/view-macros 全部宿主单元、集成与 doctest 保持通过。For 的 to_html 示例在本轮前已整段注释，不属于通过删除活跃测试逃避错误；旧 SSR 专用断言随其明确退役的 API 撤销，保留的文本/转义/class/style/生命周期行为改由模板原生及真实 DOM 回归验证。

### 持续门禁与量化

新增生产套件的 CSR scope 检查，构建真实宏/模板消费者与 WASM，原生测试 void 模板，并以 15 组负向 API 和 8 组负向 feature 检查阻止旧表面返回。CI 生产任务保存独立夹具，原有 Chrome 任务下载同次运行资产并执行真实模板恢复；`validation.json` 记录 Git HEAD/树、工作区是否有修改、当前来源收据、根锁、夹具源码和 JS/WASM 摘要，`browser-result.json` 记录实际浏览器与断言。任务总数仍为 13，没有新增服务或产品部署。

本批 foundation 从 10,642 文件 / 148,516,376 字节降至 10,638 / 148,264,661，减少 4 文件与 251,715 字节；相对 a04 共减少 371 文件、2,184,541 字节。根 Cargo 包仍为 344、活动来源仍为 326、退休身份仍为 76，多版本统计不变。来源收据、合并来源、架构索引和界面说明已同步，原始 provenance/许可证/上游身份及来源门禁保持不变。

全 workspace quality、来源/静态门禁与 Web 宿主测试已通过。最终生产构建、产品 Chrome/G4/字体/六视口/生命周期及同构建的 13 项模板断言全部通过；本地证据见 [CSR 验证收据](evidence/foundation-csr-local-validation-20261021.json)。产品 WASM 从 2,205,316 到 2,191,592 字节，减少 13,724；当前 gzip 为 659,482 字节。不据此推断运行时延迟、内存或吞吐收益。保留客户端 Suspend、owner 与 Abortable 清理，不把服务端解析清理扩展成取消客户端异步 UI。


## 第七批：模板布尔属性与未使用 nightly 表面

第六批精确 SHA `9e54eca136b03baa6264c0f9efa5406b5811c274` 的 CI [39547939294](https://github.com/isarmg/zxs/actions/runs/39547939294) 已 completed/success，13/13 项成功，包括同次生产资产的 13 项模板真实浏览器检查。

独立复核发现既有 bool hydration 缺陷：false 的 disabled 等布尔属性也会被无条件写入，而状态记录 false 使 false→false 重建不能纠正。该行为在旧 1f67340c 已存在，不是 CSR 收窄引入。hydrate 现复用正常 build；新增真实 input checkbox 模板，逐项检查 disabled、checked、required 初始 false，以及 false→false、false→true、true→false。原 13 项模板断言全部保留，扩展为 17 组，未为过门禁删除场景。

本项目未使用 tachys/nightly；其 Static 文本/属性还沿用未转义输出假设。本批诚实撤销该特性、Static 类型和分支、构建通道探测及 tachys→rustc_version 边，不新增 nightly 支持或留下空特性。rustc_version 仍有 reactive_graph 的真实构建消费者，故保留该来源。来源/架构/静态、全 quality、Web 单元/集成/doctest、生产构建及完整 Chrome/G4/字体/六视口/生命周期全部通过；模板检查扩展为 17 组负向 API、9 组负向特性。

foundation 为 10,636 文件 / 148,253,403 字节，本批减少 2 文件 / 11,258 字节；根包仍为 344、来源仍为 326、退役身份仍为 76。此构建产品 WASM 为 2,192,963 字节，较上批增加 1,371 字节，未把源码减少等同运行时或产物必然改善。17 组真实模板结果和验证源码身份见 [本地收据](evidence/foundation-template-bool-local-20261021.json)；最终 CI 在精确提交上另行生成干净树身份与资产映射。


## 第八批：按实际浏览器接口闭包裁剪

第七批精确 SHA `0de18785ef56f845096d76419edc346e33874ad7` 的 CI [39550891109](https://github.com/isarmg/zxs/actions/runs/39550891109) 已 completed/success，13/13 项成功。本批实现时并行观察 CI，没有等待它阻塞独立工作。

本批以稳定 Rust 对实际 Web/WASM 调用方生成的 MIR、维护源码/文档和浏览器夹具为依据，将支持范围写入 `tools/rust-sources/web-scope.json`，记录推导输入 SHA-256。typed HTML 从原表及特殊构造器中保留 42 个，typed SVG 保留 17 个，预定义属性键保留 55 个，事件描述符保留 10 个；class/style/property、自定义属性与事件、目标类型、捕获和监听清理继续沿用真实实现。移除未使用 MathML 公开模块与界面转发、SVG use 特殊构造器、字符串内联事件属性 trait。宏不再输出已退役的 MathML 路径，而是给出明确诊断；保留 inert HTML/SVG、自定义元素及 SVG 命名空间处理。该清单是维护的类型化接口范围，不是浏览器标记白名单，也不宣称是逐机器指令的最小集合。

收窄 tachys 的 web-sys 直接特性从 116 个到 49 个，再遍历所有当前登记 manifest，包括可选、休眠、目标条件与转发特性，并展开父类边，得到完整 web-sys 保守闭包：1,727 → 99 个特性。物理删除对应 1,628 个生成绑定文件，保留类中引用退役特性的签名和条件实现同时删除；没有保留同名空特性。保留 DOM/template、网络/HTTPS、字体、几何、存储、生命周期、ShadowRoot 及真实浏览器回退所需接口，未更改产品 G4/字体/布局算法。当前来源收据、合并来源收据、架构索引与组件说明同步更新，原始身份、provenance 和法律材料未改。

现有 Web 宿主测试仍为 236 项（含 31 项 doctest）；原生和 WASM 编译、完整 quality、静态及来源/metadata 门禁通过。原 17 组真实浏览器模板断言全部保留，新增自定义元素/Unicode 属性、自定义事件与清理、inert SVG 命名空间 3 组；负向检查扩展为 28 组 API 和 13 组特性，并核对 web-sys 策略/manifest/生成文件闭包一致，防止恢复伪接口。首次扩展夹具误用未重导出的 custom_attribute 路径，修正为既有 custom 模块后，实际原生/WASM 消费者通过，未更改生产接口来迎合测试。

本批 foundation 为 9,007 文件 / 138,493,617 字节，较上批减少 1,629 文件 / 9,759,786 字节；相对 a04 减少 2,002 文件 / 11,955,585 字节。根锁仍为 344 包，活动来源 326，退休身份 76。相对基线 353 包 / 335 来源 / 67 退休，净去掉 9 个包，原始来源身份继续可追溯。多版本名称从 12 组到 9 组，额外版本实例从 13 到 10：本轮实际统一 sha2、itertools、shlex，未把移动目录计作去重。

仍保留的 9 组多版本：base64、hashbrown、getrandom、prettyplease、syn、block-buffer、crypto-common、digest、cpufeatures。密码链 digest 0.10/Argon2/blake2 与较新的 SHA-256 链有真实 API 边界；syn/prettyplease 需要 AST 代际配套迁移；getrandom 0.2 的 rand_core 错误契约与后代不同；base64 的 SIMD 默认策略及 hashbrown/hashlink 的 HashTable、顺序和析构需要单独调用方迁移实证。本轮没有仅改版本号，也未为了包数把并发或密码算法拼成巨型 crate。JSON Schema 子集仍只是候选，契约校验未弱化。

最终产品生产 WASM 为 2,192,568 字节、gzip 659,633 字节；相对上批 WASM 变化 -395 字节。完整 Chrome/G4/字体/六视口/生命周期及 20 组模板/自定义 DOM 断言全部通过。见 [本地验证收据](evidence/foundation-web-api-local-20261021.json)。该记录明确为提交前脏树验证，最终提交的精确 SHA/树与资产对应由该提交 CI 另行记录，不以源码减少推断运行时性能收益。

第八批初次 CI 39552954545 的严格来源校验发现，夹具最后一次 custom_attribute 路径修正后未刷新 tools/web-bindings 的逐文件摘要；运行时和本地夹具已通过，但该提交的 CI 不作成功记录。随后补齐来源收据，静态门禁、生产构建、28/13 负向检查及完整产品/20 组模板浏览器验收重新通过；本地收据同步为修正后的实际资产与摘要。
