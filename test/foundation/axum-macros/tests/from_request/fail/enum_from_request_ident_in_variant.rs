use axum_macros::FromRequest;

#[derive(FromRequest, Clone)]
#[from_request(via(http_routing::Extension))]
enum Extractor {
    Foo {
        #[from_request(via(http_routing::Extension))]
        foo: (),
    },
}

fn main() {}
