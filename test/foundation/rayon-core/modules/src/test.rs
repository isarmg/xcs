#![cfg(test)]

use crate::{ThreadPoolBuildError, ThreadPoolBuilder};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/test/worker_thread_index_7.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/test/start_callback_called_17.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/test/exit_callback_called_44.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/test/handler_panics_handled_correctly_74.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/test/check_config_build_125.rs"));

/// Helper used by check_error_send_sync to ensure ThreadPoolBuildError is Send + Sync
fn _send_sync<T: Send + Sync>() {}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/test/check_error_send_sync_135.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/test/configuration_140.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/test/default_pool_162.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/test/cleared_current_thread_168.rs"));
