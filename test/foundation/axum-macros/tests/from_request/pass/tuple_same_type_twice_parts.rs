use http_routing::extract::Query;
use axum_macros::FromRequestParts;
use data_serialization::Deserialize;

#[derive(FromRequestParts)]
struct Extractor(Query<Payload>, http_routing::extract::Path<Payload>);

#[derive(Deserialize)]
struct Payload {}

fn assert_from_request()
where
    Extractor: http_routing::extract::FromRequestParts<()>,
{
}

fn main() {}
