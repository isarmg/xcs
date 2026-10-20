use crate::{
    component::{component_props_builder, component_view},
    i18n::{switch_language, t},
    portal::Portal,
    prelude::*,
    theme,
};
use gloo_events::EventListener;
use wasm_bindgen::JsCast;
#[derive(Clone, Copy)]
pub(super) struct HeaderTargets {
    pub(super) actions: NodeRef<zxs_web::html::Div>,
    pub(super) navigation: NodeRef<zxs_web::html::Div>,
}

#[component]
pub fn HeaderActions(children: ChildrenFn) -> impl IntoView {
    let target = use_context::<HeaderTargets>().map(|targets| targets.actions);
    move || {
        let children = children.clone();
        target.and_then(|r| r.get()).map(|element| {
            View::new({
                #[allow(unreachable_code)]
                #[allow(unused_mut)]
                #[allow(clippy::let_and_return, clippy::let_unit_value, clippy::unit_arg)]
                component_view(
                    #[allow(clippy::needless_borrows_for_generic_args)]
                    &Portal,
                    {
                        let mut props = component_props_builder(&Portal)
                            .mount(
                                #[allow(unused_braces)]
                                {
                                    { element.unchecked_into::<web_sys::Element>() }
                                },
                            )
                            .children({
                                ::zxs_web::children::ToChildren::to_children(move || {
                                    IntoRender::into_render(children())
                                })
                            })
                            .build();
                        props
                    },
                )
            })
        })
    }
}
#[component]
pub fn HeaderNavigation(
    children: ChildrenFn,
    #[prop(default=t("实例工作区", "Instance workspace"))] label: String,
) -> impl IntoView {
    let label = StoredValue::new(label);
    let target = use_context::<HeaderTargets>().map(|targets| targets.navigation);
    move || {
        let children = children.clone();
        target.and_then(|r| r.get()).map(|element| {
            View::new({
                #[allow(unreachable_code)]
                #[allow(unused_mut)]
                #[allow(clippy::let_and_return, clippy::let_unit_value, clippy::unit_arg)]
                component_view(
                    #[allow(clippy::needless_borrows_for_generic_args)]
                    &Portal,
                    {
                        let mut props = component_props_builder(&Portal)
                            .mount(
                                #[allow(unused_braces)]
                                {
                                    { element.unchecked_into::<web_sys::Element>() }
                                },
                            )
                            .children({
                                ::zxs_web::children::ToChildren::to_children(move || {
                                    zxs_web::html::nav()
                            .child(
                                #[allow(unused_braces)]
                                {
                                    IntoRender::into_render(children())
                                },
                            )
                            .class("zxs-header-navigation zxs-flow")
                            .aria_label(
                                ::zxs_web::prelude::IntoAttributeValue::into_attribute_value(
                                    label.get_value(),
                                ),
                            )
                                })
                            })
                            .build();
                        props
                    },
                )
            })
        })
    }
}

#[component]
pub fn ThemeToggle() -> impl IntoView {
    let current = RwSignal::new(theme::current());
    if let Some(window) = web_sys::window() {
        let listener = EventListener::new(&window, "zxs-theme-change", move |_| {
            let _ = current.try_set(theme::current());
        });
        let listener = send_wrapper::SendWrapper::new(listener);
        on_cleanup(move || drop(listener));
    }
    View::new(
    zxs_web::html::button()
        .child(
            #[allow(unused_braces)]
            {
                zxs_web::svg::svg()
                    .child(
                        #[allow(unused_braces)]
                        {
                            zxs_web::svg::path()
                                .attr(
                                    "d",
                                    ::zxs_web::prelude::IntoAttributeValue::into_attribute_value(move ||
                                    {
                                        if current.get() == "light" {
                                            "M20.8 13A9 9 0 0 1 11 3.2 9 9 0 1 0 20.8 13Z"
                                        } else {
                                            "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.4 1.4m11.2 11.2L19 19M5 19l1.4-1.4M17.6 6.4 19 5"
                                        }
                                    }),
                                )
                        },
                    )
                    .attr("aria-hidden", "true")
                    .attr("width", "1em")
                    .attr("height", "1em")
                    .attr("viewBox", "0 0 24 24")
                    .attr("fill", "none")
                    .attr("stroke", "currentColor")
                    .attr("stroke-width", "1.7")
            },
        )
        .class("zxs-button")
        .r#type("button")
        .title(
            ::zxs_web::prelude::IntoAttributeValue::into_attribute_value(move || {
                if current.get() == "light" {
                    t("切换到深色模式", "Switch to dark mode")
                } else {
                    t("切换到浅色模式", "Switch to light mode")
                }
            }),
        )
        .aria_label(
            ::zxs_web::prelude::IntoAttributeValue::into_attribute_value(move || {
                if current.get() == "light" {
                    t("切换到深色模式", "Switch to dark mode")
                } else {
                    t("切换到浅色模式", "Switch to light mode")
                }
            }),
        )
        .on(zxs_web::ev::click, move |_| theme::toggle()),
)
}

#[component]
pub fn LanguageToggle() -> impl IntoView {
    let label = t("切换为英文", "Switch to Chinese");
    View::new(
    zxs_web::html::button()
        .child(
            #[allow(unused_braces)]
            {
                zxs_web::svg::svg()
                    .child(
                        #[allow(unused_braces)]
                        {
                            zxs_web::svg::path()
                                .attr(
                                    "d",
                                    "M3 5h12M9 3v2M6 5c0 5 4 9 8 11M13 5c0 5-4 9-9 12m10 4 4-10 4 10m-6.5-4h5",
                                )
                        },
                    )
                    .attr("aria-hidden", "true")
                    .attr("width", "1em")
                    .attr("height", "1em")
                    .attr("viewBox", "0 0 24 24")
                    .attr("fill", "none")
                    .attr("stroke", "currentColor")
                    .attr("stroke-width", "1.7")
            },
        )
        .class("zxs-button")
        .r#type("button")
        .aria_label(
            ::zxs_web::prelude::IntoAttributeValue::into_attribute_value(
                label.clone(),
            ),
        )
        .title(::zxs_web::prelude::IntoAttributeValue::into_attribute_value(label))
        .on(zxs_web::ev::click, move |_| switch_language()),
)
}
