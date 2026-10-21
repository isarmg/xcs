# System entropy

Single operating-system/browser CSPRNG provider for the repository. Native Linux amd64 uses the retained secure kernel entropy implementation and its existing entropy-ready `/dev/urandom` fallback. Browser WebAssembly explicitly enables `wasm_js` and uses Web Crypto. Failures remain errors; there is no predictable fallback.

```rust
let mut bytes = [0u8; 32];
system_entropy::fill(&mut bytes)?;
# Ok::<(), system_entropy::Error>(())
```

The package and Rust crate names are `system-entropy` and `system_entropy`. The optional random-generator-core feature is `system-entropy`. Internal backend configuration uses `system_entropy_backend` and test configurations use `system_entropy_test_*`. Linux syscall names and upstream citations retain their actual names.

Source: upstream getrandom 0.4.3, replacing active getrandom 0.2.17 and previously consolidated 0.3.4. Original upstream documentation, licenses, package reconstruction materials and historical test evidence remain preserved under their original identities. Backend algorithm code is not rewritten by this consolidation.
