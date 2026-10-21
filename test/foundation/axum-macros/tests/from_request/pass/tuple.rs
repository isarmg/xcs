use axum_macros::FromRequest;

#[derive(FromRequest)]
struct Extractor(http_routing::http::HeaderMap, String);

fn assert_from_request()
where
    Extractor: http_routing::extract::FromRequest<()>,
{
}

fn main() {}
