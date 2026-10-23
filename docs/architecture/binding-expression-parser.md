<a id="binding-expression-parser-consolidation"></a>

# C 绑定表达式解析器整合

仅宿主机使用的 C 绑定路径现将 C 表达式解析器放在 `src/foundation/tooling/bindgen/src/cexpr`。原 `bindgen → cexpr → nom → minimal-lexical` 包链缩为一个包，不是三个改名或转发依赖。`clang-sys` 继续作为独立 ABI 边界。三个移除源码树共 47 份文件 / 614,895 字节；专用解析器为五份 / 50,962 字节，净减 563,933 字节（91.7%），此口径不含测试、来源材料和其他未变 bindgen 源码。

<a id="reviewed-call-closure"></a>

## 已审阅的调用闭包

- `clang.rs` 将 libclang token 转为保持不变的 `Token { kind, raw }`；`callbacks::{Token, TokenKind}` 仍为 `ParseCallbacks::modify_macro` 暴露五种 token 及其拥有的字节。
- `ir/var.rs` 用 `IdentifierParser::macro_definition` 求值宏，以 `expr::expr` 处理字面 token 流，并消费 `EvalResult`、`CChar`；`ir/context.rs` 保留标识符/值映射。它们是原 cexpr 的全部生产消费者。
- 字面语法只需要字节标签、字符集、分支、映射、顺序、重复与识别；表达式语法使用相同有界切片操作。不需要位解析器、通用数值解析器、字符串输入实现、排列、迭代器或 nom 数值/浮点机制。
- cexpr 自有十进制浮点识别调用 Rust `f64::from_str`，因此 minimal-lexical 数值转换虽在旧 Cargo 图中，却不在实际调用闭包。
- 新切片操作针对该闭包独立实现，并承认 nom 7.1.3 行为来源和保留的 MIT 许可；不是复制的私有 nom 包或兼容重导出。

<a id="semantics-and-boundaries"></a>

## 语义与边界

既有 C 表达式算法保留：整数字面先解析为 `u64` 再按位转为 `i64`，算术采用 `Wrapping<i64>`，宽度/符号后缀处理不变。运算符优先级、浮点/整数混合提升、位移、转义字符/原始字节、字符串拼接、标识符替换与函数宏声明解析均保留。分支只对可恢复错误回溯；输入不完整是独立状态，保留所需大小和剩余输入。

不新增旧求值器不具备的 C 类型语义：数值模型为 i64/f64，右移为算术移位，忽略宽度前缀，不求值 cast、sizeof 或十六进制浮点。bindgen 独立的 clang 求值/回退路径（包括 cast 与真实 ABI 宽度）不变。SQLite 系统头生成和 Samba 构建绑定仍使用同一 bindgen/clang-sys 路径，没有增加浏览器/WASM 运行时解析器。

<a id="verification"></a>

## 验证

`python3 .github/scripts/check-binding-expressions.py` 运行六项不依赖 SDK 的回归。黄金语料覆盖 109,250 个字面输入和 50,000 个 token 序列；每个序列包含表达式、宏定义和函数宏声明结果。参考结果来自提交 `aa7ffc46e702bde224d21db6e7df6a278238a230` 的原 cexpr/nom，包含 Debug 值、剩余输入及错误/不完整类别。退役前另行并排比较每个结果，而非只比较语料摘要。显式测试覆盖全部支持运算符、回绕、浮点提升、有值标识符映射、函数参数、Unicode/原始转义、回调 token 形态、注释、非法 token、回溯、不完整传播和不消耗输入的重复。

`python3 .github/scripts/check-binding-expressions.py --clang` 另使用真实 libclang 执行保留的六套上游 C 头测试，接入本已安装 clang/libclang 和 SQLite 开发头的 foundation-data CI 分片。既有系统 SQLite 和 Samba SDK 门禁仍不可替代。仅解析器测试、默认/仅运行时 bindgen 消费者编译，不构成 SDK、ABI 或端到端绑定验证。

生产代码位于 bindgen/src；独立回归在 test/ci/fixtures，历史 clang 套件在 test/foundation/bindgen/cexpr。原始身份、许可证和退役/整合证据保留于合规材料。

不依赖 SDK 的模式接入 foundation-language 分片。路由测试要求各模式在所属分片及 aggregate foundation 门禁中各出现一次，不替代 SQLite 或 Samba 检查。

整合收据覆盖 62 份输入生产文件及 30 份外置 bindgen/cexpr 测试。两个退役依赖归档合计保留 88 份精确文件：42 份生产文件及 46 份外置测试。归档检查认证完整路径集、原始哈希、字节数、来源记录及活动树中不再存在的事实；这些保留检查不表示退役通用解析器测试套件已执行。
