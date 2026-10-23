#[wasm_bindgen]
pub async fn task_dispatch_contract() -> Result<(), JsValue> {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use task_dispatch::Executor;

    Executor::init_wasm_bindgen().map_err(|_| JsValue::from_str("executor already set"))?;
    if Executor::init_wasm_bindgen().is_ok() {
        return Err(JsValue::from_str(
            "duplicate executor initialization accepted",
        ));
    }
    static SEND_POLLED: AtomicBool = AtomicBool::new(false);
    let local_polled = Rc::new(Cell::new(false));
    let task_polled = local_polled.clone();
    Executor::spawn(async { SEND_POLLED.store(true, Ordering::Relaxed) });
    Executor::spawn_local(async move { task_polled.set(true) });
    if SEND_POLLED.load(Ordering::Relaxed) || local_polled.get() {
        return Err(JsValue::from_str("dispatch ran before the microtask tick"));
    }
    Executor::tick().await;
    if !SEND_POLLED.load(Ordering::Relaxed) || !local_polled.get() {
        return Err(JsValue::from_str("dispatch lost a browser task"));
    }
    Ok(())
}
