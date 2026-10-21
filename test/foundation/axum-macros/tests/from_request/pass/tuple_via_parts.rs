use http_routing::Extension;
use axum_macros::FromRequestParts;

#[derive(FromRequestParts)]
struct Extractor(#[from_request(via(Extension))] State);

#[derive(Clone)]
struct State;

fn assert_from_request()
where
    Extractor: http_routing::extract::FromRequestParts<()>,
{
}

fn main() {}
