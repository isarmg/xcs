//! HTTP client utilities

/// Legacy implementations of `connect` module and `Client`
#[cfg(feature = "client-legacy")]
pub mod legacy;

#[cfg(feature = "client-pool")]
pub mod pool;

#[cfg(feature = "client-proxy")]
pub mod proxy;

#[cfg(any(feature = "client-legacy", feature = "client-proxy"))]
fn strip_ipv6_brackets(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host)
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/http-transport/adapters/unit/src/client/mod/all_tests_20.rs"
));
