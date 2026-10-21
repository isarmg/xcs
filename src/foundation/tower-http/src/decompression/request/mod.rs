pub(super) mod future;
pub(super) mod layer;
pub(super) mod service;

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tower-http/unit/src/decompression/request/mod/tests_5.rs"));
