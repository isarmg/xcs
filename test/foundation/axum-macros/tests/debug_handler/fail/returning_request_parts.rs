#[http_routing::debug_handler]
async fn handler() -> (
    http_routing::http::request::Parts, // this should be response parts, not request parts
    http_routing::http::StatusCode,
) {
    panic!()
}

fn main() {}
