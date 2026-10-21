# CPU feature detection

`cpu-feature-detect` provides cached runtime CPU-feature checks without requiring
`std`. The Rust crate identifier is `cpu_feature_detect`. It retains the CPU and
operating-system state checks of the consolidated implementation; renaming does
not enable stronger target instructions globally.

```rust
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
{
    cpu_feature_detect::new!(dispatch, "avx2");
    let (token, detected) = dispatch::init_get();
    assert_eq!(detected, token.get());
    assert_eq!(detected, dispatch::get());
}
```

A detector with several feature strings requires all of them. Initialization
caches the result; its zero-sized token allows later reads without repeating the
initialization check. Compiler-enabled target features can make a check constant.
Runtime detection does not itself make it safe to execute an unsupported intrinsic.
AVX-family checks retain the required OSXSAVE/XSAVE and XCR0 state predicates.

The maintained deployment scope is Linux amd64 and browser WASM consumers. The
macro's architecture-specific implementation remains conditional; browser code
must not invoke x86-only dispatch unconditionally.

The unmodified [upstream documentation](../upstream/cpufeatures-0.3.1/README.md)
records cpufeatures 0.3.1, its original platform/feature catalogue and licenses.
Original source, versions and legal material remain in the provenance catalogue.
