use crate::{i18n::t, prelude::*};
#[component]
pub fn LoadingState() -> impl IntoView {
    View::new(
        xcs_web::html::div()
            .child(
                #[allow(unused_braces)]
                {
                    IntoRender::into_render({ t("正在加载…", "Loading…") })
                },
            )
            .class("xcs-loading xcs-row")
            .role("status")
            .aria_live("polite"),
    )
}
#[component]
pub fn EmptyState(message: String) -> impl IntoView {
    View::new(
        xcs_web::html::div()
            .child(
                #[allow(unused_braces)]
                {
                    IntoRender::into_render(message)
                },
            )
            .class("xcs-empty xcs-row"),
    )
}
#[component]
pub fn ErrorState(
    message: String,
    #[prop(optional_no_strip)] request_id: Option<String>,
) -> impl IntoView {
    View::new(
        xcs_web::html::div()
            .child((
                IntoRender::into_render(message),
                IntoRender::into_render({
                    request_id.filter(|v| super::safe_request_id(v)).map(|id| {
                        View::new(
                            xcs_web::html::p()
                                .child((
                                    IntoRender::into_render({
                                        t("请求标识：", "Request ID: ")
                                    }),
                                    xcs_web::html::code().child(
                                        #[allow(unused_braces)]
                                        {
                                            IntoRender::into_render({ id })
                                        },
                                    ),
                                ))
                                .class("xcs-request-id"),
                        )
                    })
                }),
            ))
            .class("xcs-error xcs-row")
            .role("alert"),
    )
}
