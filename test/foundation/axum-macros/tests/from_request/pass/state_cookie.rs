use http_routing::extract::FromRef;
use axum_extra::extract::cookie::{Key, PrivateCookieJar};
use axum_macros::FromRequest;

#[derive(FromRequest)]
#[from_request(state(AppState))]
struct Extractor {
    cookies: PrivateCookieJar,
}

struct AppState {
    key: Key,
}

impl FromRef<AppState> for Key {
    fn from_ref(input: &AppState) -> Self {
        input.key.clone()
    }
}

fn assert_from_request()
where
    Extractor: http_routing::extract::FromRequest<AppState, Rejection = http_routing::response::Response>,
{
}

fn main() {}
