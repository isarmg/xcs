use http_routing::extract::State;
use axum_macros::FromRequestParts;

#[derive(FromRequestParts)]
struct Extractor {
    inner_state: State<AppState>,
}

#[derive(Clone)]
struct AppState {}

fn assert_from_request()
where
    Extractor: http_routing::extract::FromRequestParts<AppState, Rejection = http_routing::response::Response>,
{
}

fn main() {}
