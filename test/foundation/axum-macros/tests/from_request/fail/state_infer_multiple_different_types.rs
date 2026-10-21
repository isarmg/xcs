use http_routing::extract::State;
use axum_macros::FromRequest;

#[derive(FromRequest)]
struct Extractor {
    inner_state: State<AppState>,
    other_state: State<OtherState>,
}

#[derive(Clone)]
struct AppState {}

#[derive(Clone)]
struct OtherState {}

fn assert_from_request()
where
    Extractor: http_routing::extract::FromRequest<AppState, Rejection = http_routing::response::Response>,
{
}

fn main() {}
