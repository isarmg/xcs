#![cfg(test)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

use crate::{join, Scope, ScopeFifo, ThreadPool, ThreadPoolBuilder};

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/panic_propagate_9.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/workers_stop_18.rs"));

fn join_a_lot(n: usize) {
    if n > 0 {
        join(|| join_a_lot(n - 1), || join_a_lot(n - 1));
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/sleeper_stop_46.rs"));

/// Creates a start/exit handler that increments an atomic counter.
fn count_handler() -> (Arc<AtomicUsize>, impl Fn(usize)) {
    let count = Arc::new(AtomicUsize::new(0));
    (Arc::clone(&count), move |_| {
        count.fetch_add(1, Ordering::SeqCst);
    })
}

/// Wait until a counter is no longer shared, then return its value.
fn wait_for_counter(mut counter: Arc<AtomicUsize>) -> usize {
    use std::{thread, time};

    for _ in 0..60 {
        counter = match Arc::try_unwrap(counter) {
            Ok(counter) => return counter.into_inner(),
            Err(counter) => {
                thread::sleep(time::Duration::from_secs(1));
                counter
            }
        };
    }

    // That's too long!
    panic!("Counter is still shared!");
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/failed_thread_stack_93.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/panic_thread_name_120.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/self_install_145.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/mutual_install_154.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/mutual_install_sleepy_174.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/check_thread_pool_new_202.rs"));

macro_rules! test_scope_order {
    ($scope:ident => $spawn:ident) => {{
        let builder = ThreadPoolBuilder::new().num_threads(1);
        let pool = builder.build().unwrap();
        pool.install(|| {
            let vec = Mutex::new(vec![]);
            pool.$scope(|scope| {
                let vec = &vec;
                for i in 0..10 {
                    scope.$spawn(move |_| {
                        vec.lock().unwrap().push(i);
                    });
                }
            });
            vec.into_inner().unwrap()
        })
    }};
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/scope_lifo_order_229.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/scope_fifo_order_237.rs"));

macro_rules! test_spawn_order {
    ($spawn:ident) => {{
        let builder = ThreadPoolBuilder::new().num_threads(1);
        let pool = &builder.build().unwrap();
        let (tx, rx) = channel();
        pool.install(move || {
            for i in 0..10 {
                let tx = tx.clone();
                pool.$spawn(move || {
                    tx.send(i).unwrap();
                });
            }
        });
        rx.iter().collect::<Vec<i32>>()
    }};
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/spawn_lifo_order_262.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/spawn_fifo_order_270.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/nested_scopes_278.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/nested_fifo_scopes_315.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/in_place_scope_no_deadlock_352.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/in_place_scope_fifo_no_deadlock_368.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/yield_now_to_spawn_384.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon-core/functions/src/thread_pool/test/yield_local_to_spawn_402.rs"));
