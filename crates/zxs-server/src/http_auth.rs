//! HTTP authentication admission and account endpoints.
//!
//! Credential algorithms come from `zxs-auth` through the compatibility module;
//! persistent account/session state remains in `auth::AuthStore`.
use crate::{ApiError, AppState, ServerConfig, auth::Identity, auth_primitives, lock};
use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, Method, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Mutex,
    time::{Duration, Instant},
};

const ATTEMPT_WINDOW: Duration = Duration::from_secs(60);
const MAX_ATTEMPT_KEYS: usize = 4096;
const MAX_ATTEMPTS: u32 = 10;

pub(crate) struct Attempts {
    since: Instant,
    count: u32,
}

fn login_username_key(username: &str) -> String {
    // Rate limiting must identify the same account as password verification,
    // including the canonical policy's accepted outer ASCII spaces.
    let canonical = auth_primitives::normalize_administrator_username(username)
        .unwrap_or_else(|_| username.to_ascii_lowercase());
    format!("user:{canonical}")
}

fn admit_attempts(
    attempts: &Mutex<HashMap<String, Attempts>>,
    keys: [String; 2],
    capacity_message: &str,
    attempt_message: &str,
) -> Result<(), ApiError> {
    let mut attempts = lock(attempts)?;
    attempts.retain(|_, attempt| attempt.since.elapsed() < ATTEMPT_WINDOW);
    for key in keys {
        if !attempts.contains_key(&key) && attempts.len() >= MAX_ATTEMPT_KEYS {
            return Err(ApiError::limit(capacity_message));
        }
        let attempt = attempts.entry(key).or_insert(Attempts {
            since: Instant::now(),
            count: 0,
        });
        attempt.count = attempt.count.saturating_add(1);
        if attempt.count > MAX_ATTEMPTS {
            return Err(ApiError::limit(attempt_message));
        }
    }
    Ok(())
}

fn same_origin(headers: &HeaderMap, config: &ServerConfig, required: bool) -> Result<(), ApiError> {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return if required {
            Err(ApiError::forbidden())
        } else {
            Ok(())
        };
    };
    if headers.get_all(header::ORIGIN).iter().count() != 1 {
        return Err(ApiError::forbidden());
    }
    let origin = origin.to_str().map_err(|_| ApiError::forbidden())?;
    let expected = match &config.external_origin {
        Some(origin) => origin.clone(),
        None => {
            let host = headers
                .get(header::HOST)
                .and_then(|host| host.to_str().ok())
                .ok_or_else(ApiError::forbidden)?;
            let parsed =
                url::Url::parse(&format!("http://{host}")).map_err(|_| ApiError::forbidden())?;
            if !matches!(
                parsed.host_str(),
                Some("localhost") | Some("127.0.0.1") | Some("[::1]") | Some("::1")
            ) {
                return Err(ApiError::forbidden());
            }
            format!("http://{host}")
        }
    };
    if origin != expected {
        return Err(ApiError::forbidden());
    }
    if let Some(site) = headers.get("sec-fetch-site") {
        if site != "same-origin" {
            return Err(ApiError::forbidden());
        }
    }
    Ok(())
}

fn request_token(headers: &HeaderMap) -> Result<(String, bool), ApiError> {
    if let Some(value) = headers.get(header::AUTHORIZATION) {
        if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
            return Err(ApiError::unauthorized());
        }
        let value = value.to_str().map_err(|_| ApiError::unauthorized())?;
        return Ok((
            value
                .strip_prefix("Bearer ")
                .ok_or_else(ApiError::unauthorized)?
                .to_owned(),
            false,
        ));
    }
    if headers.get_all(header::COOKIE).iter().count() != 1 {
        return Err(ApiError::unauthorized());
    }
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|cookie| cookie.to_str().ok())
        .ok_or_else(ApiError::unauthorized)?;
    Ok((
        auth_primitives::parse_cookie_value(cookie, "zxs_session")
            .ok_or_else(ApiError::unauthorized)?
            .to_owned(),
        true,
    ))
}

pub(crate) async fn authorize(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let (token, cookie) = request_token(request.headers())?;
    let changing = !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    same_origin(request.headers(), &state.config, cookie && changing)?;
    if cookie && changing {
        let csrf = request
            .headers()
            .get("x-csrf-token")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(ApiError::forbidden)?;
        if request.headers().get_all("x-csrf-token").iter().count() != 1
            || !auth_primitives::token_matches_hash(
                csrf,
                &auth_primitives::token_hash(
                    &auth_primitives::derive_csrf_token(&token)
                        .map_err(|_| ApiError::forbidden())?,
                ),
            )
        {
            return Err(ApiError::forbidden());
        }
    }
    let identity = lock(&state.auth)?.authenticate(&token)?;
    request.extensions_mut().insert(identity);
    request.extensions_mut().insert(SessionToken(token));
    Ok(next.run(request).await)
}
#[derive(Clone)]
pub(crate) struct SessionToken(String);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Login {
    username: String,
    password: String,
}
pub(crate) async fn login(
    State(state): State<AppState>,
    request: Request,
) -> Result<Response, ApiError> {
    same_origin(request.headers(), &state.config, false)?;
    let ip = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));
    let bytes = axum::body::to_bytes(request.into_body(), 4096)
        .await
        .map_err(ApiError::invalid)?;
    let body: Login = zxs_contract::decode_strict(&bytes).map_err(ApiError::invalid)?;
    admit_attempts(
        &state.attempts,
        [format!("ip:{ip}"), login_username_key(&body.username)],
        "login admission capacity reached",
        "too many login attempts; retry after 60 seconds",
    )?;
    let permit = state
        .password_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::limit("password verification is busy"))?;
    let auth = state.auth.clone();
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        lock(&auth)?.login(&body.username, &body.password)
    })
    .await
    .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "authentication task failed"))??;
    let cookie = format!(
        "zxs_session={}; Path=/v1; HttpOnly; SameSite=Strict; Max-Age={}{}",
        result.token,
        crate::auth::SESSION_TTL_SECONDS,
        if state.config.secure_cookie() {
            "; Secure"
        } else {
            ""
        }
    );
    let mut response = Json(result).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(ApiError::invalid)?,
    );
    Ok(response)
}

pub(crate) async fn me(
    axum::Extension(identity): axum::Extension<Identity>,
    axum::Extension(token): axum::Extension<SessionToken>,
) -> Result<Json<Value>, ApiError> {
    let csrf_token = auth_primitives::derive_csrf_token(&token.0).map_err(ApiError::invalid)?;
    Ok(Json(
        json!({"principal":identity.principal,"admin":identity.admin,"repositories":identity.repositories,"csrf_token":csrf_token}),
    ))
}
pub(crate) async fn logout(
    State(state): State<AppState>,
    axum::Extension(token): axum::Extension<SessionToken>,
) -> Result<Response, ApiError> {
    let auth = state.auth.clone();
    tokio::task::spawn_blocking(move || lock(&auth)?.logout(&token.0))
        .await
        .map_err(ApiError::invalid)??;
    let mut response = Json(json!({"logged_out":true})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("zxs_session=; Path=/v1; HttpOnly; SameSite=Strict; Max-Age=0"),
    );
    Ok(response)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PasswordChange {
    current_password: String,
    new_password: String,
}

pub(crate) async fn change_password(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<Identity>,
    axum::Extension(token): axum::Extension<SessionToken>,
    request: Request,
) -> Result<Response, ApiError> {
    let ip = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));
    let body: PasswordChange = zxs_contract::decode_strict(
        &axum::body::to_bytes(request.into_body(), 16 * 1024)
            .await
            .map_err(ApiError::invalid)?,
    )
    .map_err(ApiError::invalid)?;
    admit_attempts(
        &state.attempts,
        [
            format!("password:ip:{ip}"),
            format!("password:user:{}", identity.principal),
        ],
        "password admission capacity reached",
        "too many password changes; retry after 60 seconds",
    )?;
    let permit = state
        .password_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::limit("password verification is busy"))?;
    let auth = state.auth.clone();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        lock(&auth)?.change_password(&token.0, &body.current_password, &body.new_password)
    })
    .await
    .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "password task failed"))??;
    let mut response =
        Json(json!({"changed":true,"reauthentication_required":true})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("zxs_session=; Path=/v1; HttpOnly; SameSite=Strict; Max-Age=0"),
    );
    Ok(response)
}
pub(crate) async fn accounts(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<Identity>,
) -> Result<Json<Value>, ApiError> {
    identity.require_admin()?;
    Ok(Json(json!({"accounts":lock(&state.auth)?.list_accounts()})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountUpdate {
    username: String,
    password: Option<String>,
    #[serde(default)]
    admin: bool,
    #[serde(default)]
    repositories: std::collections::BTreeSet<String>,
    #[serde(default)]
    disabled: bool,
}
pub(crate) async fn account_upsert(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<Identity>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    identity.require_admin()?;
    let body: AccountUpdate = zxs_contract::decode_strict(
        &axum::body::to_bytes(request.into_body(), 8192)
            .await
            .map_err(ApiError::invalid)?,
    )
    .map_err(ApiError::invalid)?;
    let auth = state.auth.clone();
    let permit = state
        .password_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::limit("password verification is busy"))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        lock(&auth)?.upsert_account(
            &body.username,
            body.password.as_deref(),
            body.admin,
            body.repositories,
            body.disabled,
        )
    })
    .await
    .map_err(ApiError::invalid)??;
    Ok(Json(json!({"updated":true})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_username_variants_share_one_attempt_budget_across_ips() {
        let attempts = Mutex::new(HashMap::new());
        for index in 0..MAX_ATTEMPTS {
            let username = if index % 2 == 0 { " Admin " } else { "ADMIN" };
            admit_attempts(
                &attempts,
                [format!("ip:{index}"), login_username_key(username)],
                "capacity",
                "attempts",
            )
            .unwrap();
        }
        assert_eq!(
            admit_attempts(
                &attempts,
                ["ip:new".into(), login_username_key("admin")],
                "capacity",
                "attempts"
            )
            .unwrap_err()
            .status,
            429
        );
        assert_eq!(attempts.lock().unwrap()["user:admin"].count, 11);
    }

    #[test]
    fn expired_attempts_are_removed_and_key_capacity_is_never_exceeded() {
        let attempts = Mutex::new(HashMap::from([(
            "expired".into(),
            Attempts {
                since: Instant::now() - ATTEMPT_WINDOW,
                count: MAX_ATTEMPTS,
            },
        )]));
        admit_attempts(
            &attempts,
            ["ip:one".into(), "user:one".into()],
            "capacity",
            "attempts",
        )
        .unwrap();
        assert!(!attempts.lock().unwrap().contains_key("expired"));
        {
            let mut values = attempts.lock().unwrap();
            for index in values.len()..MAX_ATTEMPT_KEYS - 1 {
                values.insert(
                    format!("existing:{index}"),
                    Attempts {
                        since: Instant::now(),
                        count: 1,
                    },
                );
            }
        }
        let error = admit_attempts(
            &attempts,
            ["new:one".into(), "new:two".into()],
            "capacity",
            "attempts",
        )
        .unwrap_err();
        assert_eq!(error.status, 429);
        assert_eq!(attempts.lock().unwrap().len(), MAX_ATTEMPT_KEYS);
    }
}
