use axum_macros::FromRequestParts;

#[derive(FromRequestParts)]
struct Extractor();

fn assert_from_request()
where
    Extractor: http_routing::extract::FromRequestParts<(), Rejection = std::convert::Infallible>,
{
}

fn main() {}
