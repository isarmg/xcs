#![cfg(test)]

use crate::ThreadPoolBuilder;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::{thread, time};

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/broadcast_global_9.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/spawn_broadcast_global_15.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/broadcast_pool_26.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/spawn_broadcast_pool_34.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/broadcast_self_46.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/spawn_broadcast_self_54.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/broadcast_mutual_66.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/spawn_broadcast_mutual_82.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/broadcast_mutual_sleepy_100.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/spawn_broadcast_mutual_sleepy_119.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/broadcast_panic_one_142.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/spawn_broadcast_panic_one_159.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/broadcast_panic_many_180.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/spawn_broadcast_panic_many_197.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/broadcast_sleep_race_218.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/broadcast_after_spawn_broadcast_233.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rayon-core/functions/src/broadcast/test/broadcast_after_spawn_249.rs"));
