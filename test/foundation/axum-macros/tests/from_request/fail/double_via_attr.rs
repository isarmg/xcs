use axum_macros::FromRequest;

#[derive(FromRequest)]
struct Extractor(#[from_request(via(http_routing::Extension), via(http_routing::Extension))] State);

#[derive(Clone)]
struct State;

fn main() {}
