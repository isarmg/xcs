use axum_macros::FromRequestParts;

#[derive(FromRequestParts)]
struct Extractor(http_routing::http::HeaderMap, http_routing::http::Method);

fn assert_from_request()
where
    Extractor: http_routing::extract::FromRequestParts<()>,
{
}

fn main() {}
