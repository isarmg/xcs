//! Browser presentation, theme, locale, contours and startup lifecycle for ZXS.
extern crate self as zxs_web;

pub mod components;
#[expect(
    clippy::doc_lazy_continuation,
    reason = "Preserve the audited G4 source snapshot byte-for-byte; its legacy math documentation uses non-indented continuation lines"
)]
pub mod contours;
pub mod i18n;
#[cfg(any(target_arch = "wasm32", test))]
mod lifecycle;
pub mod startup;
pub mod tasks;
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
        static APPLICATION: RefCell<lifecycle::Lifecycle<Application>> =
            const { RefCell::new(lifecycle::Lifecycle::new()) };
    }
    struct Application {
        // Keep the owner and controller until an uncached page leaves.
        view: mount::UnmountHandle<tachys::view::any_view::AnyViewState>,
        contours: contours::ContourController,
        _listeners: (EventListener, EventListener),
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
        let application = APPLICATION.with(|slot| slot.borrow_mut().close());
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
        let window = document
            .default_view()
            .ok_or_else(|| JsValue::from_str("Browser window is unavailable"))?;
        if !APPLICATION.with(|slot| slot.borrow_mut().begin()) {
            return Err(JsValue::from_str("Application mounting already began"));
        }
        if let Err(error) = startup::begin(&document) {
            cleanup();
            return Err(error);
        }
        // Register before yielding, and retain these same listeners through the
        // mounted lifetime. No cancellation gap exists between font and view work.
        let (cancel, cancelled) = oneshot::channel::<()>();
        let cancel = Rc::new(RefCell::new(Some(cancel)));
        let failed_cancel = cancel.clone();
        let failed = EventListener::new(&document, startup::FAILED_EVENT, move |_| {
            if let Some(sender) = failed_cancel.borrow_mut().take() {
                let _ = sender.send(());
            }
            cleanup();
        });
        let pagehide = EventListener::new(&window, "pagehide", move |event| {
            if !event
                .dyn_ref::<web_sys::PageTransitionEvent>()
                .is_some_and(|event| event.persisted())
            {
                if let Some(sender) = cancel.borrow_mut().take() {
                    let _ = sender.send(());
                }
                cleanup();
            }
        });
        i18n::initialize();
        theme::initialize();
        wasm_bindgen_futures::spawn_local(async move {
            let result: Result<(), JsValue> = async {
                wait_for_fonts(&document, cancelled).await?;
                ensure_starting()?;
                startup::ensure_pending(&document)?;
                root.set_text_content(None);
                let view = mount::mount_to(root, move || {
                    tasks::install();
                    app().into_any()
                });
                // Components may synchronously dispatch lifecycle events too.
                ensure_starting()?;
                let html = document
                    .document_element()
                    .ok_or_else(|| JsValue::from_str("Browser document root is missing"))?;
                html.set_attribute("data-zxs-appearance", "content-blocks")?;
                html.set_attribute("data-zxs-selection", "underline")?;
                let contours = contours::start(&document)?;
                let application = Application {
                    view,
                    contours,
                    _listeners: (pagehide, failed),
                };
                // Publish before dispatch: a synchronous ready listener can
                // navigate away or reject startup and must find the live owner.
                let installed = APPLICATION.with(|slot| slot.borrow_mut().install(application));
                if let Err(application) = installed {
                    drop(application);
                    return Err(JsValue::from_str("Application startup was cancelled"));
                }
                startup::ready(&document)?;
                if !APPLICATION.with(|slot| slot.borrow().is_mounted()) {
                    return Err(JsValue::from_str("Application startup was cancelled"));
                }
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
    fn ensure_starting() -> Result<(), JsValue> {
        if APPLICATION.with(|slot| slot.borrow().is_starting()) {
            Ok(())
        } else {
            Err(JsValue::from_str("Application startup was cancelled"))
        }
    }
    async fn wait_for_fonts(
        document: &web_sys::Document,
        cancelled: oneshot::Receiver<()>,
    ) -> Result<(), JsValue> {
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
            // FontFaceSet::load resolves after the requested faces are loaded.
            // Waiting for document.fonts.ready here also waits for unrelated
            // document fonts and layout, although neither is a startup resource.
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
