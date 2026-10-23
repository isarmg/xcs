//! This module abstracts over `loom` and `std::sync` depending on whether we
//! are running tests or not.

#![allow(unused)]


mod std;

pub(crate) use self::std::*;
