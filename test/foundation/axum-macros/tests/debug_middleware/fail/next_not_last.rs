use http_routing::{debug_middleware, extract::Request, middleware::Next, response::Response};

#[debug_middleware]
async fn my_middleware(next: Next, request: Request) -> Response {
    next.run(request).await
}

fn main() {}
