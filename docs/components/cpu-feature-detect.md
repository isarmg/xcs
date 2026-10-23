<a id="cpu-feature-detection"></a>

# CPU 特性检测

`cpu-feature-detect` 提供带缓存的运行时 CPU 特性检查，无需 `std`。Rust crate 标识为 `cpu_feature_detect`。检查 CPU 和操作系统状态，不会在全局启用更强的目标指令。

```rust
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
{
    cpu_feature_detect::new!(dispatch, "avx2");
    let (token, detected) = dispatch::init_get();
    assert_eq!(detected, token.get());
    assert_eq!(detected, dispatch::get());
}
```

含多个特性字符串的检测器要求全部满足。初始化会缓存结果；零大小令牌允许后续读取时不再重复初始化检查。编译器启用的目标特性可能让检查成为常量。运行时检测本身并不使不受支持的内建指令变得可安全执行。AVX 系列检查保留必要的 OSXSAVE/XSAVE 和 XCR0 状态谓词。

维护的部署范围为 Linux amd64 和浏览器 WASM 消费者。宏的架构专用实现仍受条件编译控制；浏览器代码不得无条件调用仅适用于 x86 的分派。

未改动的[上游文档](../upstream/cpufeatures-0.3.1/README.md)记录 cpufeatures 0.3.1、其原始平台/特性目录和许可证。原始源码、版本及法律材料保留在来源目录中。
