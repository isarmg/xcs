# HTTP 请求与响应体

`http-body` 是单一的 HTTP 流式请求体/响应体组件。原来的工具包已经作为内部 `utilities` 模块合入，不保留旧包或依赖别名。

## 功能边界

- 默认功能为空，仅提供 `Body`、`Frame`、`SizeHint` 及基础实现。原有公共类型身份保持唯一。
- `utilities` 启用 `BodyExt`、完整/空体、流适配、收集、长度限制、映射、错误处理及组合器；仅增加 `futures-core` 和 `pin-project-lite`。
- `channel` 进一步启用基于 Tokio sync 的发送/接收通道。
- `full` 包含 `channel`，因此也包含 `utilities`。

工具功能不会因为存在于同一个包而自动启用运行时。Hyper 的 FFI 功能显式启用 `http-body/utilities`；Tower HTTP 的相关可选功能同样显式转发。合包没有改变帧、trailers、背压、错误传播或长度限制的算法。

## 来源与测试

合并来源是 `http-body` 1.1.0 与 `http-body-util` 0.1.5，均为 MIT 许可。原始来源、许可证和版本继续由中央来源目录独立登记。工具源码保存在 `src/foundation/http-body/src/utilities`，测试保存在 `test/foundation/http-body/utilities`；每个测试仍保留其原始来源身份。

维护门禁为 `.github/scripts/check-http-body-consolidation.py`。它验证四种功能集合在 Linux amd64 与浏览器 WASM 下的编译、默认依赖隔离、缺少工具功能时的编译拒绝、保留的单元/集成/文档测试，以及实际 Axum、Hyper、Tower HTTP、Reqwest 消费者。门禁使用临时独立 manifest，不修改产品 manifest 或根锁文件。

包数减少一个不代表编译、体积或运行速度提升；这些结果需要单独测量。
