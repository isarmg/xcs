//! Native ANSI output adapters. Enable the `output` feature explicitly.

pub mod adapter;
pub mod stream;

mod auto;
mod buffer;
mod fmt;
mod strip;

pub use auto::AutoStream;
pub use strip::StripStream;

#[allow(deprecated)]
pub use buffer::Buffer;

/// An adaptive wrapper around the global standard output stream of the current process
pub type Stdout = AutoStream<std::io::Stdout>;
/// An adaptive wrapper around the global standard error stream of the current process
pub type Stderr = AutoStream<std::io::Stderr>;

/// Create an ANSI escape code compatible stdout
///
/// **Note:** Call [`AutoStream::lock`] in loops to avoid the performance hit of acquiring/releasing
/// from the implicit locking in each [`std::io::Write`] call
#[cfg(feature = "output")]
pub fn stdout() -> Stdout {
    let stdout = std::io::stdout();
    AutoStream::auto(stdout)
}

/// Create an ANSI escape code compatible stderr
///
/// **Note:** Call [`AutoStream::lock`] in loops to avoid the performance hit of acquiring/releasing
/// from the implicit locking in each [`std::io::Write`] call
#[cfg(feature = "output")]
pub fn stderr() -> Stderr {
    let stderr = std::io::stderr();
    AutoStream::auto(stderr)
}

/// Selection for overriding color output
use crate::ColorChoice;
