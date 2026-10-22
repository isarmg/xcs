use crate::{expand, expand_class_marker};
use proc_macro2::TokenStream;
use quote::quote;

fn generated(attr: TokenStream, item: TokenStream, expected: &str) -> String {
    let result = expand(attr, item).unwrap().to_string();
    assert!(result.contains(expected), "{result}");
    assert!(!result.contains("wasm_bindgen_futures"), "{result}");
    result
}

#[test]
fn async_exports_use_canonical_executor() {
    generated(
        quote! {},
        quote! { pub async fn value() -> u32 { 1 } },
        "js_sys :: futures :: future_to_promise",
    );
    generated(
        quote! { js_sys = browser },
        quote! { pub async fn value() -> u32 { 1 } },
        "browser :: futures :: future_to_promise",
    );
}

#[test]
fn async_start_and_main_use_canonical_executor() {
    generated(
        quote! { start },
        quote! { pub async fn start() {} },
        "js_sys :: futures :: spawn_local",
    );
    generated(
        quote! { main, js_sys = browser },
        quote! { async fn main() {} },
        "browser :: futures :: spawn_local",
    );
}

#[test]
fn async_imports_use_canonical_promise_and_future() {
    for attr in [quote! {}, quote! { js_sys = browser }] {
        let path = if attr.is_empty() { "js_sys" } else { "browser" };
        let result = generated(
            attr,
            quote! { extern "C" { async fn load() -> u32; } },
            &format!("{path} :: futures :: JsFuture"),
        );
        assert!(result.contains(&format!("{path} :: Promise")), "{result}");
    }
}

#[test]
fn generic_async_imports_use_canonical_executor() {
    generated(
        quote! { js_sys = browser },
        quote! {
            extern "C" {
                #[wasm_bindgen(experimental_generic_mono)]
                async fn load<T: wasm_bindgen::JsGeneric>(value: T) -> T;
            }
        },
        "browser :: futures :: JsFuture",
    );
}

#[test]
fn async_class_marker_preserves_js_sys_override() {
    let result = expand_class_marker(
        quote! { Counter = "Counter", js_sys = browser },
        quote! { pub async fn value(&self) -> u32 { 1 } },
    )
    .unwrap()
    .to_string();
    assert!(
        result.contains("browser :: futures :: future_to_promise"),
        "{result}"
    );
    assert!(!result.contains("wasm_bindgen_futures"), "{result}");
}

#[test]
fn removed_experimental_tokio_never_falls_back() {
    for attrs in [
        quote! { experimental_tokio },
        quote! { experimental_tokio = "isolated" },
        quote! { experimental_tokio = "other" },
        quote! { jspi, experimental_tokio },
        quote! { start, experimental_tokio },
    ] {
        let error = expand(attrs, quote! { pub async fn task() {} }).unwrap_err();
        assert!(quote! { #error }
            .to_string()
            .contains("experimental_tokio is unsupported"));
    }
    let error = expand(
        quote! {},
        quote! {
            extern "C" { #[wasm_bindgen(experimental_tokio)] async fn task(); }
        },
    )
    .unwrap_err();
    assert!(quote! { #error }
        .to_string()
        .contains("experimental_tokio is unsupported"));
}

#[test]
fn removed_futures_override_is_an_explicit_error() {
    for item in [
        quote! { pub async fn task() {} },
        quote! { pub struct Counter {} },
        quote! { impl Counter { pub async fn task(&self) {} } },
        quote! { extern "C" { async fn task(); } },
    ] {
        let error = expand(quote! { wasm_bindgen_futures = legacy }, item).unwrap_err();
        assert!(quote! { #error }
            .to_string()
            .contains("wasm_bindgen_futures is retired"));
    }
}
