mod maybe_dangling;
#[cfg(feature = "adapters-codec")]
pub(crate) mod memchr;
#[cfg(any(feature = "adapters-io", feature = "adapters-codec"))]
mod poll_buf;

pub(crate) use maybe_dangling::MaybeDangling;
#[cfg(any(feature = "adapters-io", feature = "adapters-codec"))]
#[cfg_attr(not(feature = "adapters-io"), allow(unreachable_pub))]
pub use poll_buf::{poll_read_buf, poll_write_buf};

cfg_adapters_rt! {
    #[cfg_attr(not(feature = "adapters-io"), allow(unused))]
    pub(crate) use crate::task::coop::poll_proceed;
}

cfg_adapters_not_rt! {
    #[cfg_attr(not(feature = "adapters-io"), allow(unused))]
    use std::task::{Context, Poll};

    #[cfg_attr(not(feature = "adapters-io"), allow(unused))]
    pub(crate) struct RestoreOnPending;

    #[cfg_attr(not(feature = "adapters-io"), allow(unused))]
    impl RestoreOnPending {
        pub(crate) fn made_progress(&self) {}
    }

    #[cfg_attr(not(feature = "adapters-io"), allow(unused))]
    pub(crate) fn poll_proceed(_cx: &mut Context<'_>) -> Poll<RestoreOnPending> {
        Poll::Ready(RestoreOnPending)
    }
}
