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
