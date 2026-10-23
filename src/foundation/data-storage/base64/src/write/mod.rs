//! Implementations of `io::Write` to transparently handle base64.
mod encoder;
mod encoder_string_writer;

pub use self::{
    encoder::EncoderWriter,
    encoder_string_writer::{EncoderStringWriter, StrConsumer},
};

#[cfg(test)]
#[path = "../../../../../../test/foundation/base64/modules/src/write/encoder_tests.rs"]
mod encoder_tests;
