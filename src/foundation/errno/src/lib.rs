//! Cross-platform interface to the `errno` variable.
//!
//! # Examples
//! ```
//! use errno::{Errno, errno, set_errno};
//!
//! // Get the current value of errno
//! let e = errno();
//!
//! // Set the current value of errno
//! set_errno(e);
//!
//! // Extract the error code as an i32
//! let code = e.0;
//!
//! // Display a human-friendly error message
//! println!("Error {}: {}", code, e);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
#[cfg_attr(target_os = "wasi", path = "wasi.rs")]
#[cfg_attr(target_os = "hermit", path = "hermit.rs")]
mod sys;

use core::fmt;
#[cfg(feature = "std")]
use std::error::Error;
#[cfg(feature = "std")]
use std::io;

/// Wraps a platform-specific error code.
///
/// The `Display` instance maps the code to a human-readable string. It
/// calls [`strerror_r`][1] under POSIX, and [`FormatMessageW`][2] on
/// Windows.
///
/// [1]: http://pubs.opengroup.org/onlinepubs/009695399/functions/strerror.html
/// [2]: https://msdn.microsoft.com/en-us/library/windows/desktop/ms679351%28v=vs.85%29.aspx
#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Errno(pub i32);

impl fmt::Debug for Errno {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        sys::with_description(*self, |desc| {
            fmt.debug_struct("Errno")
                .field("code", &self.0)
                .field("description", &desc.ok())
                .finish()
        })
    }
}

impl fmt::Display for Errno {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        sys::with_description(*self, |desc| match desc {
            Ok(desc) => fmt.write_str(desc),
            Err(fm_err) => write!(
                fmt,
                "OS error {} ({} returned error {})",
                self.0,
                sys::STRERROR_NAME,
                fm_err.0
            ),
        })
    }
}

impl From<Errno> for i32 {
    fn from(e: Errno) -> Self {
        e.0
    }
}

#[cfg(feature = "std")]
impl Error for Errno {
    // TODO: Remove when MSRV >= 1.27
    #[allow(deprecated)]
    fn description(&self) -> &str {
        "system error"
    }
}

#[cfg(feature = "std")]
impl From<Errno> for io::Error {
    fn from(errno: Errno) -> Self {
        io::Error::from_raw_os_error(errno.0)
    }
}

/// Returns the platform-specific value of `errno`.
pub fn errno() -> Errno {
    sys::errno()
}

/// Sets the platform-specific value of `errno`.
pub fn set_errno(err: Errno) {
    sys::set_errno(err)
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/errno/functions/src/lib/it_works_103.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/errno/functions/src/lib/it_works_with_to_string_109.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/errno/functions/src/lib/check_description_116.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/errno/functions/src/lib/check_error_into_errno_152.rs"));
