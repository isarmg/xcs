#![allow(unused_parens)]

struct NotIntoResponse;

#[http_routing::debug_handler]
async fn handler() -> (NotIntoResponse) {
    panic!()
}

fn main() {}
