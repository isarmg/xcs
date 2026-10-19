use crate::prelude::*;
use gloo_timers::callback::Timeout;
#[derive(Clone)]
pub(super) struct ToastRecord {
    pub(super) id: u64,
    pub(super) message: String,
}
#[derive(Clone, Copy)]
pub(super) struct Toasts {
    pub(super) records: RwSignal<Vec<ToastRecord>>,
    pub(super) sequence: RwSignal<u64>,
}
pub fn notify(message: String) {
    // Async completions have no reactive Owner. Dispatch on the current
    // document rather than trying to resolve a context after an await.
    let Some(window) = web_sys::window() else {
        return;
    };
    let init = web_sys::CustomEventInit::new();
    init.set_detail(&wasm_bindgen::JsValue::from_str(&message));
    if let Ok(event) = web_sys::CustomEvent::new_with_event_init_dict("xcs-notify", &init) {
        let _ = window.dispatch_event(&event);
    }
}
pub(super) fn push_toast(toasts: Toasts, message: String) {
    toasts.sequence.update(|n| *n += 1);
    let id = toasts.sequence.get_untracked();
    toasts.records.update(|records| {
        if records.len() >= 5 {
            records.remove(0);
        }
        records.push(ToastRecord {
            id,
            message: message.chars().take(512).collect(),
        });
    });
    Timeout::new(5_000, move || {
        let _ = toasts
            .records
            .try_update(|records| records.retain(|r| r.id != id));
    })
    .forget();
}
