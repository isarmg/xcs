<a id="system-entropy"></a>

# 系统熵源

仓库统一的操作系统/浏览器密码学安全伪随机数生成器（CSPRNG）提供方。原生 Linux amd64 使用保留的安全内核熵实现，以及现有的、等待熵源就绪后使用的 `/dev/urandom` 回退路径。浏览器 WebAssembly 显式启用 `wasm_js` 并使用 Web Crypto。失败仍返回错误，不提供可预测的回退值。

```rust
let mut bytes = [0u8; 32];
system_entropy::fill(&mut bytes)?;
# Ok::<(), system_entropy::Error>(())
```

包名和 Rust crate 名分别为 `system-entropy` 与 `system_entropy`。random-generator-core 的可选特性名为 `system-entropy`。内部后端配置使用 `system_entropy_backend`，测试配置使用 `system_entropy_test_*`。Linux 系统调用名称和上游引用保留其实际名称。

来源为上游 getrandom 0.4.3，替代之前活动的 getrandom 0.2.17 及更早整合的 0.3.4。原始上游文档、许可证、包重建材料及历史测试证据仍以原身份保留。本次整合未重写后端算法代码。
