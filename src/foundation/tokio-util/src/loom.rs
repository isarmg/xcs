//! This module abstracts over `loom` and `std::sync` types depending on whether we
//! are running loom tests or not.

pub(crate) mod sync {


    pub(crate) use std::sync::{Arc, Mutex, MutexGuard};
}
