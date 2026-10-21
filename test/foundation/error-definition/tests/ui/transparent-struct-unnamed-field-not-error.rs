use error_definition::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct Error(String);

fn main() {}
