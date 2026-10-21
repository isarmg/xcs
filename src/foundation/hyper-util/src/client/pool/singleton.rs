//! Singleton pools
//!
//! This ensures that only one active connection is made.
//!
//! The singleton pool wraps a `MakeService<T, Req>` so that it only produces a
//! single `Service<Req>`. It bundles all concurrent calls to it, so that only
//! one connection is made. All calls to the singleton will return a clone of
//! the inner service once established.
//!
//! This fits the HTTP/2 case well.
//!
//! ## Example
//!
//! ```rust,ignore
//! let mut pool = Singleton::new(some_make_svc);
//!
//! let svc1 = pool.call(some_dst).await?;
//!
//! let svc2 = pool.call(some_dst).await?;
//! // svc1 == svc2
//! ```

use std::sync::{Arc, Mutex};
use std::task::{self, Poll};

use tower_service::Service;

use self::internal::{SingletonError, SingletonFuture, State};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[cfg(docsrs)]
pub use self::internal::Singled;

/// A singleton pool over an inner service.
///
/// The singleton wraps an inner service maker, bundling all calls to ensure
/// only one service is created. Once made, it returns clones of the made
/// service.
#[derive(Debug)]
pub struct Singleton<M, Dst>
where
    M: Service<Dst>,
{
    mk_svc: M,
    state: Arc<Mutex<State<M::Future, M::Response>>>,
}

impl<M, Target> Singleton<M, Target>
where
    M: Service<Target>,
    M::Response: Clone,
{
    /// Create a new singleton pool over an inner make service.
    pub fn new(mk_svc: M) -> Self {
        Singleton {
            mk_svc,
            state: Arc::new(Mutex::new(State::Empty)),
        }
    }

    // pub fn clear? cancel?

    /// Retains the inner made service if specified by the predicate.
    pub fn retain<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&mut M::Response) -> bool,
    {
        let mut locked = self.state.lock().unwrap();
        match *locked {
            State::Empty => {}
            State::Making(..) => {}
            State::Made(ref mut svc) => {
                if !predicate(svc) {
                    *locked = State::Empty;
                }
            }
        }
    }

    /// Returns whether this singleton pool is empty.
    ///
    /// If this pool has created a shared instance, or is currently in the
    /// process of creating one, this returns false.
    pub fn is_empty(&self) -> bool {
        matches!(*self.state.lock().unwrap(), State::Empty)
    }
}

impl<M, Target> Service<Target> for Singleton<M, Target>
where
    M: Service<Target>,
    M::Response: Clone,
    M::Error: Into<BoxError>,
{
    type Response = internal::Singled<M::Future, M::Response>;
    type Error = SingletonError;
    type Future = SingletonFuture<M::Future, M::Response>;

    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        if let State::Empty = *self.state.lock().unwrap() {
            return self
                .mk_svc
                .poll_ready(cx)
                .map_err(|e| SingletonError::new(e.into()));
        }
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, dst: Target) -> Self::Future {
        let mut locked = self.state.lock().unwrap();
        match *locked {
            State::Empty => {
                let fut = self.mk_svc.call(dst);
                let mut batch = internal::Batch::new(fut);
                let id = batch.register_driver();
                *locked = State::Making(batch);
                SingletonFuture::Participating {
                    id,
                    state: self.state.clone(),
                    rx: None,
                }
            }
            State::Making(ref mut batch) => {
                let (id, rx) = batch.register_waiter();
                SingletonFuture::Participating {
                    id,
                    state: self.state.clone(),
                    rx: Some(rx),
                }
            }
            State::Made(ref svc) => SingletonFuture::Made {
                svc: Some(svc.clone()),
                state: Arc::downgrade(&self.state),
            },
        }
    }
}

impl<M, Target> Clone for Singleton<M, Target>
where
    M: Service<Target> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            mk_svc: self.mk_svc.clone(),
            state: self.state.clone(),
        }
    }
}

// Holds some "pub" items that otherwise shouldn't be public.
/// Baton-passing implementation.
///
/// While a singleton service is being made, one participating future is
/// responsible for driving that work. If that future is canceled, the work
/// should not be canceled for every other caller waiting on the same service.
/// Baton-passing lets another participant take over, so cancellation remains
/// local to the future that was dropped.
mod internal {
    use std::fmt;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex, Weak};
    use std::task::{self, Poll, Waker};

    use tokio::sync::oneshot;
    use tower_service::Service;

    use super::BoxError;

    pub enum SingletonFuture<F, S> {
        Participating {
            id: WaiterId,
            state: Arc<Mutex<State<F, S>>>,
            rx: Option<oneshot::Receiver<Result<S, SharedError>>>,
        },
        Made {
            svc: Option<S>,
            state: Weak<Mutex<State<F, S>>>,
        },
    }

    impl<F, S> Unpin for SingletonFuture<F, S> {}

    // XXX: pub because of the enum SingletonFuture
    pub enum State<F, S> {
        Empty,
        Making(Batch<F, S>),
        Made(S),
    }

    impl<F, S: fmt::Debug> fmt::Debug for State<F, S> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                State::Empty => f.write_str("Empty"),
                State::Making(..) => f.write_str("Making"),
                State::Made(svc) => f.debug_tuple("Made").field(svc).finish(),
            }
        }
    }

    // XXX: pub because of the enum SingletonFuture
    pub struct Batch<F, S> {
        future: Option<Pin<Box<F>>>,
        next_id: WaiterId,
        driver: Option<Driver>,
        waiters: Vec<Waiter<S>>,
    }

    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub struct WaiterId(usize);

    struct Driver {
        id: WaiterId,
        waker: Option<Waker>,
    }

    struct Waiter<S> {
        id: WaiterId,
        waker: Option<Waker>,
        tx: oneshot::Sender<Result<S, SharedError>>,
    }

    /// A cached service returned from a [`Singleton`].
    ///
    /// Implements `Service` by delegating to the inner service. If
    /// `poll_ready` returns an error, this will clear the cache in the related
    /// `Singleton`.
    ///
    /// [`Singleton`]: super::Singleton
    ///
    /// # Unnameable
    ///
    /// This type is normally unnameable, forbidding naming of the type within
    /// code. The type is exposed in the documentation to show which methods
    /// can be publicly called.
    #[derive(Debug)]
    pub struct Singled<F, S> {
        inner: S,
        state: Weak<Mutex<State<F, S>>>,
    }

    impl<F, S, E> Future for SingletonFuture<F, S>
    where
        F: Future<Output = Result<S, E>>,
        E: Into<BoxError>,
        S: Clone,
    {
        type Output = Result<Singled<F, S>, SingletonError>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
            match &mut *self {
                SingletonFuture::Participating { id, state, rx } => {
                    if let Some(receiver) = rx.as_mut() {
                        match Pin::new(receiver).poll(cx) {
                            Poll::Ready(Ok(Ok(svc))) => {
                                return Poll::Ready(Ok(Singled::new(svc, Arc::downgrade(state))));
                            }
                            Poll::Ready(Ok(Err(err))) => {
                                return Poll::Ready(Err(SingletonError(err)));
                            }
                            Poll::Ready(Err(_canceled)) => {
                                *rx = None;
                            }
                            Poll::Pending => {}
                        }
                    }

                    let state_weak = Arc::downgrade(state);
                    let mut locked = state.lock().unwrap();

                    match &mut *locked {
                        State::Making(batch) => match batch.poll(*id, cx) {
                            Poll::Pending => Poll::Pending,
                            Poll::Ready(Ok(svc)) => {
                                batch.send_result(Ok(svc.clone()));
                                *locked = State::Made(svc.clone());
                                Poll::Ready(Ok(Singled::new(svc, state_weak)))
                            }
                            Poll::Ready(Err(err)) => {
                                batch.send_result(Err(err.clone()));
                                *locked = State::Empty;
                                Poll::Ready(Err(SingletonError(err)))
                            }
                        },
                        State::Made(svc) => Poll::Ready(Ok(Singled::new(svc.clone(), state_weak))),
                        State::Empty => {
                            unreachable!("singleton participant polled after making was canceled")
                        }
                    }
                }
                SingletonFuture::Made { svc, state } => {
                    Poll::Ready(Ok(Singled::new(svc.take().unwrap(), state.clone())))
                }
            }
        }
    }

    impl<F, S> Drop for SingletonFuture<F, S> {
        fn drop(&mut self) {
            if let SingletonFuture::Participating { id, state, .. } = self {
                if let Ok(mut locked) = state.lock() {
                    if let State::Making(batch) = &mut *locked {
                        if batch.remove(*id) {
                            *locked = State::Empty;
                        }
                    }
                }
            }
        }
    }

    impl<F, S> Singled<F, S> {
        fn new(inner: S, state: Weak<Mutex<State<F, S>>>) -> Self {
            Singled { inner, state }
        }
    }

    impl<F, S, Req> Service<Req> for Singled<F, S>
    where
        S: Service<Req>,
    {
        type Response = S::Response;
        type Error = S::Error;
        type Future = S::Future;

        fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
            // We notice if the cached service dies, and clear the singleton cache.
            match self.inner.poll_ready(cx) {
                Poll::Ready(Err(err)) => {
                    if let Some(state) = self.state.upgrade() {
                        *state.lock().unwrap() = State::Empty;
                    }
                    Poll::Ready(Err(err))
                }
                other => other,
            }
        }

        fn call(&mut self, req: Req) -> Self::Future {
            self.inner.call(req)
        }
    }

    impl<F, S> Batch<F, S> {
        pub(super) fn new(future: F) -> Self {
            Batch {
                future: Some(Box::pin(future)),
                next_id: WaiterId(0),
                driver: None,
                waiters: Vec::new(),
            }
        }

        pub(super) fn register_driver(&mut self) -> WaiterId {
            let id = self.next_id;
            self.next_id.0 += 1;
            self.driver = Some(Driver { id, waker: None });
            id
        }

        pub(super) fn register_waiter(
            &mut self,
        ) -> (WaiterId, oneshot::Receiver<Result<S, SharedError>>) {
            let id = self.next_id;
            self.next_id.0 += 1;
            let (tx, rx) = oneshot::channel();
            self.waiters.push(Waiter {
                id,
                waker: None,
                tx,
            });
            (id, rx)
        }

        fn remove(&mut self, id: WaiterId) -> bool {
            if let Some(pos) = self.waiters.iter().position(|waiter| waiter.id == id) {
                self.waiters.swap_remove(pos);
                return false;
            }

            if self.driver.as_ref().is_some_and(|driver| driver.id == id) {
                if let Some(waiter) = self.waiters.pop() {
                    let waker = waiter.waker;
                    self.driver = Some(Driver {
                        id: waiter.id,
                        waker,
                    });
                    self.wake_driver();
                    return false;
                }

                self.driver = None;
                self.future = None;
                return true;
            }

            false
        }

        fn poll<E>(
            &mut self,
            id: WaiterId,
            cx: &mut task::Context<'_>,
        ) -> Poll<Result<S, SharedError>>
        where
            F: Future<Output = Result<S, E>>,
            E: Into<BoxError>,
            S: Clone,
        {
            if !self.driver.as_ref().is_some_and(|driver| driver.id == id) {
                self.store_waker(id, cx.waker());
                return Poll::Pending;
            }

            let future = self.future.as_mut().expect("batch future missing");
            match future.as_mut().poll(cx) {
                Poll::Pending => {
                    self.store_driver_waker(cx.waker());
                    Poll::Pending
                }
                Poll::Ready(Ok(svc)) => {
                    self.future = None;
                    Poll::Ready(Ok(svc))
                }
                Poll::Ready(Err(err)) => {
                    let err = box_error_into_shared(err.into());
                    self.future = None;
                    Poll::Ready(Err(err))
                }
            }
        }

        fn send_result(&mut self, result: Result<S, SharedError>)
        where
            S: Clone,
        {
            for waiter in std::mem::take(&mut self.waiters) {
                let _ = waiter.tx.send(result.clone());
            }
        }

        fn store_waker(&mut self, id: WaiterId, waker: &Waker) {
            if let Some(waiter) = self.waiters.iter_mut().find(|waiter| waiter.id == id) {
                if waiter
                    .waker
                    .as_ref()
                    .is_none_or(|current| !current.will_wake(waker))
                {
                    waiter.waker = Some(waker.clone());
                }
            }
        }

        fn store_driver_waker(&mut self, waker: &Waker) {
            if let Some(driver) = &mut self.driver {
                if driver
                    .waker
                    .as_ref()
                    .is_none_or(|current| !current.will_wake(waker))
                {
                    driver.waker = Some(waker.clone());
                }
            }
        }

        fn wake_driver(&mut self) {
            if let Some(driver) = &mut self.driver {
                if let Some(waker) = driver.waker.take() {
                    waker.wake();
                }
            }
        }
    }

    // An opaque error type. By not exposing the type, nor being specifically
    // Box<dyn Error>, we can change the inner representation later.
    #[derive(Debug)]
    pub struct SingletonError(pub(super) SharedError);

    impl SingletonError {
        pub(super) fn new(error: BoxError) -> Self {
            SingletonError(box_error_into_shared(error))
        }
    }

    impl std::fmt::Display for SingletonError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("singleton connection error")
        }
    }

    impl std::error::Error for SingletonError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&*self.0)
        }
    }

    type SharedError = Arc<dyn std::error::Error + Send + Sync>;

    fn box_error_into_shared(error: BoxError) -> SharedError {
        error.into()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hyper-util/unit/src/client/pool/singleton/tests_506.rs"));
