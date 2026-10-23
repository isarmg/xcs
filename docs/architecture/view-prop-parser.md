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

该解析器不替代共享 Rust token/AST 库，也不合并宿主过程宏与浏览器运行时。

<a id="origin-and-verification"></a>

## 来源与验证

现有组件/插槽实现来自保留的 Leptos 派生宏源码，登记于 `compliance/licenses/upstream/zxs-web-macro-0.8.19/UPSTREAM-NOTICE.md`。旧选项解析器由 `attribute-derive`、`attribute-derive-macro` 0.11.0 生成。专用实现参考其 span 合并/回退诊断和确定性合并/冲突顺序；即使独立包已退役，仍保留这两项语义/设计来源归属。这不是物理源码合并，没有复制原函数/模块体到宏包。原始许可证/通知在 `compliance/licenses/upstream/`，源码/来源在 `compliance/provenance/`，说明在 `docs/upstream/`。原 `docs/traits.html` rustdoc 片段逐字保留于 `docs/upstream/attribute-derive-0.11.0/docs/traits.html`；相对链接是上游 rustdoc 占位，不是已生成的本地文档。其余六个包的通用解析、插值或过程宏工具实现没有迁入。

专用解析器使用固定的行为语料，不引入通用解析器开发依赖。语料覆盖可接受语法、坏输入、显式 false、顺序选项对、单属性/跨属性重复和诊断优先级。项目测试/夹具位于 `test/view-macros/unit/`，与 `test/macros/` 中归属明确的导入测试分开。

运行 `cargo test --locked --offline -p view-macros` 检查语法和文档示例；`cargo check --locked --offline -p web-app --target wasm32-unknown-unknown` 检查浏览器消费者。编译与语料测试不证明构建耗时或浏览器性能改善。
