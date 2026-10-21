#[cfg(all(test))]
mod test {
    use super::*;

    use crate::process::unix::orphan::test::MockQueue;
    use futures::future::FutureExt;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;
    use std::task::Context;
    use std::task::Poll;

    #[derive(Debug)]
    struct MockWait {
        total_kills: usize,
        total_waits: usize,
        num_wait_until_status: usize,
        status: ExitStatus,
    }

    impl MockWait {
        fn new(status: ExitStatus, num_wait_until_status: usize) -> Self {
            Self {
                total_kills: 0,
                total_waits: 0,
                num_wait_until_status,
                status,
            }
        }
    }

    impl Wait for MockWait {
        fn id(&self) -> u32 {
            0
        }

        fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            let ret = if self.num_wait_until_status == self.total_waits {
                Some(self.status)
            } else {
                None
            };

            self.total_waits += 1;
            Ok(ret)
        }
    }

    impl Kill for MockWait {
        fn kill(&mut self) -> io::Result<()> {
            self.total_kills += 1;
            Ok(())
        }
    }

    struct MockStream {
        total_polls: usize,
        values: Vec<Option<()>>,
    }

    impl MockStream {
        fn new(values: Vec<Option<()>>) -> Self {
            Self {
                total_polls: 0,
                values,
            }
        }
    }

    impl InternalStream for MockStream {
        fn poll_recv(&mut self, _cx: &mut Context<'_>) -> Poll<Option<()>> {
            self.total_polls += 1;
            match self.values.remove(0) {
                Some(()) => Poll::Ready(Some(())),
                None => Poll::Pending,
            }
        }
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/process/unix/reap/reaper_210.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/process/unix/reap/kill_249.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/process/unix/reap/drop_reaps_if_possible_263.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/process/unix/reap/drop_enqueues_orphan_if_wait_fails_282.rs"));
}
