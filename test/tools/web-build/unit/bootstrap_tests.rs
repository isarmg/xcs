use super::*;

#[test]
fn readiness_requires_runtime_and_styles_not_optional_fonts() {
    let loader = loader();
    let ready = loader
        .split("function ready()")
        .nth(1)
        .unwrap()
        .split("function failed()")
        .next()
        .unwrap();
    assert!(ready.contains("data-zxs-runtime"));
    assert!(!ready.contains("data-zxs-fonts"));
    assert!(loader.contains("link[rel=\"stylesheet\"]"));
    assert!(loader.contains("if (!settled) return module.default()"));
    assert!(loader.contains("setTimeout(() => fail(\"timeout\"), 30000)"));
}

#[test]
fn startup_failure_identifies_modern_wasm_requirement() {
    assert!(index().contains("WebAssembly SIMD"));
    assert!(loader().contains("modern browser with WebAssembly SIMD"));
    assert!(loader().contains("支持 WebAssembly SIMD 的现代浏览器"));
}
