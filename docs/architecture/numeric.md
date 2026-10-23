<a id="numeric-foundation"></a>

# 数值基础组件

`numeric` 聚合基础数值 trait 与可选数学值类型。各来源的许可证和归属由集中合规材料独立管理。

<a id="boundaries"></a>

## 能力边界

- `traits` 始终可用，并从 crate 根导出。默认只启用 `std`。仅使用 trait 不编译大整数、复数、有理数或迭代器，也没有运行时依赖。
- `integer` 提供整数算法、求根与平均值。
- `iter` 需要 `integer`；`complex` 不依赖分配器。
- `rational` 需要 `integer`，固定大小比值无需分配。
- `bigint` 需要 `alloc` 与 `integer`；同时启用 `rational`、`bigint` 才提供大整数有理数别名。
- `collection` 提供传统广泛数学接口，但不自动启用堆分配。消费者按需显式选择 `bigint`。
- `serialization` 使用真实维护的 `data-serialization`，只为已选择的值模块启用适配。
- `std` 不隐含 bigint 或分配能力。Linux amd64 与浏览器 WASM 分别在禁用默认特性条件下检查。

`autocfg` 仍为构建依赖。原构建脚本探测浮点全序能力，在宿主机运行，不链接到仅 trait 或 WASM 消费者，也没有反向依赖 numeric。

<a id="consumers"></a>

## 消费者

日期有界整数由 `date-time` 内部维护。`fraction` 使用 collection 接口，其 `with-bigint` 特性启用 bigint 与 std。schema 值/验证实现使用基础 trait，仅在任意精度模式启用 bigint；当前归属见 [schema-validation](schema-validation.md)。绝对外部 crate 路径用于区别消费者内部已有的 `numeric` 模块。

<a id="removed-unsupported-surfaces"></a>

## 不支持的接口

不提供 libm、rand、arbitrary、quickcheck、rkyv、bytecheck、bytemuck 集成，不引入空壳提供者或无效特性。普通 `no_std` float-core、整数/有理数/复数、大整数及真实序列化可用；i128 无条件支持。

<a id="origins"></a>

## 原始来源

- num 0.4.3：https://github.com/rust-num/num
- num-bigint 0.4.8：https://github.com/rust-num/num-bigint
- num-complex 0.4.6：https://github.com/rust-num/num-complex
- num-integer 0.1.47：https://github.com/rust-num/num-integer
- num-iter 0.1.46：https://github.com/rust-num/num-iter
- num-rational 0.4.2：https://github.com/rust-num/num-rational
- num-traits 0.2.19：https://github.com/rust-num/num-traits

七个来源均为 MIT OR Apache-2.0。维护包采用主来源 num-traits 的 Cargo 版本，不表示所有来源版本相同。原始法律资料见不可变合规材料与上游文档。

<a id="validation-and-integration-status"></a>

## 验证

维护的 CI 门禁检查迁移单元测试、显式接入的上游集成测试、真实消费者编译及最小原生/WASM 特性矩阵。编译与行为测试不是性能基准。

精度消费者门禁独立检查 schema 值/验证能力的特性转发及九项精确值回归，包含超出 u64 的整数、有理数分子/分母、multipleOf，以及 1e400、1e-400 指数。

九份上游基准源码逐字节保留在 `origins/num-bigint/benches` 和 `origins/num-integer/benches`。它们是未激活的上游测试材料，不是生产实现或已运行基准；继续保留 `autobenches = false`，不得静默丢弃历史清单。
