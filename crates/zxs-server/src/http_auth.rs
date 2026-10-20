//! HTTP authentication admission and account endpoints.
//!
//! Credential algorithms come directly from `zxs-auth`;
//! persistent account/session state remains in `auth::AuthStore`.
use crate::{ApiError, AppState, ServerConfig, auth::Identity, lock};
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
    collections::{HashMap, HashSet},
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const ATTEMPT_WINDOW: Duration = Duration::from_secs(60);
const MAX_ATTEMPT_KEYS: usize = 4096;
const MAX_ATTEMPTS: u32 = 10;
const MAX_PENDING_LOGINS: usize = 64;

/// At most one verifier or sleeper per canonical account, without reserving global hash slots.
struct PendingLogin {
    pending: Arc<Mutex<HashSet<String>>>,
    account: String,
}

impl PendingLogin {
    fn acquire(pending: &Arc<Mutex<HashSet<String>>>, username: &str) -> Result<Self, ApiError> {
        let account = login_username_key(username)?;
        let mut active = lock(pending)?;
        if active.len() >= MAX_PENDING_LOGINS || active.contains(&account) {
            return Err(ApiError::limit("login verification is busy"));
        }
        active.insert(account.clone());
        Ok(Self {
            pending: pending.clone(),
            account,
        })
    }
}

impl Drop for PendingLogin {
    fn drop(&mut self) {
        if let Ok(mut active) = self.pending.lock() {
            active.remove(&self.account);
        }
    }
}

pub(crate) struct Attempts {
    since: Instant,
    count: u32,
}

fn login_username_key(username: &str) -> Result<String, ApiError> {
    // Rate limiting must identify the same account as password verification,
    // including the canonical policy's accepted outer ASCII spaces.
    // Validate before retaining any input in pending/counter maps. The shared
    // policy bounds raw candidate bytes before normalization can allocate.
    let canonical = zxs_auth::normalize_administrator_username(username)
        .map_err(|_| ApiError::unauthorized())?;
    Ok(format!("user:{canonical}"))
}

/// Charge all keys atomically. Rejected requests never poison another account's budget.
fn charge_attempts<const N: usize>(
    attempts: &Mutex<HashMap<String, Attempts>>,
    keys: [(String, Option<u32>); N],
    capacity_message: &str,
    attempt_message: &str,
) -> Result<[u32; N], ApiError> {
    let mut attempts = lock(attempts)?;
    attempts.retain(|_, attempt| attempt.since.elapsed() < ATTEMPT_WINDOW);
    let additional = keys
        .iter()
        .filter(|(key, _)| !attempts.contains_key(key))
        .count();
    if attempts.len().saturating_add(additional) > MAX_ATTEMPT_KEYS {
        return Err(ApiError::limit(capacity_message));
    }
    if keys.iter().any(|(key, limit)| {
        limit.is_some_and(|limit| {
            attempts
                .get(key)
                .is_some_and(|attempt| attempt.count >= limit)
        })
    }) {
        return Err(ApiError::limit(attempt_message));
    }
    Ok(keys.map(|(key, limit)| {
        let attempt = attempts.entry(key).or_insert(Attempts {
            since: Instant::now(),
            count: 0,
        });
        attempt.count = attempt.count.saturating_add(1);
        if limit.is_none() {
            // Distributed account pressure cools down only after an idle minute,
            // rather than allowing a new burst at every fixed-window boundary.
            attempt.since = Instant::now();
        }
        attempt.count
    }))
}

fn admit_attempts(
    attempts: &Mutex<HashMap<String, Attempts>>,
    keys: [String; 2],
    capacity_message: &str,
    attempt_message: &str,
) -> Result<(), ApiError> {
    charge_attempts(
        attempts,
        keys.map(|key| (key, Some(MAX_ATTEMPTS))),
        capacity_message,
        attempt_message,
    )?;
    Ok(())
}

fn admit_login(
    attempts: &Mutex<HashMap<String, Attempts>>,
    ip: IpAddr,
    username: &str,
) -> Result<Duration, ApiError> {
    let account = login_username_key(username)?;
    let counts = charge_attempts(
        attempts,
        [
            (format!("ip:{ip}"), Some(MAX_ATTEMPTS)),
            (format!("source:{ip}:{account}"), Some(MAX_ATTEMPTS)),
            (account, None),
        ],
        "login admission capacity reached",
        "too many login attempts; retry after 60 seconds",
    )?;
    // Account-wide pressure slows distributed guesses but cannot hard-lock a victim.
    // A single bounded pending-account guard spans delay and verification.
    // Global password slots are reserved for hashing, never for this sleep.
    let excess = counts[2].saturating_sub(MAX_ATTEMPTS);
    Ok(if excess == 0 {
        Duration::ZERO
    } else {
        Duration::from_millis((250 * (1_u64 << (excess - 1).min(5))).min(6000))
    })
}

fn client_ip(request: &Request, config: &ServerConfig) -> Result<IpAddr, ApiError> {
    let peer = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip());
    // Missing transport metadata must never authenticate a forwarding header.
    let Some(peer) = peer else {
        return if config.trusted_proxy_ips.is_empty() {
            Ok(IpAddr::from([127, 0, 0, 1]))
        } else {
            Err(ApiError::invalid(
                "trusted proxy requires transport peer metadata",
            ))
        };
    };
    if !config.trusted_proxy_ips.contains(&peer) {
        return Ok(peer);
    }
    let values = request.headers().get_all("x-forwarded-for");
    let mut values = values.iter();
    let value = values
        .next()
        .ok_or_else(|| ApiError::invalid("trusted proxy must set X-Forwarded-For"))?;
    if values.next().is_some() {
        return Err(ApiError::invalid(
            "trusted proxy must overwrite X-Forwarded-For with one IP",
        ));
    }
    let ip = value
        .to_str()
        .ok()
        .and_then(|value| value.parse::<IpAddr>().ok())
        .ok_or_else(|| {
            ApiError::invalid("trusted proxy must overwrite X-Forwarded-For with one IP")
        })?;
    // Unify IPv4 and IPv4-mapped IPv6 spellings so they cannot multiply a budget.
    Ok(match ip {
        IpAddr::V6(ip) => ip
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(ip)),
        ip => ip,
    })
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
    if let Some(site) = headers.get("sec-fetch-site")
        && site != "same-origin"
    {
        return Err(ApiError::forbidden());
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
        zxs_auth::parse_cookie_value(cookie, "zxs_session")
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
            || !zxs_auth::token_matches_hash(
                csrf,
                &zxs_auth::token_hash(
                    &zxs_auth::derive_csrf_token(&token).map_err(|_| ApiError::forbidden())?,
                ),
            )
        {
            return Err(ApiError::forbidden());
        }
    }
    let auth = state.auth.clone();
    let credential = token.clone();
    let identity = tokio::task::spawn_blocking(move || lock(&auth)?.authenticate(&credential))
        .await
        .map_err(ApiError::invalid)??;
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
    let ip = client_ip(&request, &state.config)?;
    let bytes = axum::body::to_bytes(request.into_body(), 4096)
        .await
        .map_err(ApiError::invalid)?;
    let body: Login = zxs_contract::decode_strict(&bytes).map_err(ApiError::invalid)?;
    let pending = PendingLogin::acquire(&state.pending_logins, &body.username)?;
    let delay = admit_login(&state.attempts, ip, &body.username)?;
    tokio::time::sleep(delay).await;
    let permit = state
        .password_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::limit("password verification is busy"))?;
    let auth = state.auth.clone();
    let result = tokio::task::spawn_blocking(move || {
        let _pending = pending;
        let _permit = permit;
        lock(&auth)?.login(&body.username, &body.password)
    })
    .await
    .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "authentication task failed"))??;
    let cookie = format!(
        "zxs_session={}; Path=/api; HttpOnly; SameSite=Strict; Max-Age={}{}",
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
    let csrf_token = zxs_auth::derive_csrf_token(&token.0).map_err(ApiError::invalid)?;
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
        HeaderValue::from_static("zxs_session=; Path=/api; HttpOnly; SameSite=Strict; Max-Age=0"),
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
    let ip = client_ip(&request, &state.config)?;
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
        HeaderValue::from_static("zxs_session=; Path=/api; HttpOnly; SameSite=Strict; Max-Age=0"),
    );
    Ok(response)
}
pub(crate) async fn accounts(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<Identity>,
) -> Result<Json<Value>, ApiError> {
    identity.require_admin()?;
    let auth = state.auth.clone();
    let accounts = tokio::task::spawn_blocking(move || {
        let auth = lock(&auth)?;
        auth.identity_for_principal(&identity.principal)?
            .require_admin()?;
        Ok::<_, ApiError>(auth.list_accounts())
    })
    .await
    .map_err(ApiError::invalid)??;
    Ok(Json(json!({"accounts": accounts})))
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
        let mut auth = lock(&auth)?;
        auth.identity_for_principal(&identity.principal)?
            .require_admin()?;
        auth.upsert_account(
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

    fn address(last: u8) -> IpAddr {
        IpAddr::from([192, 0, 2, last])
    }

    #[test]
    fn distributed_normalized_username_attempts_slow_without_hard_lockout() {
        let attempts = Mutex::new(HashMap::new());
        for index in 0..MAX_ATTEMPTS {
            let username = if index % 2 == 0 { " Admin " } else { "ADMIN" };
            assert_eq!(
                admit_login(&attempts, address(index as u8), username).unwrap(),
                Duration::ZERO
            );
        }
        for (index, millis) in [
            (10, 250),
            (11, 500),
            (12, 1000),
            (13, 2000),
            (14, 4000),
            (15, 6000),
            (16, 6000),
        ] {
            assert_eq!(
                admit_login(&attempts, address(index), "admin").unwrap(),
                Duration::from_millis(millis)
            );
        }
        assert_eq!(attempts.lock().unwrap()["user:admin"].count, 17);
        // Pressure on admin never delays a different account on another source.
        assert_eq!(
            admit_login(&attempts, address(100), "reader").unwrap(),
            Duration::ZERO
        );
    }

    #[test]
    fn source_limit_cannot_be_evaded_by_rotating_accounts_or_poison_other_accounts() {
        let attempts = Mutex::new(HashMap::new());
        for index in 0..MAX_ATTEMPTS {
            admit_login(&attempts, address(1), &format!("user{index}")).unwrap();
        }
        assert_eq!(
            admit_login(&attempts, address(1), "victim")
                .unwrap_err()
                .status,
            429
        );
        assert!(!attempts.lock().unwrap().contains_key("user:victim"));
        assert_eq!(
            admit_login(&attempts, address(2), "victim").unwrap(),
            Duration::ZERO
        );
    }

    fn proxy_config() -> ServerConfig {
        let mut config = ServerConfig::loopback(std::path::Path::new("/tmp/zxs-test"));
        config.external_origin = Some("https://storage.example.org".into());
        config.trusted_proxy_ips = vec!["127.0.0.1".parse().unwrap()];
        config
    }

    fn request_from(peer: &str, forwarded: Option<&str>) -> Request {
        let mut request = Request::new(axum::body::Body::empty());
        request.extensions_mut().insert(axum::extract::ConnectInfo(
            peer.parse::<SocketAddr>().unwrap(),
        ));
        if let Some(value) = forwarded {
            request
                .headers_mut()
                .insert("x-forwarded-for", value.parse().unwrap());
        }
        request
    }

    #[test]
    fn forwarding_is_ignored_by_default_and_from_untrusted_peers() {
        let default = ServerConfig::loopback(std::path::Path::new("/tmp/zxs-test"));
        assert_eq!(
            client_ip(
                &request_from("127.0.0.1:1234", Some("192.0.2.10")),
                &default
            )
            .unwrap(),
            "127.0.0.1".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            client_ip(
                &request_from("127.0.0.2:1234", Some("192.0.2.10")),
                &proxy_config()
            )
            .unwrap(),
            "127.0.0.2".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            client_ip(&request_from("127.0.0.1:1234", None), &default).unwrap(),
            "127.0.0.1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn trusted_proxy_requires_exact_single_ip_and_transport_metadata() {
        let config = proxy_config();
        for value in [
            None,
            Some(""),
            Some("garbage"),
            Some("192.0.2.1, 192.0.2.2"),
            Some("192.0.2.1:8080"),
            Some(" 192.0.2.1"),
        ] {
            assert!(client_ip(&request_from("127.0.0.1:1234", value), &config).is_err());
        }
        let mut duplicate = request_from("127.0.0.1:1234", Some("192.0.2.1"));
        duplicate
            .headers_mut()
            .append("x-forwarded-for", HeaderValue::from_static("192.0.2.2"));
        assert!(client_ip(&duplicate, &config).is_err());
        let missing_peer = Request::builder()
            .header("x-forwarded-for", "192.0.2.1")
            .body(axum::body::Body::empty())
            .unwrap();
        assert!(client_ip(&missing_peer, &config).is_err());
        assert_eq!(
            client_ip(
                &request_from("127.0.0.1:1234", Some("::ffff:192.0.2.1")),
                &config
            )
            .unwrap(),
            address(1)
        );
    }

    #[test]
    fn proxy_clients_have_separate_budgets_but_cannot_rotate_accounts_to_bypass_source_limit() {
        let attempts = Mutex::new(HashMap::new());
        let config = proxy_config();
        let first = client_ip(&request_from("127.0.0.1:1234", Some("192.0.2.1")), &config).unwrap();
        let second =
            client_ip(&request_from("127.0.0.1:1234", Some("192.0.2.2")), &config).unwrap();
        for _ in 0..MAX_ATTEMPTS {
            admit_login(&attempts, first, "admin").unwrap();
        }
        assert_eq!(
            admit_login(&attempts, first, "reader").unwrap_err().status,
            429
        );
        assert_eq!(
            admit_login(&attempts, second, "admin").unwrap(),
            Duration::from_millis(250)
        );
    }

    #[test]
    fn proxy_configuration_requires_loopback_peer_and_valid_https_origin() {
        let mut config = proxy_config();
        config.validate().unwrap();
        config.external_origin = None;
        assert!(config.validate().is_err());
        config.external_origin = Some("http://storage.example.org".into());
        assert!(config.validate().is_err());
        config.external_origin = Some("https://storage.example.org".into());
        config.trusted_proxy_ips = vec![address(1)];
        assert!(config.validate().is_err());
    }

    #[test]
    fn invalid_usernames_never_allocate_retained_pending_or_counter_keys() {
        let pending = Arc::new(Mutex::new(HashSet::new()));
        let attempts = Mutex::new(HashMap::new());
        for username in [
            "x".repeat(1_000_000),
            "管理员".repeat(100_000),
            "é".to_owned(),
            "a\nb".to_owned(),
            "".to_owned(),
            " admin ".repeat(100),
        ] {
            assert_eq!(login_username_key(&username).unwrap_err().status, 401);
            assert!(PendingLogin::acquire(&pending, &username).is_err());
            assert_eq!(
                admit_login(&attempts, address(1), &username)
                    .unwrap_err()
                    .status,
                401
            );
            assert!(pending.lock().unwrap().is_empty());
            assert!(attempts.lock().unwrap().is_empty());
        }
        assert_eq!(login_username_key(" Admin ").unwrap(), "user:admin");
        let longest = "x".repeat(zxs_auth::ADMINISTRATOR_USERNAME_MAX_BYTES);
        assert_eq!(
            login_username_key(&longest).unwrap().len(),
            5 + zxs_auth::ADMINISTRATOR_USERNAME_MAX_BYTES
        );
        assert!(login_username_key(&(longest + "x")).is_err());
    }

    #[test]
    fn pending_logins_are_per_canonical_account_bounded_and_release_on_drop() {
        let pending = Arc::new(Mutex::new(HashSet::new()));
        let first = PendingLogin::acquire(&pending, " Admin ").unwrap();
        assert!(PendingLogin::acquire(&pending, "ADMIN").is_err());
        let other = PendingLogin::acquire(&pending, "reader").unwrap();
        drop(first);
        let first = PendingLogin::acquire(&pending, "admin").unwrap();
        let mut guards = vec![first, other];
        for index in 2..MAX_PENDING_LOGINS {
            guards.push(PendingLogin::acquire(&pending, &format!("user{index}")).unwrap());
        }
        assert!(PendingLogin::acquire(&pending, "overflow").is_err());
        assert_eq!(pending.lock().unwrap().len(), MAX_PENDING_LOGINS);
        drop(guards);
        assert!(pending.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn cancelled_waiter_releases_account_guard_without_delaying_other_account() {
        let pending = Arc::new(Mutex::new(HashSet::new()));
        let waiting = pending.clone();
        let (ready, started) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _guard = PendingLogin::acquire(&waiting, "admin").unwrap();
            ready.send(()).unwrap();
            tokio::time::sleep(Duration::from_secs(6)).await;
        });
        started.await.unwrap();
        assert!(PendingLogin::acquire(&pending, "admin").is_err());
        let other = PendingLogin::acquire(&pending, "reader").unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        let admin = PendingLogin::acquire(&pending, "admin").unwrap();
        drop((admin, other));
        assert!(pending.lock().unwrap().is_empty());
    }

    #[test]
    fn account_backoff_survives_active_window_boundary_and_cools_after_idle_minute() {
        let attempts = Mutex::new(HashMap::from([(
            "user:admin".into(),
            Attempts {
                since: Instant::now() - Duration::from_secs(59),
                count: 20,
            },
        )]));
        assert_eq!(
            admit_login(&attempts, address(1), "admin").unwrap(),
            Duration::from_secs(6)
        );
        assert!(attempts.lock().unwrap()["user:admin"].since.elapsed() < Duration::from_secs(1));
        attempts
            .lock()
            .unwrap()
            .get_mut("user:admin")
            .unwrap()
            .since = Instant::now() - ATTEMPT_WINDOW;
        assert_eq!(
            admit_login(&attempts, address(2), "admin").unwrap(),
            Duration::ZERO
        );
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
        assert_eq!(attempts.lock().unwrap().len(), MAX_ATTEMPT_KEYS - 1);
        assert!(!attempts.lock().unwrap().contains_key("new:one"));
    }
}
