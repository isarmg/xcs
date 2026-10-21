use axum_macros::FromRequest;

#[derive(FromRequest)]
#[from_request(via(http_routing::Extension))]
struct Extractor(#[from_request(via(http_routing::Extension))] State);

#[derive(Clone)]
struct State;

fn main() {}
