use axum_macros::FromRequest;

#[derive(FromRequest, Clone)]
#[from_request(via(http_routing::Extension))]
enum Extractor {
    #[from_request(via(http_routing::Extension))]
    Foo,
}

fn main() {}
