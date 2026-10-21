use http_routing::extract::Query;
use http_routing::response::Response;
use axum_macros::FromRequestParts;
use data_serialization::Deserialize;

#[derive(FromRequestParts)]
struct Extractor(
    #[from_request(via(Query))] Payload,
    #[from_request(via(http_routing::extract::Path))] Payload,
);

#[derive(Deserialize)]
struct Payload {}

fn assert_from_request()
where
    Extractor: http_routing::extract::FromRequestParts<(), Rejection = Response>,
{
}

fn main() {}
