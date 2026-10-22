#![allow(clippy::needless_doctest_main)]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub
)]

//! Utilities for working with Tokio.
//!
//! Runtime adapters share the runtime package and its synchronization primitives.
//! Optional adapter capabilities use `adapters-*` features, independently of core features.

#[macro_use]
mod cfg;

mod loom;

cfg_adapters_codec! {
    #[macro_use]
    mod tracing;

    pub mod codec;
}

cfg_adapters_net! {
    #[cfg(not(target_arch = "wasm32"))]
    pub mod udp;
    pub mod net;
}

cfg_adapters_compat! {
    pub mod compat;
}

cfg_adapters_io! {
    pub mod io;
}

cfg_adapters_rt! {
    pub mod context;
}

#[cfg(feature = "adapters-rt")]
pub mod task;

cfg_adapters_time! {
    pub mod time;
}

pub mod sync;

pub mod either;

#[cfg(any(feature = "adapters-io", feature = "adapters-codec"))]
pub use bytes;

mod util;

pub mod future;
