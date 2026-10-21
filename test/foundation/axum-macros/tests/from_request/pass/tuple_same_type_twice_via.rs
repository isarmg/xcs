use http_routing::extract::Query;
use http_routing::response::Response;
use axum_macros::FromRequest;
use data_serialization::Deserialize;

#[derive(FromRequest)]
struct Extractor(
    #[from_request(via(Query))] Payload,
    #[from_request(via(http_routing::extract::Json))] Payload,
);

#[derive(Deserialize)]
struct Payload {}

fn assert_from_request()
where
    Extractor: http_routing::extract::FromRequest<(), Rejection = Response>,
{
}

fn main() {}
