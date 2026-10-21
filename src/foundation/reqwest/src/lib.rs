#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(test, deny(warnings))]

//! Native asynchronous HTTP transport for the storage service and operations client.
//!
//! The retained implementation uses Tokio, Rustls with WebPKI roots, JSON and
//! streaming bodies. Browser Web requests use the application's Fetch transport;
//! this internal component supports the native Linux targets only.
//!
//! Reuse [`Client`] instances to retain connection pooling. The original
//! upstream documentation and package identity are preserved in the central
//! `docs/upstream` and `compliance/provenance` archives.
//!
//! ```rust
//! # async fn example() -> Result<(), reqwest::Error> {
//! let client = reqwest::Client::new();
//! let response = client.get("https://example.com").send().await?;
//! let _body = response.bytes().await?;
//! # Ok(())
//! # }
//! ```

#[cfg(target_arch = "wasm32")]
compile_error!("the internal reqwest component is native-only; use the Web Fetch transport");

// Ignore `unused_crate_dependencies` warnings.
// Used in many features that they're not worth making it optional.
use futures as _;
use sync_wrapper as _;





pub use http::header;
pub use http::Method;
pub use http::{StatusCode, Version};
pub use url::Url;

// universal mods
#[macro_use]
mod error;

    mod config;

mod into_url;
mod response;

pub use self::error::{Error, Result};
pub use self::into_url::IntoUrl;
pub use self::response::ResponseBuilderExt;

/// Shortcut method to quickly make a `GET` request.
///
/// See also the methods on the [`reqwest::Response`](./struct.Response.html)
/// type.
///
/// **NOTE**: This function creates a new internal `Client` on each call,
/// and so should not be used if making many requests. Create a
/// [`Client`](./struct.Client.html) instead.
///
/// # Examples
///
/// ```rust
/// # async fn run() -> Result<(), reqwest::Error> {
/// let body = reqwest::get("https://www.rust-lang.org").await?
///     .text().await?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// This function fails if:
///
/// - native TLS backend cannot be initialized
/// - supplied `Url` cannot be parsed
/// - there was an error while sending request
/// - redirect limit was exhausted
pub async fn get<T: IntoUrl>(url: T) -> crate::Result<Response> {
    Client::builder().build()?.get(url).send().await
}

fn _assert_impls() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    fn assert_clone<T: Clone>() {}

    assert_send::<Client>();
    assert_sync::<Client>();
    assert_clone::<Client>();

    assert_send::<Request>();
    assert_send::<RequestBuilder>();


    {
        assert_send::<Response>();
    }

    assert_send::<Error>();
    assert_sync::<Error>();

    assert_send::<Body>();
    assert_sync::<Body>();
}



    pub use self::async_impl::{
        Body, Client, ClientBuilder, Request, RequestBuilder, Response, Upgraded,
    };
    pub use self::proxy::{Proxy,NoProxy};
    #[cfg(feature = "__tls")]
    // Re-exports, to be removed in a future release
    pub use tls::{Certificate, Identity};



    mod async_impl;

    mod connect;

    pub mod dns;
    mod proxy;
    pub mod redirect;
    pub mod retry;
    #[cfg(feature = "__tls")]
    pub mod tls;
    mod util;

    #[cfg(docsrs)]
    pub use connect::uds::UnixSocketProvider;
