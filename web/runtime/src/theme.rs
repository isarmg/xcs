use gloo_events::EventListener;
use std::cell::{Cell, RefCell};
use wasm_bindgen::JsCast;

thread_local! {
    static INITIALIZED: Cell<bool> = const { Cell::new(false) };
    static MANUAL: Cell<bool> = const { Cell::new(false) };
    static SYSTEM_LISTENER: RefCell<Option<EventListener>> = const { RefCell::new(None) };
}

pub fn current() -> String {
    let Some(window) = web_sys::window() else {
        return "light".into();
    };
    if let Some(document) = window.document()
        && let Some(root) = document.document_element()
        && let Some(value) = root.get_attribute("data-xcs-theme")
        && (value == "light" || value == "dark")
    {
        return value;
    }
    if let Ok(Some(storage)) = window.local_storage()
        && let Ok(Some(value)) = storage.get_item("xcs-theme")
        && (value == "light" || value == "dark")
    {
        return value;
    }
    if window
        .match_media("(prefers-color-scheme: dark)")
        .ok()
        .flatten()
        .is_some_and(|query| query.matches())
    {
        "dark"
    } else {
        "light"
    }
    .into()
}

fn apply(value: &str) {
    if let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        let _ = root.set_attribute("data-xcs-theme", value);
    }
}

pub fn initialize() {
    if INITIALIZED.with(|initialized| initialized.replace(true)) {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let saved = window
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item("xcs-theme").ok().flatten())
        .filter(|value| value == "light" || value == "dark");
    MANUAL.with(|manual| manual.set(saved.is_some()));
    let query = window
        .match_media("(prefers-color-scheme: dark)")
        .ok()
        .flatten();
    apply(
        saved
            .as_deref()
            .unwrap_or(if query.as_ref().is_some_and(|query| query.matches()) {
                "dark"
            } else {
                "light"
            }),
    );
    if let Some(query) = query {
        let listener = EventListener::new(&query, "change", |event| {
            if MANUAL.with(Cell::get) {
                return;
            }
            if let Some(event) = event.dyn_ref::<web_sys::MediaQueryListEvent>() {
                apply(if event.matches() { "dark" } else { "light" });
                changed();
            }
        });
        SYSTEM_LISTENER.with(|listener_slot| *listener_slot.borrow_mut() = Some(listener));
    }
}

pub fn toggle() {
    let next = if current() == "dark" { "light" } else { "dark" };
    // Keep an explicit preference for this document even when storage is denied.
    MANUAL.with(|manual| manual.set(true));
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item("xcs-theme", next);
        }
        apply(next);
        changed();
    }
}

fn changed() {
    if let Some(window) = web_sys::window()
        && let Ok(event) = web_sys::Event::new("xcs-theme-change")
    {
        let _ = window.dispatch_event(&event);
    }
}

/// Release the system preference listener when this document leaves its lifecycle.
#[cfg(target_arch = "wasm32")]
pub(crate) fn cleanup() {
    SYSTEM_LISTENER.with(|listener| listener.borrow_mut().take());
    INITIALIZED.with(|initialized| initialized.set(false));
}
