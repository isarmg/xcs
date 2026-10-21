pub(crate) mod request;
pub(crate) mod request_parts;

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/axum-core/unit/src/ext_traits/mod/tests_4.rs"));
