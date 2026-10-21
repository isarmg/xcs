#[cfg(all(test))]
mod test {
    use super::kill::Kill;
    use super::ChildDropGuard;

    use futures::future::FutureExt;
    use std::future::Future;
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    struct Mock {
        num_kills: usize,
        num_polls: usize,
        poll_result: Poll<Result<(), ()>>,
    }

    impl Mock {
        fn new() -> Self {
            Self::with_result(Poll::Pending)
        }

        fn with_result(result: Poll<Result<(), ()>>) -> Self {
            Self {
                num_kills: 0,
                num_polls: 0,
                poll_result: result,
            }
        }
    }

    impl Kill for Mock {
        fn kill(&mut self) -> io::Result<()> {
            self.num_kills += 1;
            Ok(())
        }
    }

    impl Future for Mock {
        type Output = Result<(), ()>;

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            let inner = Pin::get_mut(self);
            inner.num_polls += 1;
            inner.poll_result
        }
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/process/mod/kills_on_drop_if_specified_1764.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/process/mod/no_kill_on_drop_by_default_1780.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/process/mod/no_kill_if_already_killed_1796.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/process/mod/no_kill_if_reaped_1813.rs"));
}
