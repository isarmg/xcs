use super::ErrorState;
use crate::component::{component_props_builder, component_view};
use crate::{i18n::t, prelude::*};
#[component]
pub fn LoginPage(
    pending: Signal<bool>,
    remote_failure: Signal<Option<super::Feedback>>,
    on_submit: Callback<super::LoginCredentials>,
) -> impl IntoView {
    let username = NodeRef::<xcs_web::html::Input>::new();
    let password = NodeRef::<xcs_web::html::Input>::new();
    let failure = RwSignal::new(None::<(String, Option<String>)>);
    Effect::new(move |_| {
        if let Some(error) = remote_failure.get() {
            failure.set(Some((error.message, error.request_id)));
            if let Some(secret) = password.get_untracked() {
                secret.set_value("");
                let _ = secret.focus();
            }
        }
    });
    on_cleanup(move || {
        if let Some(secret) = password.get_untracked() {
            secret.set_value("");
        }
    });
    View::new(
        xcs_web::html::form()
            .child((
                xcs_web::html::h1().child(
                    #[allow(unused_braces)]
                    {
                        IntoRender::into_render({ t("登录", "Sign in") })
                    },
                ),
                xcs_web::html::label()
                    .child((
                        xcs_web::html::span().child(
                            #[allow(unused_braces)]
                            {
                                IntoRender::into_render({ t("用户名", "Username") })
                            },
                        ),
                        xcs_web::html::input()
                            .class("xcs-input")
                            .node_ref(username)
                            .name("username")
                            .autocomplete("username")
                            .required(true)
                            .maxlength("64")
                            .readonly(
                                ::xcs_web::prelude::IntoAttributeValue::into_attribute_value(
                                    move || pending.get(),
                                ),
                            ),
                    ))
                    .class("xcs-form-field"),
                xcs_web::html::label()
                    .child((
                        xcs_web::html::span().child(
                            #[allow(unused_braces)]
                            {
                                IntoRender::into_render({ t("密码", "Password") })
                            },
                        ),
                        xcs_web::html::input()
                            .class("xcs-input")
                            .node_ref(password)
                            .name("password")
                            .r#type("password")
                            .autocomplete("current-password")
                            .required(true)
                            .maxlength("1024")
                            .readonly(
                                ::xcs_web::prelude::IntoAttributeValue::into_attribute_value(
                                    move || pending.get(),
                                ),
                            ),
                    ))
                    .class("xcs-form-field"),
                IntoRender::into_render({
                    move || {
                        failure.get().map(|(message, id)| {
                            View::new({
                                #[allow(unreachable_code)]
                                #[allow(unused_mut)]
                                #[allow(
                                    clippy::let_and_return,
                                    clippy::let_unit_value,
                                    clippy::unit_arg
                                )]
                                component_view(
                                    #[allow(clippy::needless_borrows_for_generic_args)]
                                    &ErrorState,
                                    {
                                        let mut props = component_props_builder(&ErrorState)
                                            .message(
                                                #[allow(unused_braces)]
                                                {
                                                    message
                                                },
                                            )
                                            .request_id(
                                                #[allow(unused_braces)]
                                                {
                                                    id
                                                },
                                            )
                                            .build();
                                        props
                                    },
                                )
                            })
                        })
                    }
                }),
                xcs_web::html::button()
                    .child(
                        #[allow(unused_braces)]
                        {
                            IntoRender::into_render({
                                move || {
                                    if pending.get() {
                                        t("正在登录…", "Signing in…")
                                    } else {
                                        t("登录", "Sign in")
                                    }
                                }
                            })
                        },
                    )
                    .class("xcs-button")
                    .r#type("submit")
                    .disabled(
                        ::xcs_web::prelude::IntoAttributeValue::into_attribute_value(move || {
                            pending.get()
                        }),
                    ),
            ))
            .novalidate(true)
            .aria_busy(
                ::xcs_web::prelude::IntoAttributeValue::into_attribute_value(move || {
                    pending.get().to_string()
                }),
            )
            .on(xcs_web::ev::input, move |_| failure.set(None))
            .on(xcs_web::ev::submit, move |event| {
                event.prevent_default();
                if pending.get_untracked() {
                    return;
                }
                let name = username.get_untracked().unwrap();
                let secret = password.get_untracked().unwrap();
                if name.value().is_empty() {
                    failure.set(Some((t("请输入用户名。", "Enter your username."), None)));
                    let _ = name.focus();
                    return;
                }
                if secret.value().is_empty() {
                    failure.set(Some((t("请输入密码。", "Enter your password."), None)));
                    let _ = secret.focus();
                    return;
                }
                failure.set(None);
                let credentials = super::LoginCredentials {
                    username: name.value(),
                    password: secret.value(),
                };
                secret.set_value("");
                on_submit.run(credentials);
            }),
    )
}
