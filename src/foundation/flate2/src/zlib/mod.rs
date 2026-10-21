pub mod bufread;
pub mod read;
pub mod write;

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/flate2/unit/src/zlib/mod/tests_5.rs"));
