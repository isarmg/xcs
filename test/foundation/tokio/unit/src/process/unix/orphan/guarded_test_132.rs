#[cfg(all(test))]
pub(crate) mod test {
    use super::*;
    use crate::runtime::io::Driver as IoDriver;
    use crate::runtime::signal::{Driver as SignalDriver, Handle as SignalHandle};
    use crate::sync::watch;
    use std::cell::{Cell, RefCell};
    use std::io;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;
    use std::rc::Rc;

    pub(crate) struct MockQueue<W> {
        pub(crate) all_enqueued: RefCell<Vec<W>>,
    }

    impl<W> MockQueue<W> {
        pub(crate) fn new() -> Self {
            Self {
                all_enqueued: RefCell::new(Vec::new()),
            }
        }
    }

    impl<W> OrphanQueue<W> for MockQueue<W> {
        fn push_orphan(&self, orphan: W) {
            self.all_enqueued.borrow_mut().push(orphan);
        }
    }

    struct MockWait {
        total_waits: Rc<Cell<usize>>,
        num_wait_until_status: usize,
        return_err: bool,
    }

    impl MockWait {
        fn new(num_wait_until_status: usize) -> Self {
            Self {
                total_waits: Rc::new(Cell::new(0)),
                num_wait_until_status,
                return_err: false,
            }
        }

        fn with_err() -> Self {
            Self {
                total_waits: Rc::new(Cell::new(0)),
                num_wait_until_status: 0,
                return_err: true,
            }
        }
    }

    impl Wait for MockWait {
        fn id(&self) -> u32 {
            42
        }

        fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            let waits = self.total_waits.get();

            let ret = if self.num_wait_until_status == waits {
                if self.return_err {
                    Ok(Some(ExitStatus::from_raw(0)))
                } else {
                    Err(io::Error::new(io::ErrorKind::Other, "mock err"))
                }
            } else {
                Ok(None)
            };

            self.total_waits.set(waits + 1);
            ret
        }
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/functions/src/process/unix/orphan/drain_attempts_a_single_reap_of_all_queued_orphans_209.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/functions/src/process/unix/orphan/no_reap_if_no_signal_received_254.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/functions/src/process/unix/orphan/no_reap_if_signal_lock_held_278.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/functions/src/process/unix/orphan/does_not_register_signal_if_queue_empty_295.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/functions/src/process/unix/orphan/does_nothing_if_signal_could_not_be_registered_318.rs"));
}
