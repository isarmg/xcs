use axum::extract::Query;
use axum::response::Response;
use axum_macros::FromRequest;
use data_serialization::Deserialize;

#[derive(FromRequest)]
struct Extractor(
    #[from_request(via(Query))] Payload,
    #[from_request(via(axum::extract::Json))] Payload,
);

#[derive(Deserialize)]
struct Payload {}

fn assert_from_request()
where
    Extractor: axum::extract::FromRequest<(), Rejection = Response>,
{
}

fn main() {}
