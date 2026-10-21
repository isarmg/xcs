//! # hyper-rustls
//!
//! A pure-Rust HTTPS connector for [hyper](https://hyper.rs), based on
//! [Rustls](https://github.com/rustls/rustls).
//!
//! ## Example client
//!
//! ```no_run
//! # #[cfg(all(feature = "rustls-native-certs", feature = "http1"))]
//! # fn main() {
//! use http::StatusCode;
//! use http_body::Empty;
//! use http_transport::body::Bytes;
//! use http_transport::adapters::client::legacy::Client;
//! use http_transport::adapters::rt::TokioExecutor;
//!
//! let mut rt = tokio::runtime::Runtime::new().unwrap();
//! let url = ("https://hyper.rs").parse().unwrap();
//! let https = hyper_rustls::HttpsConnectorBuilder::new()
//!     .with_native_roots()
//!     .expect("no native root CA certificates found")
//!     .https_only()
//!     .enable_http1()
//!     .build();
//!
//! let client: Client<_, Empty<Bytes>> = Client::builder(TokioExecutor::new()).build(https);
//!
//! let res = rt.block_on(client.get(url)).unwrap();
//! assert_eq!(res.status(), StatusCode::OK);
//! # }
//! # #[cfg(not(all(feature = "rustls-native-certs", feature = "http1")))]
//! # fn main() {}
//! ```

#![warn(missing_docs)]
#![cfg_attr(hyper_rustls_docsrs, feature(doc_cfg))]

mod config;
mod connector;
mod stream;

#[cfg(feature = "logging")]
mod log {


}

#[cfg(not(feature = "logging"))]
mod log {




}

pub use crate::config::ConfigBuilderExt;
pub use crate::connector::builder::ConnectorBuilder as HttpsConnectorBuilder;
pub use crate::connector::{
    DefaultServerNameResolver, FixedServerNameResolver, HttpsConnector, ResolveServerName,
};
pub use crate::stream::MaybeHttpsStream;

/// The various states of the [`HttpsConnectorBuilder`]
pub mod builderstates {

    pub use crate::connector::builder::{
        WantsProtocols1, WantsProtocols2, WantsSchemes, WantsTlsConfig,
    };
}
