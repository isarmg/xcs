#![allow(unused_parens)]

#[http_routing::debug_handler]
async fn named_type() -> (
    http_routing::http::StatusCode,
    http_routing::Json<&'static str>,
    http_routing::response::AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
) {
    panic!()
}

struct CustomIntoResponse {}
impl http_routing::response::IntoResponse for CustomIntoResponse {
    fn into_response(self) -> http_routing::response::Response {
        todo!()
    }
}
#[http_routing::debug_handler]
async fn custom_type() -> (
    http_routing::http::StatusCode,
    CustomIntoResponse,
    http_routing::response::AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
) {
    panic!()
}

fn main() {}
