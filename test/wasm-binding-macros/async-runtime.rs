mod browser_future_contract {
    use std::{cell::Cell, rc::Rc};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(js_namespace = Promise)]
    extern "C" {
        #[wasm_bindgen(js_name = resolve)]
        async fn imported_resolve(value: JsValue) -> JsValue;
    }

    #[wasm_bindgen]
    pub async fn async_roundtrip(value: String) -> Result<String, JsValue> {
        let value = imported_resolve(value.into()).await;
        let value = js_sys::futures::JsFuture::from(js_sys::Promise::resolve(&value)).await?;
        let value = js_sys::futures::future_to_promise(async move { Ok(value) }).await?;
        let rejected =
            js_sys::futures::future_to_promise(async { Err(JsValue::from_str("rejected")) }).await;
        if rejected
            .as_ref()
            .err()
            .and_then(JsValue::as_string)
            .as_deref()
            != Some("rejected")
        {
            return Err(JsValue::from_str("rejection propagation"));
        }
        let polled = Rc::new(Cell::new(false));
        let task_polled = polled.clone();
        js_sys::futures::spawn_local(async move { task_polled.set(true) });
        if polled.get() {
            return Err(JsValue::from_str(
                "spawn_local ran before the microtask tick",
            ));
        }
        let _ = js_sys::Promise::resolve(&JsValue::UNDEFINED).await?;
        if !polled.get() {
            return Err(JsValue::from_str(
                "spawn_local did not run on the microtask tick",
            ));
        }
        value
            .as_string()
            .ok_or_else(|| JsValue::from_str("string roundtrip"))
    }

    #[wasm_bindgen]
    pub struct AsyncCounter {
        value: u32,
    }

    #[wasm_bindgen]
    impl AsyncCounter {
        #[wasm_bindgen(constructor)]
        pub fn new(value: u32) -> Self {
            Self { value }
        }
        pub async fn increment(&mut self) -> u32 {
            self.value += 1;
            self.value
        }
    }
}
