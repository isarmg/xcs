use service_stack::{layer_fn, service_fn, Layer, Service, ServiceBuilder, ServiceExt};
use std::{future::{Future, ready}, pin::Pin, sync::{Arc, atomic::{AtomicUsize, Ordering}}, task::{Context, Poll, Wake, Waker}};
struct Noop;
impl Wake for Noop { fn wake(self: Arc<Self>) {} }
fn run<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    match Box::pin(future).as_mut().poll(&mut cx) { Poll::Ready(value) => value, Poll::Pending => panic!("unexpected pending") }
}
#[test]
fn layer_order_and_response_are_preserved() {
    let mut service = ServiceBuilder::new()
        .map_request(|value: u32| value + 1)
        .map_request(|value: u32| value * 2)
        .service(service_fn(|value| ready(Ok::<_, &'static str>(value))));
    assert_eq!(run(run(service.ready()).unwrap().call(4)), Ok(10));
}
#[test]
fn exact_error_propagates_through_layers() {
    let service = ServiceBuilder::new().map_err(|error: &'static str| format!("wrapped:{error}"))
        .service(service_fn(|_: ()| ready(Err::<(), _>("upstream"))));
    assert_eq!(run(service.oneshot(())), Err("wrapped:upstream".into()));
}
struct PendingDrop(Arc<AtomicUsize>);
impl Future for PendingDrop { type Output = Result<(), &'static str>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
impl Drop for PendingDrop { fn drop(&mut self) { self.0.fetch_add(1, Ordering::SeqCst); } }
#[test]
fn cancelling_pending_response_releases_inner_future_once() {
    let dropped = Arc::new(AtomicUsize::new(0));
    let count = dropped.clone();
    let service = service_fn(move |_: ()| PendingDrop(count.clone()));
    let mut response = Box::pin(service.oneshot(()));
    let waker = Waker::from(Arc::new(Noop));
    assert!(response.as_mut().poll(&mut Context::from_waker(&waker)).is_pending());
    assert_eq!(dropped.load(Ordering::SeqCst), 0);
    drop(response);
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}
#[test]
fn trait_identity_and_layer_composition_are_single_owner() {
    fn accepts<S: service_stack::Service<()>>(service: S) -> S { service }
    let service = layer_fn(|inner| inner).layer(service_fn(|()| ready(Ok::<_, ()>(7))));
    assert_eq!(run(accepts(service).oneshot(())), Ok(7));
}
