#[derive(Debug, error_definition::Error)]
pub enum Error {
    #[error("TryFrom failed: {0}")]
    TryFrom(String),
}
