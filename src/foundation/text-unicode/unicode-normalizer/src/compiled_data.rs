// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

//! Data for the `unicode_normalizer` crate
//!
//! This data was generated with CLDR version 48.2.1, ICU version release-78.1rc, and
//! LSTM segmenter version v0.1.0.

// Generated tables retain their upstream bytes and macro exports.
// Some published table aliases are not used by this component.
#![allow(unused_imports)]

#[cfg(icu4x_custom_data)]
include!(concat!(core::env!("ICU4X_DATA_DIR"), "/mod.rs"));
#[cfg(not(icu4x_custom_data))]
include!("data/mod.rs");
