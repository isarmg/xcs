//! `powerfmt` is a library that provides utilities for formatting values. Specifically, it makes it
//! significantly easier to support filling to a minimum width with alignment, avoid heap
//! allocation, and avoid repetitive calculations.

#![cfg_attr(docsrs, feature(doc_cfg, rustc_attrs))]
#![cfg_attr(docsrs, allow(internal_features))]


pub(crate) mod smart_display;
mod smart_display_impls;
