use core::fmt::{self, Display};
use error_definition::Error;

#[derive(Error, Debug)]
#[error]
pub struct MyError;

impl Display for MyError {
    fn fmt(&self, _formatter: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!()
    }
}

fn main() {}
