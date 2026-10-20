//! HTTP-only admission and wall-clock body lifetime bounds.
use crate::{ApiError, lock};
use futures_util::Stream;
use std::{
    collections::BTreeMap,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    time::Duration,
};
use tokio::time::Instant;

#[derive(Clone, Debug)]
pub struct TransferLimits {
    /// Independent upload and read pools, each with this total capacity.
    pub slots_total: usize,
    pub slots_per_account: usize,
    pub idle_timeout: Duration,
    pub total_timeout: Duration,
}
impl Default for TransferLimits {
    fn default() -> Self {
        Self {
            slots_total: 8,
            slots_per_account: 2,
            idle_timeout: Duration::from_secs(60),
            total_timeout: Duration::from_secs(3600),
        }
    }
}
impl TransferLimits {
    pub(crate) fn validate(&self) -> Result<(), ApiError> {
        if self.slots_per_account == 0
            || self.slots_per_account >= self.slots_total
            || self.idle_timeout.is_zero()
            || self.total_timeout < self.idle_timeout
            || self.total_timeout > Duration::from_secs(7 * 24 * 3600)
        {
            return Err(ApiError::invalid(
                "HTTP transfers require 0 < account slots < total slots and 0 < idle <= total timeout <= 7 days",
            ));
        }
        Ok(())
    }
}
#[derive(Default)]
struct Counts {
    total: usize,
    accounts: BTreeMap<String, usize>,
}
pub(crate) struct Admission {
    counts: Mutex<Counts>,
    total: usize,
    per_account: usize,
}
pub(crate) struct Permit {
    admission: Arc<Admission>,
    principal: String,
}
impl Admission {
    pub(crate) fn new(limits: &TransferLimits) -> Arc<Self> {
        Arc::new(Self {
            counts: Mutex::new(Counts::default()),
            total: limits.slots_total,
            per_account: limits.slots_per_account,
        })
    }
    pub(crate) fn acquire(self: &Arc<Self>, principal: &str) -> Result<Permit, ApiError> {
        let mut counts = lock(&self.counts)?;
        if counts.total >= self.total
            || counts.accounts.get(principal).copied().unwrap_or(0) >= self.per_account
        {
            return Err(ApiError::limit(
                "HTTP transfer account or global capacity reached",
            ));
        }
        counts.total += 1;
        *counts.accounts.entry(principal.into()).or_default() += 1;
        Ok(Permit {
            admission: self.clone(),
            principal: principal.into(),
        })
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        if let Ok(mut counts) = self.admission.counts.lock() {
            counts.total -= 1;
            if let Some(count) = counts.accounts.get_mut(&self.principal) {
                *count -= 1;
                if *count == 0 {
                    counts.accounts.remove(&self.principal);
                }
            }
        }
    }
}
pub(crate) fn timeout_error() -> ApiError {
    ApiError::new(
        408,
        "TRANSFER_TIMEOUT",
        "HTTP transfer deadline exceeded; durable work data is retained",
    )
}

struct StreamState<S> {
    stream: Option<Pin<Box<S>>>,
    expired: bool,
    waker: Option<Waker>,
}
/// The timer drops an idle body even when the HTTP consumer stops polling it.
/// Blocking jobs retain their own lease/permit until they actually finish.
pub(crate) struct BoundedStream<S> {
    state: Arc<Mutex<StreamState<S>>>,
    progress: tokio::sync::watch::Sender<Instant>,
    timer: tokio::task::JoinHandle<()>,
    idle: Duration,
    deadline: Instant,
}
impl<S> BoundedStream<S>
where
    S: Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send + 'static,
{
    pub(crate) fn new(stream: S, limits: &TransferLimits, deadline: Instant) -> Self {
        let state = Arc::new(Mutex::new(StreamState {
            stream: Some(Box::pin(stream)),
            expired: false,
            waker: None,
        }));
        let (progress, mut updates) =
            tokio::sync::watch::channel(Instant::now() + limits.idle_timeout);
        let shared = state.clone();
        let timer = tokio::spawn(async move {
            loop {
                let next = (*updates.borrow_and_update()).min(deadline);
                tokio::select! {
                    biased;
                    _ = tokio::time::sleep_until(next) => {
                        let mut state = shared.lock().unwrap_or_else(|e| e.into_inner());
                        // A chunk may have reset idle while this timer became ready.
                        if Instant::now() < (*updates.borrow()).min(deadline) { continue; }
                        if state.stream.is_none() { return; }
                        state.expired = true;
                        state.stream = None;
                        if let Some(waker) = state.waker.take() { waker.wake(); }
                        return;
                    }
                    result = updates.changed() => { if result.is_err() { return; } }
                }
            }
        });
        Self {
            state,
            progress,
            timer,
            idle: limits.idle_timeout,
            deadline,
        }
    }
}
impl<S: Stream<Item = Result<bytes::Bytes, std::io::Error>>> Stream for BoundedStream<S> {
    type Item = Result<bytes::Bytes, std::io::Error>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.stream.is_some() && Instant::now() >= (*self.progress.borrow()).min(self.deadline)
        {
            state.stream = None;
            state.expired = true;
            self.timer.abort();
        }
        if state.expired {
            state.expired = false;
            return Poll::Ready(Some(Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "HTTP transfer deadline exceeded",
            ))));
        }
        let Some(stream) = state.stream.as_mut() else {
            return Poll::Ready(None);
        };
        let result = stream.as_mut().poll_next(cx);
        match &result {
            Poll::Ready(Some(Ok(bytes))) if !bytes.is_empty() => {
                self.progress.send_replace(Instant::now() + self.idle);
            }
            Poll::Ready(None) | Poll::Ready(Some(Err(_))) => {
                state.stream = None;
                self.timer.abort();
            }
            _ => {}
        }
        state.waker = Some(cx.waker().clone());
        result
    }
}
impl<S> Drop for BoundedStream<S> {
    fn drop(&mut self) {
        self.timer.abort();
        self.state.lock().unwrap_or_else(|e| e.into_inner()).stream = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{StreamExt, stream};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn admission_is_atomic_account_scoped_and_reclaims_keys() {
        let limits = TransferLimits {
            slots_total: 3,
            slots_per_account: 2,
            ..TransferLimits::default()
        };
        let admission = Admission::new(&limits);
        let a = admission.acquire("alice").unwrap();
        let b = admission.acquire("alice").unwrap();
        assert_eq!(admission.acquire("alice").err().unwrap().status, 429);
        let c = admission.acquire("bob").unwrap();
        assert_eq!(admission.acquire("carol").err().unwrap().status, 429);
        drop(a);
        let d = admission.acquire("carol").unwrap();
        drop((b, c, d));
        let counts = admission.counts.lock().unwrap();
        assert_eq!(counts.total, 0);
        assert!(counts.accounts.is_empty());
    }

    #[test]
    fn invalid_limits_cannot_disable_fairness_or_deadlines() {
        for limits in [
            TransferLimits {
                slots_per_account: 0,
                ..TransferLimits::default()
            },
            TransferLimits {
                slots_per_account: 8,
                ..TransferLimits::default()
            },
            TransferLimits {
                idle_timeout: Duration::ZERO,
                ..TransferLimits::default()
            },
            TransferLimits {
                total_timeout: Duration::from_secs(30),
                ..TransferLimits::default()
            },
            TransferLimits {
                total_timeout: Duration::MAX,
                ..TransferLimits::default()
            },
        ] {
            assert!(limits.validate().is_err());
        }
        assert!(TransferLimits::default().validate().is_ok());
    }

    struct Lease(Arc<AtomicUsize>);
    impl Drop for Lease {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn short_limits() -> TransferLimits {
        TransferLimits {
            idle_timeout: Duration::from_millis(30),
            total_timeout: Duration::from_secs(2),
            ..TransferLimits::default()
        }
    }
    async fn wait_for_drop(dropped: &AtomicUsize) {
        tokio::time::timeout(Duration::from_secs(3), async {
            while dropped.load(Ordering::SeqCst) == 0 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn unpolled_body_expires_and_cannot_resume_old_stream() {
        let dropped = Arc::new(AtomicUsize::new(0));
        let lease = Lease(dropped.clone());
        let source = stream::repeat_with(move || {
            let _lease = &lease;
            Ok(bytes::Bytes::from_static(b"x"))
        });
        let limits = short_limits();
        let mut body = BoundedStream::new(source, &limits, Instant::now() + limits.total_timeout);
        wait_for_drop(&dropped).await;
        assert_eq!(
            body.next().await.unwrap().unwrap_err().kind(),
            std::io::ErrorKind::TimedOut
        );
        assert!(body.next().await.is_none());
        drop(body);
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn total_deadline_expires_even_while_chunks_keep_progressing() {
        let dropped = Arc::new(AtomicUsize::new(0));
        let lease = Lease(dropped.clone());
        let source = stream::repeat_with(move || {
            let _lease = &lease;
            Ok(bytes::Bytes::from_static(b"x"))
        });
        let limits = TransferLimits {
            idle_timeout: Duration::from_secs(2),
            ..short_limits()
        };
        let mut body =
            BoundedStream::new(source, &limits, Instant::now() + Duration::from_millis(40));
        while body.next().await.unwrap().is_ok() {
            tokio::time::sleep(Duration::from_millis(3)).await;
        }
        assert!(body.next().await.is_none());
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn timeout_does_not_release_a_running_blocking_read() {
        let dropped = Arc::new(AtomicUsize::new(0));
        let lease = Lease(dropped.clone());
        let limits = short_limits();
        let admission = Admission::new(&limits);
        let permit = admission.acquire("alice").unwrap();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let source = stream::once(async move {
            tokio::task::spawn_blocking(move || {
                let _ownership = (lease, permit);
                started_tx.send(()).unwrap();
                finish_rx.recv_timeout(Duration::from_secs(3)).unwrap();
                Ok::<_, std::io::Error>(bytes::Bytes::from_static(b"x"))
            })
            .await
            .unwrap()
        });
        let mut body = BoundedStream::new(source, &limits, Instant::now() + limits.total_timeout);
        let pending = tokio::spawn(async move {
            assert_eq!(
                body.next().await.unwrap().unwrap_err().kind(),
                std::io::ErrorKind::TimedOut
            );
            assert!(body.next().await.is_none());
            body
        });
        started_rx.await.unwrap();
        let body = pending.await.unwrap();
        assert_eq!(dropped.load(Ordering::SeqCst), 0);
        assert_eq!(admission.counts.lock().unwrap().total, 1);
        finish_tx.send(()).unwrap();
        wait_for_drop(&dropped).await;
        assert_eq!(admission.counts.lock().unwrap().total, 0);
        drop(body);
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn ready_stream_cannot_starve_total_or_empty_frame_idle_deadline() {
        // The current-thread runtime cannot run the watchdog while this loop
        // consumes always-ready chunks without yielding. poll_next must enforce
        // the deadline itself, including when empty chunks supply no progress.
        for empty in [false, true] {
            let limits = TransferLimits {
                idle_timeout: if empty {
                    Duration::from_millis(3)
                } else {
                    Duration::from_secs(1)
                },
                ..TransferLimits::default()
            };
            let deadline = Instant::now()
                + if empty {
                    Duration::from_secs(1)
                } else {
                    Duration::from_millis(3)
                };
            let source = stream::repeat_with(move || {
                Ok(if empty {
                    bytes::Bytes::new()
                } else {
                    bytes::Bytes::from_static(b"x")
                })
            });
            let mut body = BoundedStream::new(source, &limits, deadline);
            loop {
                if let Err(error) = body.next().await.unwrap() {
                    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
                    break;
                }
            }
            assert!(body.next().await.is_none());
        }
    }

    #[tokio::test]
    async fn explicit_drop_releases_idle_stream_without_waiting_for_timer() {
        let dropped = Arc::new(AtomicUsize::new(0));
        let lease = Lease(dropped.clone());
        let source = stream::pending::<Result<bytes::Bytes, std::io::Error>>().map(move |chunk| {
            let _lease = &lease;
            chunk
        });
        let limits = short_limits();
        let body = BoundedStream::new(source, &limits, Instant::now() + limits.total_timeout);
        drop(body);
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    }
}
