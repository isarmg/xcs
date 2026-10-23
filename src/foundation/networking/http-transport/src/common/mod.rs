#[cfg(all(any(feature = "client", feature = "server"), feature = "http1"))]
pub(crate) mod buf;
#[cfg(all(feature = "server", feature = "http1"))]
pub(crate) mod date;

#[cfg(any(
    all(feature = "client", feature = "http1"),
    all(feature = "server", feature = "http1")
))]
pub(crate) mod future;
pub(crate) mod io;
pub(crate) mod lock;
#[cfg(all(any(feature = "client", feature = "server"), feature = "http1"))]
pub(crate) mod task;
#[cfg(all(feature = "server", feature = "http1"))]
pub(crate) mod time;

// Private dispatch primitives retain their independent unsafe audit boundaries.
#[cfg(feature = "client")]
pub(crate) mod demand;
#[cfg(feature = "http1")]
pub(crate) mod task_waker;
#[cfg(feature = "client")]
pub(crate) mod try_lock;
