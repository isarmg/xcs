use std::future::ready;
use service_stack::util::service_fn;
use service_stack::Service;

#[tokio::test(flavor = "current_thread")]
async fn simple() {
    let _t = super::support::trace_init();

    let mut add_one = service_fn(|req| ready(Ok::<_, ()>(req + 1)));
    let answer = add_one.call(1).await.unwrap();
    assert_eq!(answer, 2);
}
