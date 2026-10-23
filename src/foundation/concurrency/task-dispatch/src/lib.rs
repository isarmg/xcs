//! This crate makes it easier to write asynchronous code that is executor-agnostic, by providing a
//! utility that can be used to spawn tasks in a variety of executors.
//!
//! It only supports single executor per program, but that executor can be set at runtime, anywhere
//! in your crate (or an application that depends on it).
//!
//! Supported backends are Tokio, the browser event loop, and the futures executor used by tests.
//!
//! This is a least common denominator implementation in many ways. Limitations include:
//! - setting an executor is a one-time, global action
//! - no "join handle" or other result is returned from the spawn
//! - the `Future` must output `()`
//!
//! ```no_run
//! use task_dispatch::Executor;
//!
//! // make sure an Executor has been initialized with one of the init_ functions
//!
//! // spawn a thread-safe Future
//! Executor::spawn(async { /* ... */ });
//!
//! // spawn a Future that is !Send
//! Executor::spawn_local(async { /* ... */ });
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

use error_definition::Error;
use std::{future::Future, pin::Pin, sync::OnceLock};

/// A future that has been pinned.
pub type PinnedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
/// A future that has been pinned.
pub type PinnedLocalFuture<T> = Pin<Box<dyn Future<Output = T>>>;

// Type alias for the spawn function pointer.
type SpawnFn = fn(PinnedFuture<()>);
// Type alias for the spawn_local function pointer.
type SpawnLocalFn = fn(PinnedLocalFuture<()>);
// Type alias for the poll_local function pointer.
type PollLocalFn = fn();

/// Holds the function pointers for the current global executor.
#[derive(Clone, Copy)]
struct ExecutorFns {
    spawn: SpawnFn,
    spawn_local: SpawnLocalFn,
    poll_local: PollLocalFn,
}

// Use a single OnceLock to ensure atomic initialization of all functions.
static EXECUTOR_FNS: OnceLock<ExecutorFns> = OnceLock::new();

// Backends driven by their own event loop do not need explicit polling.
#[cfg(any(feature = "tokio", feature = "wasm-bindgen"))]
fn no_op_poll() {}

/// Errors that can occur when using the executor.
#[derive(Error, Debug)]
pub enum ExecutorError {
    /// The executor has already been set.
    #[error("Global executor has already been set.")]
    AlreadySet,
}

/// A global async executor that can spawn tasks.
pub struct Executor;

impl Executor {
    /// Spawns a thread-safe [`Future`].
    ///
    /// Uses the globally configured executor.
    /// Panics if no global executor has been initialized.
    #[inline(always)]
    #[track_caller]
    pub fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
        let fns = EXECUTOR_FNS
            .get()
            .expect("Executor::spawn called before initialization");
        (fns.spawn)(Box::pin(fut));
    }

    /// Spawns a [`Future`] that cannot be sent across threads.
    ///
    /// Uses the globally configured executor. The backend's local execution context
    /// must be active on the calling thread (for example, a Tokio `LocalSet`).
    /// Panics if no global executor has been initialized.
    #[inline(always)]
    #[track_caller]
    pub fn spawn_local(fut: impl Future<Output = ()> + 'static) {
        let fns = EXECUTOR_FNS
            .get()
            .expect("Executor::spawn_local called before initialization");
        (fns.spawn_local)(Box::pin(fut));
    }

    /// Waits until the next "tick" of the current async executor.
    /// Respects the global executor.
    #[inline(always)]
    pub async fn tick() {
        let (tx, rx) = futures::channel::oneshot::channel();
        #[cfg(not(all(feature = "wasm-bindgen", target_family = "wasm")))]
        Executor::spawn(async move {
            _ = tx.send(());
        });
        #[cfg(all(feature = "wasm-bindgen", target_family = "wasm"))]
        Executor::spawn_local(async move {
            _ = tx.send(());
        });

        _ = rx.await;
    }

    /// Polls the global async executor.
    ///
    /// Uses the globally configured executor.
    /// Does nothing if the global executor does not support polling.
    #[inline(always)]
    pub fn poll_local() {
        if let Some(fns) = EXECUTOR_FNS.get() {
            (fns.poll_local)()
        }
        // If not initialized or doesn't support polling, do nothing gracefully.
    }
}

impl Executor {
    /// Globally sets the [`tokio`] runtime as the executor used to spawn tasks.
    ///
    /// Returns `Err(_)` if a global executor has already been set.
    ///
    /// Requires the `tokio` feature to be activated on this crate.
    #[cfg(feature = "tokio")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
    pub fn init_tokio() -> Result<(), ExecutorError> {
        let executor_impl = ExecutorFns {
            spawn: |fut| {
                tokio::spawn(fut);
            },
            spawn_local: |fut| {
                tokio::task::spawn_local(fut);
            },
            // Tokio doesn't have an explicit global poll function like LocalPool::run_until_stalled
            poll_local: no_op_poll,
        };
        EXECUTOR_FNS
            .set(executor_impl)
            .map_err(|_| ExecutorError::AlreadySet)
    }

    /// Globally sets the [`js_sys::futures`] runtime as the executor used to spawn tasks.
    ///
    /// Returns `Err(_)` if a global executor has already been set.
    ///
    /// Requires the `wasm-bindgen` feature to be activated on this crate.
    #[cfg(feature = "wasm-bindgen")]
    #[cfg_attr(docsrs, doc(cfg(feature = "wasm-bindgen")))]
    pub fn init_wasm_bindgen() -> Result<(), ExecutorError> {
        let executor_impl = ExecutorFns {
            // Send futures can also run on the single-threaded browser event loop.
            spawn: |fut| js_sys::futures::spawn_local(fut),
            spawn_local: |fut| {
                js_sys::futures::spawn_local(fut);
            },
            poll_local: no_op_poll,
        };
        EXECUTOR_FNS
            .set(executor_impl)
            .map_err(|_| ExecutorError::AlreadySet)
    }

    /// Globally sets the [`futures`] executor as the executor used to spawn tasks,
    /// lazily creating a thread pool to spawn tasks into.
    ///
    /// Returns `Err(_)` if a global executor has already been set.
    ///
    /// Requires the `futures-executor` feature to be activated on this crate.
    #[cfg(feature = "futures-executor")]
    #[cfg_attr(docsrs, doc(cfg(feature = "futures-executor")))]
    pub fn init_futures_executor() -> Result<(), ExecutorError> {
        use futures::{
            executor::{LocalPool, LocalSpawner, ThreadPool},
            task::{LocalSpawnExt, SpawnExt},
        };
        use std::cell::RefCell;

        // Keep the lazy-init ThreadPool and thread-local LocalPool for spawn_local impl
        static THREAD_POOL: OnceLock<ThreadPool> = OnceLock::new();
        thread_local! {
            static LOCAL_POOL: RefCell<LocalPool> = RefCell::new(LocalPool::new());
            // SPAWNER is derived from LOCAL_POOL, keep it for efficiency inside the closure
            static SPAWNER: LocalSpawner = LOCAL_POOL.with(|pool| pool.borrow().spawner());
        }

        fn get_thread_pool() -> &'static ThreadPool {
            THREAD_POOL.get_or_init(|| {
                ThreadPool::new().expect("could not create futures executor ThreadPool")
            })
        }

        let executor_impl = ExecutorFns {
            spawn: |fut| {
                get_thread_pool()
                    .spawn(fut)
                    .expect("failed to spawn future on ThreadPool");
            },
            spawn_local: |fut| {
                // Use the thread_local SPAWNER derived from LOCAL_POOL
                SPAWNER.with(|spawner| {
                    spawner
                        .spawn_local(fut)
                        .expect("failed to spawn local future");
                });
            },
            poll_local: || {
                // Use the thread_local LOCAL_POOL
                LOCAL_POOL.with(|pool| {
                    // Use try_borrow_mut to prevent panic during re-entrant calls
                    if let Ok(mut pool) = pool.try_borrow_mut() {
                        pool.run_until_stalled();
                    }
                    // If already borrowed, we're likely in a nested poll, so do nothing.
                });
            },
        };

        EXECUTOR_FNS
            .set(executor_impl)
            .map_err(|_| ExecutorError::AlreadySet)
    }
}
