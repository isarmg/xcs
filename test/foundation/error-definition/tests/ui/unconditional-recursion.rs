use error_definition::Error;

#[derive(Error, Debug)]
#[error("{self}")]
pub struct Error;

fn main() {
    __FAIL__;
}
