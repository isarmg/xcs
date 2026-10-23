<a id="service-stack"></a>

# 服务组合层

`service-stack` 提供 `Service`、`Layer`、层组合、服务适配器及可选中间件。

<a id="boundaries"></a>

## 能力边界

- 无特性时没有外部依赖；`Service`、`Layer`、`ServiceBuilder`、恒等层、堆叠层和函数层支持原生及浏览器 WASM。
- `util`、`filter`、`load-shed`、`steer` 为显式启用的纯服务工具，不引入 Tokio。
- 运行时中间件按需启用；`full` 是维护范围内的原生中间件并集，不是默认值。
- 全库禁止 unsafe，不负责 HTTP 协议解析或 TLS。
- 不提供 `hedge` 中间件或对应特性，`full` 也不包含它。

<a id="validation-and-origins"></a>

## 检查与来源

运行 `python3 .github/scripts/check-service-stack.py`，检查原生/WASM 特性隔离、零依赖基础 trait，以及真实层组合、错误、取消和 trait 身份。依赖不可用 `tower-test` 的上游用例及 hedge 测试仅保留来源，不计为执行。

来源为 Tower 0.5.3、tower-layer 0.3.3 和 tower-service 0.3.3。精确源码、许可证、上游版本和测试归属保存在 `compliance/provenance`、来源目录及整合凭据中，包括不启用的 hedge 原始生产文件。
