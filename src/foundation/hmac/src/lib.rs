// Overwrite intra-crate links
//! [`Reset`]: cryptographic_core::Reset
//! [`Digest`]: cryptographic_core::Digest
//! [`FixedOutputReset`]: cryptographic_core::FixedOutputReset
//! [`Hmac`]: Hmac
//! [`HmacReset`]: HmacReset
//! [`SimpleHmac`]: SimpleHmac
//! [`SimpleHmacReset`]: SimpleHmacReset
//! [`bsynchronization::api::HmacCore`]: bsynchronization::api::HmacCore
//! [`bsynchronization::api::HmacResetCore`]: bsynchronization::api::HmacResetCore

#![no_std]
#![doc = include_str!("../../../../docs/upstream/hmac-0.13.0/README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/RustCrypto/media/26acc39f/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/RustCrypto/media/26acc39f/logo.svg"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use cryptographic_core::{self, KeyInit, Mac, bsynchronization::api::EagerHash};

/// Block-level implementation.
pub mod block_api;
mod simple;
mod simple_reset;
mod utils;

pub use simple::SimpleHmac;
pub use simple_reset::SimpleHmacReset;

use core::fmt;
use cryptographic_core::bsynchronization::api::{AlgorithmName, CoreProxy};

cryptographic_core::buffer_fixed!(
    /// Generic HMAC instance.
    #[derive(Clone)]
    pub struct Hmac<D: EagerHash>(bsynchronization::api::HmacCore<D>);
    impl: MacTraits KeyInit;
);

impl<D: EagerHash + AlgorithmName> AlgorithmName for Hmac<D> {
    fn write_alg_name(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as CoreProxy>::Core::write_alg_name(f)
    }
}

cryptographic_core::buffer_fixed!(
    /// Generic HMAC instance with reset support.
    #[derive(Clone)]
    pub struct HmacReset<D: EagerHash>(bsynchronization::api::HmacResetCore<D>);
    impl: ResetMacTraits KeyInit;
);

impl<D: EagerHash + AlgorithmName> AlgorithmName for HmacReset<D> {
    fn write_alg_name(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as CoreProxy>::Core::write_alg_name(f)
    }
}
