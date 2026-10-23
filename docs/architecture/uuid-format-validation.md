<a id="uuid-syntax-validation-without-unused-decoding"></a>

# 无需解码的 UUID 语法验证

schema-validation 的 UUID 格式检查只需要成功/失败，不使用 UUID 字节。现使用安全的固定 36 字节判定：第 8、13、18、23 位为连字符，其余为 ASCII 十六进制。接受大小写，不限制版本或 variant。固定十六进制通道数组允许普通编译器向量化，不引入 unsafe、架构分派、复制解码器或兼容适配器。

唯一 uuid-simd 依赖及其没有其他消费者的 vsimd/outref 链已退役。原始身份、版本、许可证及完整生产/测试字节保存在经认证合规归档。未计替代实现及 manifest 变化前，活动 Rust 源码减少 29 份 / 195,516 字节；由于归档保留，这不是仓库总大小缩减声明。

<a id="correctness-and-consumer-boundaries"></a>

## 正确性与消费者边界

保留的差异探针与原解析器比较 628,043 个案例：长度、各位置的全部字节、合法混合大小写、Unicode/非 ASCII、任意输入及非法分隔符/数字布局。只有核对精确目录收据、完整文件集、SHA-256、规范编码、数量和安全输出路径后才恢复退役参考实现。无需 Git 历史或网络，浅克隆也可同样验证；退役代码仅作为参考，不是活动产品依赖。

既有 1,509 个规范夹具，以及 2019-09 / 2020-12 两种 draft 下的 UUID 消费者回归已通过。schema 检查器显式执行 UUID 测试，不仅登记一个未调用的 manifest 目标。判定器和完整 schema 验证器在现有 wasm_js 配置下通过浏览器 WASM 编译。根 contract-check 测试及仓库文档契约验证也通过，但不替代存储、模型或平台测试。当前契约 schema 不使用 UUID 格式断言。

<a id="measured-trade-off-not-a-speedup-claim"></a>

## 实测取舍，不是加速声明

与原 SIMD 解码器相比，直接合法/非法十六进制案例增加约 1—4 纳秒。最终混合判定样本为每输入 13.43 → 13.67 ns。四次交替的优化对象验证运行，每次含 11 组一百万对象样本，聚合中位数为 58.713 → 59.653 ns（+1.6%），抖动范围大量重叠。未证实稳定且显著的宿主影响，也不声称加速。这是宿主验证工具简化，不是已证明的应用热路径优化。

`python3 .github/scripts/check-uuid-format.py` 重放差异语料，`--bench` 另报告直接判定器比较。被忽略的 `uuid_validation_throughput` 消费者测试保留用于主动执行优化版新旧宿主比较，不作为共享 CI 的时间断言。

三个退役归档保存全部 32 份生产文件 / 197,446 字节及六份外置测试 / 9,148 字节，连同原收据。归档完整性认证不表示全部退役 uuid-simd/vsimd/outref 上游套件执行。

默认差异门禁在 foundation-language 与 aggregate foundation 中各运行一次，由清单/路由检查约束，并编译浏览器 WASM 判定器。schema 能力门禁运行 UUID 消费者回归，在每个检查的特性图中拒绝 uuid-simd/vsimd/outref。归档恢复的损坏和路径越界回归采取失败关闭策略，优化 Python（`-O`）下也通过。
