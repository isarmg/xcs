use http_routing::extract::State;
use axum_macros::debug_handler;

#[debug_handler]
async fn handler(_: State<AppState>) {}

#[debug_handler]
async fn handler_2(_: http_routing::extract::State<AppState>) {}

#[debug_handler]
async fn handler_3(_: http_routing::extract::State<AppState>, _: http_routing::extract::State<AppState>) {}

#[debug_handler]
async fn handler_4(_: State<AppState>, _: State<AppState>) {}

#[debug_handler]
async fn handler_5(_: http_routing::extract::State<AppState>, _: State<AppState>) {}

#[derive(Clone)]
struct AppState;

fn main() {}
