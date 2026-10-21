#[cfg(test)]
mod test {
    use crate::{
        filter::LevelFilter,
        fmt::{
            format::{self, Format},
            time,
            writer::MakeWriter,
            Subscriber,
        },
    };
    use alloc::{borrow::ToOwned, string::String, vec::Vec};
    use std::{
        io,
        sync::{Arc, Mutex, MutexGuard, TryLockError},
    };
    use tracing_core::dispatcher::Dispatch;

    pub(crate) struct MockWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl MockWriter {
        pub(crate) fn new(buf: Arc<Mutex<Vec<u8>>>) -> Self {
            Self { buf }
        }

        pub(crate) fn map_error<Guard>(err: TryLockError<Guard>) -> io::Error {
            match err {
                TryLockError::WouldBlock => io::Error::from(io::ErrorKind::WouldBlock),
                TryLockError::Poisoned(_) => io::Error::from(io::ErrorKind::Other),
            }
        }

        pub(crate) fn buf(&self) -> io::Result<MutexGuard<'_, Vec<u8>>> {
            self.buf.try_lock().map_err(Self::map_error)
        }
    }

    impl io::Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buf()?.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.buf()?.flush()
        }
    }

    #[derive(Clone, Default)]
    pub(crate) struct MockMakeWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl MockMakeWriter {
        pub(crate) fn new(buf: Arc<Mutex<Vec<u8>>>) -> Self {
            Self { buf }
        }

        // this is currently only used by the JSON formatter tests. if we need
        // it elsewhere in the future, feel free to remove the `#[cfg]`
        // attribute!
        #[cfg(feature = "json")]
        pub(crate) fn buf(&self) -> MutexGuard<'_, Vec<u8>> {
            self.buf.lock().unwrap()
        }

        pub(crate) fn get_string(&self) -> String {
            let mut buf = self.buf.lock().expect("lock shouldn't be poisoned");
            let string = std::str::from_utf8(&buf[..])
                .expect("formatter should not have produced invalid utf-8")
                .to_owned();
            buf.clear();
            string
        }
    }

    impl<'a> MakeWriter<'a> for MockMakeWriter {
        type Writer = MockWriter;

        fn make_writer(&'a self) -> Self::Writer {
            MockWriter::new(self.buf.clone())
        }
    }

    #[test]
    fn impls() {
        let f = Format::default().with_timer(time::Uptime::default());
        let subscriber = Subscriber::builder().event_format(f).finish();
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default();
        let subscriber = Subscriber::builder().event_format(f).finish();
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default().compact();
        let subscriber = Subscriber::builder().event_format(f).finish();
        let _dispatch = Dispatch::new(subscriber);
    }

    #[test]
    fn subscriber_downcasts() {
        let subscriber = Subscriber::builder().finish();
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<Subscriber>().is_some());
    }

    #[test]
    fn subscriber_downcasts_to_parts() {
        let subscriber = Subscriber::new();
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<format::DefaultFields>().is_some());
        assert!(dispatch.downcast_ref::<LevelFilter>().is_some());
        assert!(dispatch.downcast_ref::<format::Format>().is_some())
    }

    #[test]
    fn is_lookup_span() {
        fn assert_lookup_span<T: for<'a> crate::registry::LookupSpan<'a>>(_: T) {}
        let subscriber = Subscriber::new();
        assert_lookup_span(subscriber)
    }
}
