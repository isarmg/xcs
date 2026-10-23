<a id="unified-data-serialization"></a>

# 数据序列化

`data-serialization` 提供序列化 trait、标准库/core 类型实现、派生支持及可选结构化错误路径。`serialization-derive` 是独立过程宏 crate，固定到匹配的私有支持版本。

<a id="capabilities"></a>

## 能力

- 无默认特性：无分配 trait 和 core 类型，不引入过程宏、JSON 或整数格式化。
- `alloc`：自持有集合、字符串及缓冲派生值。
- `std`：标准库类型，包含 `alloc`。
- `rc`：在可分配时提供 `Rc`/`Arc` 实现。
- `derive`：可选宏再导出，手写 trait 实现无需开启。
- `error-path`：仅需 alloc 的 `data_serialization::error_path`；显示路径时启用可选整数格式化器。

支持项目的 Rust 1.99 工具链、Linux amd64 和浏览器 WASM。派生输出与运行时使用同一 `__derive_support` 命名空间。JSON/TOML 协议标记、有符号/无符号整数语义、Result、借用反序列化及自定义派生 crate 路径均为维护契约。不提供旧编译器兼容探针、空操作兼容宏或不受支持的 nightly 特性。

<a id="sources-and-checks"></a>

## 检查与来源

实现位于 `src/foundation/data-storage/data-serialization`，测试位于 `test/foundation/data-serialization`。运行：

```sh
python3 .github/scripts/check-serialization-family.py
python3 .github/scripts/check-serialization-consolidation.py
```

门禁检查原生/WASM 特性配置、依赖隔离及缺失能力拒绝，并测试整数、Result、派生、Content、借用、错误路径、日志序列化、格式化和文档示例。源码完整性与行为测试分别验证，不互相替代。

来源为 serde 1.0.229、serde_core 1.0.229 和 serde_path_to_error 0.1.20；原始身份、许可证、文档及内容寻址凭据保存在 `compliance/provenance`、`docs/upstream` 和来源清单中。
