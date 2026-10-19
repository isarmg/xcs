use super::{
    header::HeaderTargets,
    notification::{Toasts, push_toast},
    *,
};
use crate::{
    component::{component_props_builder, component_view},
    i18n::t,
    prelude::*,
};
use gloo_events::EventListener;
use wasm_bindgen::JsCast;

#[component]
pub fn AppShell(
    product: &'static str,
    session: Signal<SessionView>,
    pending_logout: Signal<bool>,
    on_logout: Callback<()>,
    on_account: Callback<()>,
    on_restore: Callback<()>,
    #[prop(optional)] connection_options: Option<ChildrenFn>,
    login: ChildrenFn,
    children: ChildrenFn,
) -> impl IntoView {
    let toasts = Toasts {
        records: RwSignal::new(Vec::new()),
        sequence: RwSignal::new(0),
    };
    provide_context(toasts);
    let targets = HeaderTargets {
        actions: NodeRef::new(),
        navigation: NodeRef::new(),
    };
    provide_context(targets);
    if let Some(window) = web_sys::window() {
        let listener = send_wrapper::SendWrapper::new(EventListener::new(
            &window,
            "xcs-notify",
            move |event| {
                if let Some(message) = event
                    .dyn_ref::<web_sys::CustomEvent>()
                    .and_then(|event| event.detail().as_string())
                {
                    push_toast(toasts, message);
                }
            },
        ));
        on_cleanup(move || drop(listener));
    }
    Effect::new(move |_| {
        if session.get().phase != SessionPhase::Authenticated {
            toasts.records.set(Vec::new());
        }
    });
    IntoRender::into_render(move || {
        let current = session.get();
        if current.phase == SessionPhase::Loading {
            return view!{<div class="xcs-application-boot" role="status" aria-busy="true" aria-label=t("正在准备应用…","Preparing application…")></div>}.into_any();
        }
        if current.phase != SessionPhase::Authenticated {
            let login = login.clone();
            return view! {
                <div class="xcs-auth-shell" data-xcs-scope="">
                    <div class="xcs-auth-language">
                        <div class="xcs-login-controls" role="group" aria-label=t("显示设置","Display settings")><LanguageToggle/><ThemeToggle/></div>
                    </div>
                    <div class="xcs-auth-content">
                        {connection_options.as_ref().map(|options|options())}
                        {(!current.message.is_empty() && current.phase==SessionPhase::Anonymous).then(||view!{<p role="status">{current.message.clone()}</p>})}
                        {(current.phase==SessionPhase::LogoutUnconfirmed).then(||view!{<div><ErrorState message=t("退出请求尚未确认，当前界面已锁定。请重试注销服务器会话。","Sign out is unconfirmed and this view is locked. Retry server session revocation.")/><button class="xcs-button" on:click=move |_|on_logout.run(())>{t("重试退出","Retry sign out")}</button></div>})}
                        <div class="xcs-auth-card">
                            <div class="xcs-product-identity"><strong>{product}</strong></div>
                            {if current.phase==SessionPhase::Error {
                                view!{<div><ErrorState message=current.message request_id=current.request_id/><button class="xcs-button" on:click=move |_|on_restore.run(())>{t("重试","Retry")}</button></div>}.into_any()
                            } else {login().into_any()}}
                        </div>
                    </div>
                </div>
            }.into_any();
        }
        let children = children.clone();
        View::new(
    xcs_web::html::div()
        .child((
            xcs_web::html::a()
                .child(
                    #[allow(unused_braces)]
                    {
                        IntoRender::into_render({
                            t("跳至正文", "Skip to content")
                        })
                    },
                )
                .class("xcs-skip-link")
                .href("#xcs-main-content")
                .on(
                    xcs_web::ev::click,
                    move |event| {
                        event.prevent_default();
                        if let Some(main) = web_sys::window()
                            .unwrap()
                            .document()
                            .unwrap()
                            .get_element_by_id("xcs-main-content")
                            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                        {
                            let _ = main.focus();
                        }
                    },
                ),
            xcs_web::html::header()
                .child((
                    xcs_web::html::div()
                        .child(
                            #[allow(unused_braces)]
                            {
                                xcs_web::html::div()
                                    .child((
                                        xcs_web::html::div()
                                            .child(
                                                #[allow(unused_braces)]
                                                {
                                                    xcs_web::html::strong()
                                                        .child(
                                                            #[allow(unused_braces)]
                                                            { IntoRender::into_render({ product }) },
                                                        )
                                                },
                                            )
                                            .class("xcs-product-identity xcs-row"),
                                        xcs_web::html::div()
                                            .node_ref(targets.navigation)
                                            .style(("display", "contents")),
                                    ))
                                    .class("xcs-header-brand-navigation xcs-flow")
                            },
                        )
                        .class("xcs-header-navigation-slot"),
                    xcs_web::html::div()
                        .child((
                            xcs_web::html::div()
                                .node_ref(targets.actions)
                                .style(("display", "contents")),
                            {
                                #[allow(unreachable_code)] #[allow(unused_mut)]
                                #[allow(clippy::let_and_return, clippy::let_unit_value, clippy::unit_arg)]
                                component_view(
                                    #[allow(clippy::needless_borrows_for_generic_args)]
                                    &LanguageToggle,
                                    {
                                        let mut props = component_props_builder(
                                                &LanguageToggle,
                                            )
                                            .build();
                                        props
                                    },
                                )
                            },
                            {
                                #[allow(unreachable_code)] #[allow(unused_mut)]
                                #[allow(clippy::let_and_return, clippy::let_unit_value, clippy::unit_arg)]
                                component_view(
                                    #[allow(clippy::needless_borrows_for_generic_args)]
                                    &ThemeToggle,
                                    {
                                        let mut props = component_props_builder(
                                                &ThemeToggle,
                                            )
                                            .build();
                                        props
                                    },
                                )
                            },
                            xcs_web::html::button()
                                .child(
                                    #[allow(unused_braces)]
                                    {
                                        xcs_web::svg::svg()
                                            .child(
                                                #[allow(unused_braces)]
                                                {
                                                    xcs_web::svg::path()
                                                        .attr("d", "M9 4H4v16h5m5-12 4 4-4 4M8 12h12")
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
                                .class("xcs-button")
                                .disabled(
                                    ::xcs_web::prelude::IntoAttributeValue::into_attribute_value(move ||
                                    pending_logout.get()),
                                )
                                .aria_label(
                                    ::xcs_web::prelude::IntoAttributeValue::into_attribute_value(move ||
                                    {
                                        if pending_logout.get() {
                                            t("正在退出…", "Signing out…")
                                        } else {
                                            t("退出", "Sign out")
                                        }
                                    }),
                                )
                                .title(
                                    ::xcs_web::prelude::IntoAttributeValue::into_attribute_value(
                                        t("退出", "Sign out"),
                                    ),
                                )
                                .on(
                                    xcs_web::ev::click,
                                    move |_| {
                                        if !pending_logout.get_untracked() { on_logout.run(()); }
                                    },
                                ),
                            xcs_web::html::button()
                                .child(
                                    #[allow(unused_braces)]
                                    {
                                        xcs_web::svg::svg()
                                            .child((
                                                xcs_web::svg::circle()
                                                    .attr("cx", "12")
                                                    .attr("cy", "8")
                                                    .attr("r", "3.5"),
                                                xcs_web::svg::path()
                                                    .attr("d", "M4.5 21v-2a7.5 7.5 0 0 1 15 0v2"),
                                            ))
                                            .attr("viewBox", "0 0 24 24")
                                            .attr("width", "1em")
                                            .attr("height", "1em")
                                            .attr("fill", "none")
                                            .attr("stroke", "currentColor")
                                            .attr("stroke-width", "1.7")
                                            .attr("aria-hidden", "true")
                                    },
                                )
                                .class("xcs-button xcs-account-entry")
                                .attr("data-xcs-contour", "none")
                                .style(("border-radius", "50%"))
                                .style(("aspect-ratio", "1"))
                                .aria_label(
                                    ::xcs_web::prelude::IntoAttributeValue::into_attribute_value(
                                        t("账号设置", "Account settings"),
                                    ),
                                )
                                .title(
                                    ::xcs_web::prelude::IntoAttributeValue::into_attribute_value(
                                        t("账号设置", "Account settings"),
                                    ),
                                )
                                .on(
                                    xcs_web::ev::click,
                                    move |_| {
                                        on_account.run(());
                                    },
                                ),
                        ))
                        .class("xcs-header-actions xcs-flow xcs-control-group--comfortable")
                        .role("group")
                        .aria_label(
                            ::xcs_web::prelude::IntoAttributeValue::into_attribute_value(
                                t("全局操作", "Global actions"),
                            ),
                        ),
                ))
                .class("xcs-page-header xcs-flow"),
            {
                #[allow(unreachable_code)] #[allow(unused_mut)]
                #[allow(clippy::let_and_return, clippy::let_unit_value, clippy::unit_arg)]
                component_view(
                    #[allow(clippy::needless_borrows_for_generic_args)]
                    &Show,
                    {
                        let mut props = component_props_builder(
                                &Show,
                            )
                            .when(
                                #[allow(unused_braces)]
                                { move || !toasts.records.get().is_empty() },
                            )
                            .children({
                                ::xcs_web::children::ToChildren::to_children(move || {
                                    xcs_web::html::div()
                                        .child(
                                            #[allow(unused_braces)]
                                            {
                                                {
                                                    #[allow(unreachable_code)] #[allow(unused_mut)]
                                                    #[allow(clippy::let_and_return, clippy::let_unit_value, clippy::unit_arg)]
                                                    component_view(
                                                        #[allow(clippy::needless_borrows_for_generic_args)]
                                                        &For,
                                                        {
                                                            let mut props = component_props_builder(
                                                                    &For,
                                                                )
                                                                .each(
                                                                    #[allow(unused_braces)]
                                                                    { move || toasts.records.get() },
                                                                )
                                                                .key(#[allow(unused_braces)] { |r| r.id })
                                                                .children(
                                                                    #[allow(unused_braces)]
                                                                    {
                                                                        move |r| {
                                                                            let id = r.id;
                                                                            View::new(
    xcs_web::html::div()
        .child((
            xcs_web::html::span()
                .child(
                    #[allow(unused_braces)]
                    { IntoRender::into_render({ r.message }) },
                ),
            xcs_web::html::button()
                .child(#[allow(unused_braces)] { "×" })
                .class("xcs-button")
                .aria_label(
                    ::xcs_web::prelude::IntoAttributeValue::into_attribute_value(
                        t("关闭通知", "Dismiss notification"),
                    ),
                )
                .on(
                    xcs_web::ev::click,
                    move |_| toasts.records.update(|v| v.retain(|r| r.id != id)),
                ),
        ))
        .class("xcs-toast xcs-row xcs-flow")
        .role("status")
        .aria_live("polite")
        .aria_atomic("true"),
)
                                                                        }
                                                                    },
                                                                )
                                                                .build();
                                                            props
                                                        },
                                                    )
                                                }
                                            },
                                        )
                                        .class("xcs-toast-stack")
                                        .role("region")
                                        .aria_label(
                                            ::xcs_web::prelude::IntoAttributeValue::into_attribute_value(
                                                t("通知", "Notifications"),
                                            ),
                                        )
                                })
                            })
                            .build();
                        props
                    },
                )
            },
            xcs_web::html::div()
                .child(
                    #[allow(unused_braces)]
                    {
                        xcs_web::html::main()
                            .child(
                                #[allow(unused_braces)]
                                {
                                    IntoRender::into_render(children())
                                },
                            )
                            .class("xcs-shell-main xcs-stack")
                            .id("xcs-main-content")
                            .tabindex("-1")
                    },
                )
                .class("xcs-shell-layout xcs-shell-layout--full"),
        ))
        .class("xcs-app-shell")
        .attr("data-xcs-scope", "")
        .style(("--xcs-header-icon-size", "1em")),
)
            .into_any()
    })
}
