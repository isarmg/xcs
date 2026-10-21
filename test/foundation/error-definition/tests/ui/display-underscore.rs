use error_definition::Error;

#[derive(Error, Debug)]
#[error("{_}")]
pub struct Error;

fn main() {}
