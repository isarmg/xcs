use axum_extra::routing::{RouterExt, TypedPath};
use data_serialization::Deserialize;

#[derive(TypedPath, Deserialize)]
#[typed_path("/{*rest}")]
struct MyPath {
    rest: String,
}

fn main() {
    _ = http_routing::Router::<()>::new().typed_get(|_: MyPath| async {});
}
