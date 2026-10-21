//! Parsers recognizing bytes streams

pub mod complete;
pub mod streaming;
#[cfg(test)]
#[path = "../../../../../test/foundation/nom/modules/src/bytes/tests.rs"]
mod tests;
