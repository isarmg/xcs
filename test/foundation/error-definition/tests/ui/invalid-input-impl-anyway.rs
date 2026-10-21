use error_definition::Error;

#[derive(Error, Debug)]
#[error]
pub struct MyError;

fn main() {
    // No error on the following line. ErrorDefinition emits an Error impl despite the
    // bad attribute.
    _ = &MyError as &dyn std::error::Error;
}
