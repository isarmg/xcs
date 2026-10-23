<a id="service-stack"></a>

# 服务组合层

`service-stack` 统一维护 `Service` 和 `Layer` 定义、层组合、服务适配器及按需启用的中间件。旧的 `tower`、`tower-layer` 和 `tower-service` 包名不作为兼容别名保留。

<a id="boundaries"></a>

## 边界

- 不启用特性时，没有外部依赖。`Service`、`Layer`、`ServiceBuilder`、恒等层、堆叠层和函数层在原生及浏览器 WASM 目标上均可用。
- `util`、`filter`、`load-shed`、`steer` 是显式启用的纯服务工具，不会引入 Tokio。
- 运行时中间件仍由原有显式能力控制。`full` 启用维护范围内的原生中间件并集，绝不作为隐式默认值。
- 未改变 HTTP 协议解析、TLS 实现或 unsafe 边界。原有 `forbid(unsafe_code)` 保持不变。

<a id="deliberate-retirement"></a>

## 有意停用的能力

未使用的 hedge 中间件原本就因缺少 `hdrhistogram` 提供方而无法运行。维护范围内没有 Cargo 消费者启用它。损坏的 `hedge` 特性及其在 `full` 中的启用项已移除。五个原始生产文件完整保存在整合凭据中，未被静默丢弃。历史 hedge 测试仍保留来源记录并处于未启用状态，不计为通过的覆盖。

<a id="validation-and-origins"></a>

## 验证与来源

`.github/scripts/check-service-stack.py` 检查原生/WASM 特性隔离、零依赖基础 trait，以及真实的层组合、错误、取消和 trait 身份。它要求全部四个行为用例通过；门禁不会伪造缺失的上游测试提供方。其他依赖不可用 `tower-test` 的保留上游测试属于历史材料，不声称已执行。

来源为 Tower 0.5.3、tower-layer 0.3.3 和 tower-service 0.3.3。精确源码、许可证、上游版本及测试归属保存在 `compliance/provenance`、来源目录及整合凭据中。
