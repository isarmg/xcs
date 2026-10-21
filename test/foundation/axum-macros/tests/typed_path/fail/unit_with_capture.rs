use axum_macros::TypedPath;
use data_serialization::Deserialize;

#[derive(TypedPath, Deserialize)]
#[typed_path("/users/{id}")]
struct MyPath;

fn main() {}
