use futures::Stream;
use std::{
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};
use tokio::{
    adapters::{
        io::ReaderStream,
        sync::{PollSemaphore, PollSender},
    },
    io::{AsyncRead, ReadBuf},
    sync::{mpsc, Semaphore},
};

struct Reader {
    remaining: usize,
    error: bool,
    calls: Arc<std::sync::atomic::AtomicUsize>,
}
impl AsyncRead for Reader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.remaining == 0 {
            return Poll::Ready(if self.error {
                Err(io::Error::other("end"))
            } else {
                Ok(())
            });
        }
        let n = self.remaining.min(buf.remaining()).min(31);
        assert!(n > 0);
        buf.put_slice(&[42; 31][..n]);
        self.remaining -= n;
        Poll::Ready(Ok(()))
    }
}

#[test]
fn reader_buffers_and_fuses_eof_and_error() {
    for error in [false, true] {
        for capacity in [0, 1, 17, 4096] {
            let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let reader = Reader {
                remaining: 8000,
                error,
                calls: calls.clone(),
            };
            let mut stream = ReaderStream::with_capacity(reader, capacity);
            let mut cx = Context::from_waker(Waker::noop());
            let mut count = 0;
            let mut errors = 0;
            loop {
                match Pin::new(&mut stream).poll_next(&mut cx) {
                    Poll::Ready(Some(Ok(bytes))) => {
                        assert!(bytes.iter().all(|b| *b == 42));
                        count += bytes.len();
                    }
                    Poll::Ready(Some(Err(_))) => errors += 1,
                    Poll::Ready(None) => break,
                    Poll::Pending => panic!("synchronous reader unexpectedly pending"),
                }
            }
            assert_eq!(count, 8000);
            assert_eq!(errors, usize::from(error));
            let terminal_calls = calls.load(std::sync::atomic::Ordering::SeqCst);
            for _ in 0..3 {
                assert!(matches!(
                    Pin::new(&mut stream).poll_next(&mut cx),
                    Poll::Ready(None)
                ));
            }
            assert_eq!(
                calls.load(std::sync::atomic::Ordering::SeqCst),
                terminal_calls
            );
        }
    }
}

#[test]
fn semaphore_fairness_cancellation_and_permit_release() {
    let semaphore = Arc::new(Semaphore::new(0));
    let mut first = PollSemaphore::new(semaphore.clone());
    let mut second = PollSemaphore::new(semaphore.clone());
    let mut cx = Context::from_waker(Waker::noop());
    assert!(first.poll_acquire_many(&mut cx, 2).is_pending());
    assert!(second.poll_acquire(&mut cx).is_pending());
    semaphore.add_permits(1);
    assert!(second.poll_acquire(&mut cx).is_pending());
    drop(first); // Cancelling the head returns its reserved permit to the next waiter.
    let Poll::Ready(Some(permit)) = second.poll_acquire(&mut cx) else {
        panic!("permit lost")
    };
    assert_eq!(semaphore.available_permits(), 0);
    drop(permit);
    assert_eq!(semaphore.available_permits(), 1);
    semaphore.close();
    for _ in 0..3 {
        assert!(matches!(second.poll_acquire(&mut cx), Poll::Ready(None)));
    }
}

#[test]
fn sender_reservation_cancellation_and_close() {
    let (tx, mut rx) = mpsc::channel(1);
    let mut first = PollSender::new(tx.clone());
    let mut second = PollSender::new(tx);
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(first.poll_reserve(&mut cx), Poll::Ready(Ok(()))));
    assert!(second.poll_reserve(&mut cx).is_pending());
    assert!(first.abort_send());
    assert!(matches!(second.poll_reserve(&mut cx), Poll::Ready(Ok(()))));
    second.send_item(7).unwrap();
    assert_eq!(rx.try_recv().unwrap(), 7);
    assert!(matches!(first.poll_reserve(&mut cx), Poll::Ready(Ok(()))));
    first.close();
    first.send_item(9).unwrap(); // Closing preserves an already acquired permit.
    assert_eq!(rx.try_recv().unwrap(), 9);
    assert!(matches!(first.poll_reserve(&mut cx), Poll::Ready(Err(_))));
    drop(rx);
    assert!(matches!(second.poll_reserve(&mut cx), Poll::Ready(Err(_))));
}

#[tokio::test]
async fn service_buffer_and_concurrency_use_runtime_permits() {
    use service_stack::{buffer::Buffer, limit::ConcurrencyLimit, service_fn, ServiceExt};
    let service = service_fn(|value: u32| async move { Ok::<_, std::io::Error>(value + 1) });
    let limited = ConcurrencyLimit::new(service, 1);
    let buffered = Buffer::new(limited, 2);
    let first = buffered.clone().oneshot(41);
    let second = buffered.oneshot(42);
    let (first, second) = tokio::join!(first, second);
    assert_eq!(first.unwrap(), 42);
    assert_eq!(second.unwrap(), 43);
}

#[tokio::test]
async fn cancellation_parent_child_and_drop_guard_wake_waiters() {
    use tokio::adapters::sync::CancellationToken;
    let parent = CancellationToken::new();
    let child = parent.child_token();
    let independent = parent.child_token();
    child.cancel();
    assert!(!parent.is_cancelled());
    assert!(!independent.is_cancelled());
    struct WakeCount(std::sync::atomic::AtomicUsize);
    impl std::task::Wake for WakeCount {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let wakes = Arc::new(WakeCount(std::sync::atomic::AtomicUsize::new(0)));
    let waker = Waker::from(wakes.clone());
    let mut waiter = Box::pin(independent.clone().cancelled_owned());
    assert!(
        std::future::Future::poll(waiter.as_mut(), &mut Context::from_waker(&waker)).is_pending()
    );
    let guard = parent.clone().drop_guard();
    drop(guard);
    assert!(wakes.0.load(std::sync::atomic::Ordering::SeqCst) > 0);
    waiter.await;
    assert!(parent.is_cancelled());
    assert!(independent.is_cancelled());
    parent.cancelled().await;
}

#[test]
fn reusable_future_replacement_releases_previous_state() {
    use std::{
        future::Future,
        sync::atomic::{AtomicUsize, Ordering},
    };
    use tokio::adapters::sync::ReusableBoxFuture;
    struct CountDrop(Arc<AtomicUsize>);
    impl Drop for CountDrop {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let drops = Arc::new(AtomicUsize::new(0));
    let old = CountDrop(drops.clone());
    let mut future = ReusableBoxFuture::new(async move {
        let _old = old;
        std::future::pending::<u8>().await
    });
    let mut cx = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    future.set(async { 42 });
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(Pin::new(&mut future).poll(&mut cx), Poll::Ready(42));
    async fn value(n: u8) -> u8 {
        n
    }
    future.set(value(7));
    assert_eq!(Pin::new(&mut future).poll(&mut cx), Poll::Ready(7));
    assert!(future.try_set(value(9)).is_ok()); // Same future layout must reuse its allocation.
    assert_eq!(Pin::new(&mut future).poll(&mut cx), Poll::Ready(9));
}

#[test]
fn codec_buffers_partial_frames_and_decodes_eof() {
    use tokio::adapters::{
        bytes::BytesMut,
        codec::{Decoder, Encoder, LinesCodec},
    };
    let mut codec = LinesCodec::new();
    let mut buffer = BytesMut::from(&b"first\npar"[..]);
    assert_eq!(codec.decode(&mut buffer).unwrap(), Some("first".to_owned()));
    assert_eq!(codec.decode(&mut buffer).unwrap(), None);
    buffer.extend_from_slice(b"tial");
    assert_eq!(
        codec.decode_eof(&mut buffer).unwrap(),
        Some("partial".to_owned())
    );
    assert_eq!(codec.decode_eof(&mut buffer).unwrap(), None);
    codec.encode("encoded", &mut buffer).unwrap();
    assert_eq!(&buffer[..], b"encoded\n");
}

#[tokio::test]
async fn tracker_close_wait_and_timer_expiration() {
    use tokio::adapters::{task::TaskTracker, time::DelayQueue};
    let tracker = TaskTracker::new();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tracker.spawn(async move {
        tx.send(42).unwrap();
    });
    tracker.close();
    tracker.wait().await;
    assert!(tracker.is_empty());
    assert_eq!(rx.await.unwrap(), 42);
    let mut queue = DelayQueue::new();
    let cancelled = queue.insert("cancelled", std::time::Duration::from_secs(60));
    queue.remove(&cancelled);
    queue.insert("ready", std::time::Duration::ZERO);
    let item = std::future::poll_fn(|cx| Pin::new(&mut queue).poll_next(cx))
        .await
        .unwrap();
    assert_eq!(item.into_inner(), "ready");
    assert!(queue.is_empty());
}
