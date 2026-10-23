<a id="component-and-slot-property-options"></a>

# 组件与插槽属性选项

`src/macros/src/prop_options.rs` 统一负责组件和插槽展开使用的 `#[prop(...)]` 语法。它使用已依赖的 `rust-syntax` AST 解析器，不通过切分字符串或 token 来解析 Rust 表达式。

<a id="boundaries"></a>

## 边界

- 共享选项：`optional`、`optional_no_strip`、`strip_option`、`default`、`into`、`attrs`。
- 组件专用选项：`name`、`marker`。插槽拒绝它们，不接受无法实现的选项。
- 标记接受裸名称、`= true` / `= false` 或括号布尔值。默认值接受 Rust 表达式；重命名接受字符串字面值。两种值选项都接受 `= value`、`(value)`。
- 多个属性合并；即使值相同也拒绝重复字段。互斥依据是否出现而非布尔值，因此 `optional = false, strip_option` 仍错误。
- 每个属性先独立解析，再与先前属性合并。合并顺序与按字母排序的冲突对保证诊断确定性。
- 该模块仅解析选项；转为 builder 配置、生成 token 的卫生性及运行时行为仍由各自组件/插槽模块负责。

专用解析器替代只服务这两种选项记录的泛型属性派生，不替换共享 Rust token/AST 库，也不合并宿主过程宏与浏览器运行时。移除唯一依赖边并通过完整依赖治理后，退役以下泛型派生闭包：`attribute-derive`、`attribute-derive-macro`、`interpolator`、`manyhow`、`manyhow-macros`、`proc-macro-utils`、`quote-use`、`quote-use-macros`。这是编译期依赖减少，不是浏览器加速或构建耗时改善的测量。

<a id="origin-and-verification"></a>

## 来源与验证

现有组件/插槽实现来自保留的 Leptos 派生宏源码，登记于 `compliance/licenses/upstream/zxs-web-macro-0.8.19/UPSTREAM-NOTICE.md`。旧选项解析器由 `attribute-derive`、`attribute-derive-macro` 0.11.0 生成。专用实现参考其 span 合并/回退诊断和确定性合并/冲突顺序；即使独立包已退役，仍保留这两项语义/设计来源归属。这不是物理源码合并，没有复制原函数/模块体到宏包。原始许可证/通知在 `compliance/licenses/upstream/`，源码/来源在 `compliance/provenance/`，说明在 `docs/upstream/`。原 `docs/traits.html` rustdoc 片段逐字保留于 `docs/upstream/attribute-derive-0.11.0/docs/traits.html`；相对链接是上游 rustdoc 占位，不是已生成的本地文档。其余六个包的通用解析、插值或过程宏工具实现没有迁入。

专用解析器对比此前实现捕获的行为，不另加通用解析器开发依赖。语料覆盖可接受语法、坏输入、显式 false、顺序选项对、单属性/跨属性重复和诊断优先级。项目测试/夹具位于 `test/view-macros/unit/`，与 `test/macros/` 中归属明确的导入测试分开。

<a id="validation-evidence-2026-10-23"></a>

### 验证记录（2026-10-23）

删除旧依赖前，269 个输入在组件和插槽模式下分别由新旧解析器执行，538 个模式/输入组合的成功值、诊断消息/数量及源码 span 全部一致。随后删除临时旧 derive；`prop_options_cases.rs` 仅保留捕获的期望结果，不依赖旧解析器。专项 span 断言仍与语料一起保留，参考代码为 `794c3514`。

删除依赖边后，`cargo test --offline -p view-macros` 通过 19 项单元和 26 项文档测试，覆盖保留宏、消费者上下文和语法测试。`cargo check --locked --offline -p web-app --target wasm32-unknown-unknown` 也通过，检查真实浏览器消费者；不能由这些正确性检查推导构建耗时或应用性能改善。

集成消费者门禁强制发现并运行规范解析器测试，与真实完整 Rust AST、bindgen、终端输出和解析消费者一起验证，不再只为探针保留 attribute-derive。`cargo clippy --locked --offline -p view-macros --all-targets -- -D warnings` 通过。临时解析值枚举仅对默认表达式 AST 装箱，避免每个布尔/名称槽都携带大载荷；公开选项形态和捕获的解析/诊断行为不变。
