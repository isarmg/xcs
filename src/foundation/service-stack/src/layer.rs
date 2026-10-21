//! A collection of [`Layer`] based tower services
//!
//! [`Layer`]: crate::Layer

pub use crate::{layer_fn, Layer, LayerFn};

/// Utilities for combining layers
///
/// [`Identity`]: crate::layer::util::Identity
/// [`Layer`]: crate::Layer
/// [`Stack`]: crate::layer::util::Stack
pub mod util {
    pub use crate::{Identity, Stack};
}
