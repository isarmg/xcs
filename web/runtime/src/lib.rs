//! Browser presentation, theme, locale, contours and startup lifecycle for ZXS.
extern crate self as zxs_web;

pub mod components;
pub mod contours;
pub mod i18n;
pub mod startup;
pub mod theme;
pub use zxs_web_core::*;

use wasm_bindgen::JsValue;

/// Begin CSR startup. Both font weights decode before the application is mounted.
/// Errors before scheduling are returned; later failures publish the startup failure event.
pub fn mount<F, V>(root_id: &str, app: F) -> Result<(), JsValue>
where
    F: FnOnce() -> V + 'static,
    V: IntoView + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        browser::mount(root_id, app)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (root_id, app);
        Err(JsValue::NULL)
    }
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use super::*;
    use futures::{FutureExt, channel::oneshot};
    use gloo_events::EventListener;
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use zxs_web_core::prelude::IntoAny;

    thread_local! {
        static STARTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
        static APPLICATION: RefCell<Option<Application>> = const { RefCell::new(None) };
    }
    struct Application {
        // Keep the owner and controller until an uncached page leaves.
        view: mount::UnmountHandle<tachys::view::any_view::AnyViewState>,
        contours: contours::ContourController,
        _pagehide: EventListener,
    }
    impl Drop for Application {
        fn drop(&mut self) {
            self.contours.stop();
            theme::cleanup();
            // `view` then unmounts the tree and releases its reactive owner.
            let _ = &self.view;
        }
    }
    fn cleanup() {
        let application = APPLICATION.with(|slot| slot.borrow_mut().take());
        drop(application);
    }
    pub fn mount<F, V>(root_id: &str, app: F) -> Result<(), JsValue>
    where
        F: FnOnce() -> V + 'static,
        V: IntoView + 'static,
    {
        console_error_panic_hook::set_once();
        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or_else(|| JsValue::from_str("Browser document is unavailable"))?;
        let root = document
            .get_element_by_id(root_id)
            .ok_or_else(|| JsValue::from_str("Application root is missing"))?
            .dyn_into::<web_sys::HtmlElement>()?;
        startup::begin(&document)?;
        if STARTED.with(|started| started.replace(true)) {
            return Err(JsValue::from_str("Application mounting already began"));
        }
        i18n::initialize();
        theme::initialize();
        wasm_bindgen_futures::spawn_local(async move {
            let result: Result<(), JsValue> = async {
                wait_for_fonts(&document).await?;
                startup::ensure_pending(&document)?;
                root.set_text_content(None);
                let view = mount::mount_to(root, move || app().into_any());
                let html = document
                    .document_element()
                    .ok_or_else(|| JsValue::from_str("Browser document root is missing"))?;
                html.set_attribute("data-zxs-appearance", "content-blocks")?;
                html.set_attribute("data-zxs-selection", "underline")?;
                let contours = contours::start(&document)?;
                let window = document
                    .default_view()
                    .ok_or_else(|| JsValue::from_str("Browser window is unavailable"))?;
                let pagehide = EventListener::new(&window, "pagehide", |event| {
                    if !event
                        .dyn_ref::<web_sys::PageTransitionEvent>()
                        .is_some_and(|event| event.persisted())
                    {
                        cleanup();
                    }
                });
                let application = Application {
                    view,
                    contours,
                    _pagehide: pagehide,
                };
                startup::ready(&document)?;
                APPLICATION.with(|slot| *slot.borrow_mut() = Some(application));
                Ok(())
            }
            .await;
            if let Err(error) = result {
                cleanup();
                theme::cleanup();
                startup::failed(&document, &error);
            }
        });
        Ok(())
    }
    async fn wait_for_fonts(document: &web_sys::Document) -> Result<(), JsValue> {
        let (cancel, cancelled) = oneshot::channel::<()>();
        let cancel = Rc::new(RefCell::new(Some(cancel)));
        let failed_cancel = cancel.clone();
        let _failed = EventListener::new(document, startup::FAILED_EVENT, move |_| {
            if let Some(sender) = failed_cancel.borrow_mut().take() {
                let _ = sender.send(());
            }
        });
        let window = document
            .default_view()
            .ok_or_else(|| JsValue::from_str("Browser window is unavailable"))?;
        let _pagehide = EventListener::new(&window, "pagehide", move |event| {
            if !event
                .dyn_ref::<web_sys::PageTransitionEvent>()
                .is_some_and(|event| event.persisted())
                && let Some(sender) = cancel.borrow_mut().take()
            {
                let _ = sender.send(());
            }
        });
        let fonts = async {
            let fonts = document.fonts();
            let regular = fonts.load("16px \"ZxsUI\"");
            let bold = fonts.load("bold 16px \"ZxsUI\"");
            let (regular, bold) =
                futures::try_join!(JsFuture::from(regular), JsFuture::from(bold))?;
            if js_sys::Array::from(&regular).length() == 0
                || js_sys::Array::from(&bold).length() == 0
            {
                return Err(JsValue::from_str("Application font styles are missing"));
            }
            JsFuture::from(fonts.ready()?).await?;
            Ok(())
        }
        .fuse();
        let timeout = gloo_timers::future::TimeoutFuture::new(startup::TIMEOUT_MS).fuse();
        let cancelled = cancelled.fuse();
        futures::pin_mut!(fonts, timeout, cancelled);
        futures::select! {
            result = fonts => result,
            _ = timeout => Err(JsValue::from_str("Application font loading timed out")),
            _ = cancelled => Err(JsValue::from_str("Application startup was cancelled")),
        }
    }
}
