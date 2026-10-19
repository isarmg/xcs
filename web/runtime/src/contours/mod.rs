//! CSS-driven contour geometry, paint and browser lifecycle.
#[cfg(target_arch = "wasm32")]
mod controller;
pub mod geometry;
#[cfg(any(target_arch = "wasm32", test))]
mod paint;
mod profile;
#[cfg(target_arch = "wasm32")]
pub use controller::{ContourController, start};
