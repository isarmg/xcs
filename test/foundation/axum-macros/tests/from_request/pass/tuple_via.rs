use http_routing::Extension;
use axum_macros::FromRequest;

#[derive(FromRequest)]
struct Extractor(#[from_request(via(Extension))] State);

#[derive(Clone)]
struct State;

fn assert_from_request()
where
    Extractor: http_routing::extract::FromRequest<()>,
{
}

fn main() {}
