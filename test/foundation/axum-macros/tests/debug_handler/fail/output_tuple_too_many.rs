use http_routing::response::AppendHeaders;

#[http_routing::debug_handler]
async fn handler() -> (
    http_routing::http::StatusCode,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    AppendHeaders<[(http_routing::http::HeaderName, &'static str); 1]>,
    http_routing::http::StatusCode,
) {
    panic!()
}

fn main() {}
