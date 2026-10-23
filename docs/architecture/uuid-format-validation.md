<a id="uuid-syntax-validation-without-unused-decoding"></a>

# UUID 语法验证

`schema-validation` 的 UUID 格式检查只判断语法，不解码 UUID 字节：输入必须恰好 36 字节，第 8、13、18、23 位为连字符，其余为 ASCII 十六进制。接受大小写，不限制版本或 variant。实现不使用 unsafe 或架构分派。

<a id="correctness-and-consumer-boundaries"></a>

## 消费者与验证

`python3 .github/scripts/check-uuid-format.py` 重放长度、各位置字节、混合大小写、Unicode、分隔符及任意输入的差异语料，并编译浏览器 WASM 判定器。参考实现仅从已认证的合规归档恢复，不是产品依赖；恢复校验完整清单、哈希、规范编码及输出路径，失败时拒绝继续。

Schema 门禁检查 2019-09 / 2020-12 draft 的真实 UUID 消费者，并拒绝 uuid-simd/vsimd/outref 进入依赖图。当前契约 schema 不使用 UUID 格式断言；该门禁不替代存储、模型或平台测试。

<a id="measured-trade-off-not-a-speedup-claim"></a>

## 性能边界

直接语法检查是宿主验证工具的接口选择，不承诺比解码器更快。`--bench` 可主动比较直接判定器；被忽略的 `uuid_validation_throughput` 用于优化构建的宿主比较，不作为共享 CI 时间断言。归档认证不表示全部参考实现的上游套件已执行。
