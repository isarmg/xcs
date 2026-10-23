<a id="unified-data-serialization"></a>

# 统一数据序列化

项目维护唯一的 `data-serialization` 实现，负责序列化 trait、标准库/core 类型实现、派生支持及可选的结构化错误路径。`serialization-derive` 仍为独立过程宏 crate，并固定到完全匹配的私有支持版本。这种拆分是 Rust 过程宏编译模型的要求，而非上游仓库布局的要求。

<a id="capabilities"></a>

## 能力

- 不启用默认特性：提供无分配 trait 和 core 类型实现，不以普通依赖引入过程宏工具、JSON 或整数格式化。
- `alloc`：自持有集合、字符串及经过缓冲的派生值。
- `std`：标准库类型，包含 `alloc`。
- `rc`：在可分配时提供 `Rc`/`Arc` 实现。
- `derive`：可选宏再导出；普通手写 trait 实现不会启用宏依赖。
- `error-path`：位于 `data_serialization::error_path` 的仅需 alloc 模块；路径显示使用可选整数格式化器。

所有保留 API 均面向项目的 Rust 1.99 工具链、Linux amd64 和浏览器 WASM。旧编译器探针、生成的私有模块、`serialization_if_integer128!` 空操作兼容宏、独立 core 误用守卫及不受支持的 nightly 特性已移除。Result 序列化是普通基础实现，不是伪装的兼容特性。

<a id="implementation-changes"></a>

## 实现变化

原运行时核心与仅供文档使用的重复核心现共用一套实现和文档树。两份旧副本存在差异：仅文档版本省略了 Result 实现，并引用其独立提取的格式化测试。统一实现保留真实运行时的 Result 行为，并运行两份均保留来源的原始格式化测试。门面与核心的私有辅助已合并，移除了跨 crate 转发及重复的私有 Content 表示。派生输出与运行时统一使用现有有效的 `__derive_support` 命名空间。

这些变化减少重复源码和编译边界。在没有对比测量前，不宣称运行加速、构建提速或二进制体积缩减。JSON/TOML 协议标记、有符号/无符号整数语义、借用反序列化及自定义派生 crate 路径仍是契约。

<a id="sources-and-checks"></a>

## 来源与检查

原始身份、版本和许可证仍保存在 `compliance/provenance`、`docs/upstream` 及内容寻址的整合凭据中：serde 1.0.229、serde_core 1.0.229 和 serde_path_to_error 0.1.20。这三个输入不被描述为新的上游版本。

实现位于 `src/foundation/data-storage/data-serialization`；原始测试在 `test/foundation/data-serialization` 下保留来源。CI language 分片同时运行协调后的序列化消费者门禁与新整合门禁：七种原生/WASM 特性配置、依赖边界及缺失能力检查、原始路径测试、精确的整数/Result/派生/Content/借用/路径回归、原始格式化单元测试和文档测试。源码治理另行核验精确原始快照、去重路径、构建脚本停用证据及不可变历史；不会把审阅通过的源码转换等同于行为等价。

<a id="receiving-tree-validation-2026-10-21"></a>

## 接收树验证（2026-10-21）

基于 `eb813854` 的组合接收树通过七种原生/WASM 配置、四项能力反向检查、20 项语义/错误路径/日志序列化测试、两项保留来源的源码测试及 72 项文档测试。存储服务端、管理 CLI 和 web-build 通过锁定依赖的离线类型检查与严格的全目标 Clippy。包含 231 项测试的可移植 CI 套件及文档契约通过。刷新后的源码元数据和生成的消费者索引覆盖 273 个活动来源，其中包括 271 个基础包。这些检查不是浏览器运行时或性能结论。

日志序列化由真实的 `serialization` 特性控制，不再使用已移除的依赖名。UUID 独立的 system-entropy 依赖和浏览器特性连接保持不变。此前停用的四个 HTTP 能力源码文件及完整 HTTP 请求头期限实现均未改变。
