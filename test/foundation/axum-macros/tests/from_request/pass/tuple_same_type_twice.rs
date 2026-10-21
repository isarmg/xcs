use http_routing::extract::Query;
use axum_macros::FromRequest;
use data_serialization::Deserialize;

#[derive(FromRequest)]
struct Extractor(Query<Payload>, http_routing::extract::Json<Payload>);

#[derive(Deserialize)]
struct Payload {}

fn assert_from_request()
where
    Extractor: http_routing::extract::FromRequest<()>,
{
}

fn main() {}
