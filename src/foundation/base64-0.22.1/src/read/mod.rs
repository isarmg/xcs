//! Implementations of `io::Read` to transparently decode base64.
mod decoder;
pub use self::decoder::DecoderReader;

#[cfg(test)]
#[path = "../../../../../test/foundation/base64-0.22.1/modules/src/read/decoder_tests.rs"]
mod decoder_tests;
