#![cfg(all(feature = "registry", feature = "fmt"))]
use tracing::collector::{filter::LevelFilter, prelude::*};

#[test]
fn registry_sets_max_level_hint() {
    tracing::collector::registry()
        .with(tracing::collector::fmt::layer())
        .with(LevelFilter::DEBUG)
        .init();
    assert_eq!(LevelFilter::current(), LevelFilter::DEBUG);
}
