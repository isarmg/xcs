use crate::{i18n::t, prelude::*};
use wasm_bindgen::JsCast;
/// Native modal semantics retain focus containment and background inertness.
#[component]
pub fn Dialog(title: String, close: Callback<()>, children: ChildrenFn) -> impl IntoView {
    let dialog = NodeRef::<zxs_web::html::Dialog>::new();
    let previous = send_wrapper::SendWrapper::new(
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()),
    );
    Effect::new(move |_| {
        if let Some(dialog) = dialog.get() {
            let _ = dialog.show_modal();
            if let Ok(Some(element)) = dialog.query_selector("[data-zxs-initial-focus]")
                && let Ok(element) = element.dyn_into::<web_sys::HtmlElement>()
            {
                let _ = element.focus();
            }
        }
    });
    on_cleanup(move || {
        if let Some(dialog) = dialog.get_untracked() {
            dialog.close();
        }
        if let Some(previous) = previous.as_ref()
            && previous.is_connected()
        {
            let _ = previous.focus();
        }
    });
    View::new(
        zxs_web::html::dialog()
            .child((
                zxs_web::html::div()
                    .child((
                        zxs_web::html::h2().child(
                            #[allow(unused_braces)]
                            {
                                IntoRender::into_render({ title.clone() })
                            },
                        ),
                        zxs_web::html::button()
                            .child(
                                #[allow(unused_braces)]
                                {
                                    "×"
                                },
                            )
                            .class("zxs-button")
                            .r#type("button")
                            .aria_label(
                                ::zxs_web::prelude::IntoAttributeValue::into_attribute_value(t(
                                    "关闭对话框",
                                    "Close dialog",
                                )),
                            )
                            .on(zxs_web::ev::click, move |_| close.run(())),
                    ))
                    .class("zxs-dialog-heading zxs-flow"),
                IntoRender::into_render(children()),
            ))
            .class("zxs-dialog")
            .node_ref(dialog)
            .aria_label(::zxs_web::prelude::IntoAttributeValue::into_attribute_value(title.clone()))
            .tabindex("-1")
            .on(
                zxs_web::ev::Custom::new("cancel"),
                move |event: web_sys::Event| {
                    event.prevent_default();
                    close.run(());
                },
            )
            .on(zxs_web::ev::keydown, move |event| {
                if event.key() != "Tab" {
                    return;
                }
                let Some(dialog) = dialog.get_untracked() else {
                    return;
                };
                if dialog
                    .query_selector("video[controls],audio[controls]")
                    .ok()
                    .flatten()
                    .is_some()
                {
                    return;
                }
                let Ok(nodes) =
                    dialog.query_selector_all("button,input,select,textarea,a[href],[tabindex]")
                else {
                    return;
                };
                let focusable: Vec<_> = (0..nodes.length())
                    .filter_map(|i| nodes.item(i))
                    .filter_map(|n| n.dyn_into::<web_sys::HtmlElement>().ok())
                    .filter(|e| {
                        e.tab_index() >= 0
                            && !e.matches(":disabled,[hidden]").unwrap_or(true)
                            && e.get_client_rects().length() > 0
                    })
                    .collect();
                let active = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.active_element());
                if let (Some(first), Some(last)) = (focusable.first(), focusable.last()) {
                    if event.shift_key()
                        && (active.as_ref() == Some(first.as_ref())
                            || active.as_ref() == Some(dialog.as_ref()))
                    {
                        event.prevent_default();
                        let _ = last.focus();
                    } else if !event.shift_key() && active.as_ref() == Some(last.as_ref()) {
                        event.prevent_default();
                        let _ = first.focus();
                    }
                } else {
                    event.prevent_default();
                    let _ = dialog.focus();
                }
            }),
    )
}
