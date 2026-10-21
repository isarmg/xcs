#![cfg(feature = "std")]

use tracing_mock::*;
use tracing::collector::prelude::*;

#[test]
fn init_ext_works() {
    let (subscriber, finished) = subscriber::mock()
        .event(
            expect::event()
                .at_level(tracing::Level::INFO)
                .with_target("init_works"),
        )
        .only()
        .run_with_handle();

    let _guard = subscriber.set_default();
    tracing::info!(target: "init_works", "it worked!");
    finished.assert_finished();
}

#[test]
#[cfg(feature = "fmt")]
fn builders_are_init_ext() {
    tracing::collector::fmt().set_default();
    let _ = tracing::collector::fmt()
        .with_target(false)
        .compact()
        .try_init();
}

#[test]
#[cfg(all(feature = "fmt", feature = "env-filter"))]
fn layered_is_init_ext() {
    tracing::collector::registry()
        .with(tracing::collector::fmt::layer())
        .with(tracing::collector::EnvFilter::new("foo=info"))
        .set_default();
}
