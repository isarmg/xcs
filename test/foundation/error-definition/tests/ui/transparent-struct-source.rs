use error_definition::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct Error(#[source] anyhow::Error);

fn main() {}
