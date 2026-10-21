use http_routing::extract::FromRef;

#[derive(Clone, FromRef)]
struct AppState<T> {
    foo: T,
}

fn main() {}
