use crate::{api, files, history, icons::FileIcon, monitor};
use serde_json::{Value, json};
use wasm_bindgen_futures::spawn_local;
use xcs_web::components::{
    AccountPage, AppShell, ErrorState, Feedback, HeaderActions, HeaderNavigation, LoginCredentials,
    LoginPage, PasswordChange, SessionPhase, SessionView,
};
use xcs_web::{i18n::t, prelude::*};

#[derive(Clone, Default)]
pub(crate) struct State {
    pub authenticated: bool,
    pub session_epoch: u64,
    pub admin: bool,
    pub principal: String,
    pub restoring: bool,
    pub session_failure: Option<Feedback>,
    pub login_failure: Option<Feedback>,
    pub logout_pending: bool,
    pub logout_unconfirmed: bool,
    pub password_pending: bool,
    pub password_failure: Option<Feedback>,
    pub page: String,
    pub busy: usize,
    pub errors: Vec<Feedback>,
    pub notice: String,
    pub files: files::Model,
    pub monitor: monitor::Model,
    pub activity: history::Model,
}

pub(crate) fn current_session(state: RwSignal<State>, epoch: u64) -> bool {
    state.with_untracked(|s| s.authenticated && !s.logout_pending && s.session_epoch == epoch)
}

pub(crate) fn list(value: &Value, key: &str) -> Vec<Value> {
    value
        .as_array()
        .or_else(|| value.get(key).and_then(Value::as_array))
        .cloned()
        .unwrap_or_default()
}
pub(crate) fn text(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| value.get(key).and_then(Value::as_str))
        .unwrap_or_default()
        .to_owned()
}
pub(crate) fn feedback(error: &api::Error) -> Feedback {
    Feedback {
        message: format!("HTTP {} · {}", error.status, error),
        request_id: error.request_id.clone(),
    }
}
pub(crate) fn fail(state: RwSignal<State>, error: api::Error) {
    state.update(|s| {
        if error.status == 401 {
            s.authenticated = false;
        }
        s.errors.insert(0, feedback(&error));
        s.errors.truncate(5);
    });
}
pub(crate) fn set_notice(state: RwSignal<State>, notice: String) {
    state.update(|s| s.notice = notice);
}

pub fn app() -> impl IntoView {
    let state = RwSignal::new(State {
        page: "files".into(),
        restoring: true,
        ..Default::default()
    });
    restore_session(state);
    let poller = gloo_timers::callback::Interval::new(2_000, move || {
        if state.with_untracked(|s| s.authenticated) {
            files::poll(state);
        }
    });
    let poller = send_wrapper::SendWrapper::new(poller);
    on_cleanup(move || drop(poller));
    let telemetry = gloo_timers::callback::Interval::new(5_000, move || {
        if state.with_untracked(|s| s.authenticated && s.page == "status") {
            monitor::load(state);
        }
    });
    let telemetry = send_wrapper::SendWrapper::new(telemetry);
    on_cleanup(move || drop(telemetry));
    let session = Memo::new(move |_| state.with(session_view));
    let logout_pending = Memo::new(move |_| state.with(|s| s.logout_pending));
    let login_pending = Memo::new(move |_| state.with(|s| s.busy > 0));
    let login_failure = Memo::new(move |_| state.with(|s| s.login_failure.clone()));
    let endpoint = RwSignal::new("https://".to_owned());
    let on_login = Callback::new(move |credentials: LoginCredentials| {
        let address = endpoint.get_untracked();
        state.update(|s| {
            s.busy += 1;
            s.session_epoch += 1;
            s.login_failure = None;
        });
        spawn_local(async move {
            match api::login(&address, &credentials.username, &credentials.password).await {
                Ok(value) => {
                    state.update(|s| {
                        let busy = s.busy;
                        let session_epoch = s.session_epoch;
                        *s = State {
                            session_epoch,
                            authenticated: true,
                            principal: text(&value, &["principal", "username"]),
                            admin: value["admin"].as_bool().unwrap_or(false),
                            page: "files".into(),
                            busy,
                            ..Default::default()
                        };
                    });
                    files::load_disks(state);
                }
                Err(error) => state.update(|s| s.login_failure = Some(feedback(&error))),
            }
            state.update(|s| s.busy = s.busy.saturating_sub(1));
        });
    });
    let login: ChildrenFn = ToChildren::to_children(
        move || view! { <LoginPage pending=login_pending.into() remote_failure=login_failure.into() on_submit=on_login/> },
    );
    let options: ChildrenFn = ToChildren::to_children(move || {
        api::native().then(|| view! {
        <label class="xcs-form-field"><span>{t("服务器地址","Server address")}</span><input class="xcs-input" type="url" prop:value=move ||endpoint.get() on:input=move |event|endpoint.set(event_target_value(&event))/></label>
    })
    });
    view! {
        <AppShell product="xcs" session=session.into() pending_logout=logout_pending.into() on_logout=Callback::new(move |_|logout(state)) on_account=Callback::new(move |_|navigate(state,"account")) on_restore=Callback::new(move |_|restore_session(state)) login=login connection_options=options>
            {workspace(state)}
            <Show when=move ||state.with(|s|!s.notice.is_empty())><p class="xcs-status xcs-row" role="status" aria-live="polite">{move ||state.with(|s|s.notice.clone())}</p></Show>
            <Show when=move ||state.with(|s|!s.errors.is_empty())><aside class="xcs-error-center xcs-stack" aria-label=t("请求错误","Request errors")>{move ||state.with(|s|s.errors.clone()).into_iter().enumerate().map(move |(index,error)|view! {
                <div class="xcs-inline-feedback xcs-flow"><ErrorState message=error.message request_id=error.request_id/><button class="xcs-button" aria-label=t("关闭错误提示","Dismiss error") on:click=move |_|state.update(|s|{if index<s.errors.len(){s.errors.remove(index);}})>"×"</button></div>
            }).collect_view()}</aside></Show>
        </AppShell>
    }
}
fn navigate(state: RwSignal<State>, page: &str) {
    state.update(|s| {
        s.files.close_overlays();
        s.page = page.into();
    });
    match page {
        "status" => monitor::load(state),
        "history" => history::load(state),
        _ => {}
    }
}
fn workspace(state: RwSignal<State>) -> impl IntoView {
    let page = Memo::new(move |_| state.with(|s| s.page.clone()));
    view! {
        <HeaderNavigation label=t("主导航","Main navigation")>
            {[ ("files",t("文件","Files")), ("details",t("详情","Details")), ("status",t("状态","Status")), ("history",t("历史记录","History")) ].into_iter().map(move |(page,title)|view! {
                <button class="xcs-button" aria-current=move ||(state.with(|s|s.page==page)).then_some("page") on:click=move |_|navigate(state,page)>{title}</button>
            }).collect_view()}
        </HeaderNavigation>
        <HeaderActions><div class="xcs-header-tools xcs-flow">
            <Show when=move ||state.with(|s|matches!(s.page.as_str(),"files"|"details"|"status"|"history"))>
                <button class="xcs-button xcs-header-refresh" type="button" aria-label=t("刷新","Refresh") title=t("刷新","Refresh") disabled=move ||state.with(|s|match s.page.as_str(){"files"=>s.files.loading||s.files.loading_disks,"details"=>s.files.loading||s.files.history_loading||s.files.selected.is_none(),"status"=>s.monitor.loading||!s.admin,"history"=>s.activity.loading,_=>true}) on:click=move |_|match state.with_untracked(|s|s.page.clone()).as_str(){"files"|"details"=>if state.with_untracked(|s|s.files.slot.is_empty()){files::load_disks(state)}else{files::load_entries(state)},"status"=>monitor::load(state),"history"=>history::load(state),_=>{}}><FileIcon name="refresh"/></button>
            </Show>
            <span class="xcs-session-identity">{move ||state.with(|s|s.principal.clone())}</span>
        </div></HeaderActions>
        {move ||match page.get().as_str() { "details"=>files::details(state).into_any(),"status"=>monitor::view(state).into_any(),"history"=>history::view(state).into_any(),"account"=>account(state).into_any(),_=>files::view(state).into_any() }}
        {files::overlays(state)}
    }
}
fn session_view(state: &State) -> SessionView {
    SessionView {
        phase: if state.restoring {
            SessionPhase::Loading
        } else if state.authenticated {
            SessionPhase::Authenticated
        } else if state.logout_unconfirmed {
            SessionPhase::LogoutUnconfirmed
        } else if state.session_failure.is_some() {
            SessionPhase::Error
        } else {
            SessionPhase::Anonymous
        },
        username: state.principal.clone(),
        admin: state.admin,
        message: state
            .session_failure
            .as_ref()
            .map(|e| e.message.clone())
            .unwrap_or_else(|| {
                if state.authenticated {
                    String::new()
                } else {
                    state.notice.clone()
                }
            }),
        request_id: state
            .session_failure
            .as_ref()
            .and_then(|e| e.request_id.clone()),
    }
}
fn restore_session(state: RwSignal<State>) {
    state.update(|s| {
        s.restoring = true;
        s.session_epoch += 1;
        s.session_failure = None;
    });
    spawn_local(async move {
        match api::restore().await {
            Ok(value) => {
                state.update(|s| {
                    s.authenticated = true;
                    s.principal = text(&value, &["principal", "username"]);
                    s.admin = value["admin"].as_bool().unwrap_or(false);
                });
                files::load_disks(state);
            }
            Err(error) => state.update(|s| {
                s.authenticated = false;
                if error.status != 401 {
                    s.session_failure = Some(feedback(&error));
                }
            }),
        }
        state.update(|s| s.restoring = false);
    });
}
fn logout(state: RwSignal<State>) {
    if state.with_untracked(|s| s.logout_pending) {
        return;
    }
    state.update(|s| {
        s.logout_pending = true;
        s.session_epoch += 1;
    });
    spawn_local(async move {
        match api::request("POST", "/v1/auth/logout", Some(json!({}))).await {
            Ok(_) => {
                end_session(state, String::new());
            }
            Err(error) if error.status == 401 => {
                end_session(state, String::new());
            }
            Err(error) => state.update(|s| {
                s.authenticated = false;
                s.logout_unconfirmed = true;
                s.logout_pending = false;
                s.session_failure = Some(feedback(&error));
            }),
        }
    });
}
fn end_session(state: RwSignal<State>, notice: String) {
    api::clear_session();
    let session_epoch = state.with_untracked(|s| s.session_epoch + 1);
    state.set(State {
        session_epoch,
        page: "files".into(),
        notice,
        ..Default::default()
    });
}
fn account(state: RwSignal<State>) -> impl IntoView {
    let username = Memo::new(move |_| state.with(|s| s.principal.clone()));
    let pending = Memo::new(move |_| state.with(|s| s.password_pending));
    let failure = Memo::new(move |_| state.with(|s| s.password_failure.clone()));
    let change = Callback::new(move |credentials: PasswordChange| {
        state.update(|s| {
            s.password_pending = true;
            s.password_failure = None;
        });
        let epoch = state.with_untracked(|s| s.session_epoch);
        spawn_local(async move {
            let result = api::request(
                "POST",
                "/v1/auth/password",
                Some(json!({
                    "current_password": credentials.current_password,
                    "new_password": credentials.new_password,
                })),
            )
            .await;
            if !current_session(state, epoch) {
                return;
            }
            match result {
                Ok(value)
                    if value["changed"] == true && value["reauthentication_required"] == true =>
                {
                    end_session(
                        state,
                        t(
                            "密码已更新，请使用新密码登录。",
                            "Your password was changed. Sign in with the new password.",
                        ),
                    );
                }
                Ok(_) => state.update(|s| {
                    s.password_pending = false;
                    s.password_failure = Some(
                        t(
                            "未收到有效的改密确认，请核对会话状态。",
                            "The password change was not confirmed. Check your session.",
                        )
                        .as_str()
                        .into(),
                    );
                }),
                Err(error) => state.update(|s| reject_password_change(s, &error)),
            }
        });
    });
    view! { <section class="xcs-content-stack xcs-stack"><p class="xcs-row">{t("修改密码后，该账号的全部会话将失效。","Changing the password signs this account out on all devices.")}</p><AccountPage username_value=username.into() pending=pending.into() remote_failure=failure.into() on_submit=change/></section> }
}
fn reject_password_change(state: &mut State, error: &api::Error) {
    state.password_pending = false;
    state.password_failure = Some(feedback(error));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn content_and_account_completion_require_the_original_active_session() {
        Owner::new().with(|| {
            let state = RwSignal::new(State {
                authenticated: true,
                session_epoch: 4,
                ..Default::default()
            });
            assert!(current_session(state, 4));
            state.update(|s| s.logout_pending = true);
            assert!(!current_session(state, 4));
            state.update(|s| {
                s.logout_pending = false;
                s.session_epoch = 5;
            });
            assert!(!current_session(state, 4));
            assert!(current_session(state, 5));
            end_session(state, "Password changed".into());
            assert!(!current_session(state, 5));
            state.with_untracked(|s| {
                assert_eq!(s.session_epoch, 6);
                assert_eq!(s.notice, "Password changed");
                assert!(!s.authenticated);
                assert!(s.files.uploads.is_empty());
            });
        });
    }
    #[test]
    fn shell_identity_does_not_change_with_file_progress() {
        let mut state = State {
            authenticated: true,
            principal: "writer".into(),
            ..Default::default()
        };
        let session = session_view(&state);
        state.busy = 3;
        state.password_pending = true;
        state.notice = "uploading".into();
        assert_eq!(session_view(&state), session);
        assert_eq!(session.phase, SessionPhase::Authenticated);
    }
    #[test]
    fn wrong_password_does_not_discard_pending_file_saves() {
        let mut state = State {
            authenticated: true,
            password_pending: true,
            ..Default::default()
        };
        state.files.slot = "P01".into();
        reject_password_change(
            &mut state,
            &api::Error {
                code: "CURRENT_PASSWORD_INVALID".into(),
                message: "Incorrect password".into(),
                status: 403,
                retryable: false,
                request_id: Some("req_test".into()),
            },
        );
        assert!(state.authenticated);
        assert_eq!(state.files.slot, "P01");
        assert!(!state.password_pending);
        assert_eq!(
            state.password_failure.unwrap().request_id.as_deref(),
            Some("req_test")
        );
    }
}
