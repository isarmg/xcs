use http_routing::{Router, routing::{get, post}, body::Body, http::{Request, StatusCode}, extract::{Path, Query}};
use http_body::BodyExt;
use service_stack::ServiceExt;
use std::{collections::HashMap, future::Future, sync::Arc, task::{Context, Poll, Wake, Waker}};
struct Noop;
impl Wake for Noop { fn wake(self: Arc<Self>) {} }
fn run<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(Noop));
    match Box::pin(future).as_mut().poll(&mut Context::from_waker(&waker)) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("in-memory routing unexpectedly pending"),
    }
}
#[test]
fn routes_path_and_query_without_runtime() {
    let app = Router::new().route("/item/{id}", get(|Path(id): Path<String>, Query(query): Query<HashMap<String,String>>| async move { format!("{id}:{}", query["mode"]) }));
    let response = run(app.oneshot(Request::builder().uri("/item/42?mode=read").body(Body::empty()).unwrap())).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(run(response.into_body().collect()).unwrap().to_bytes(), "42:read");
}
#[test]
fn distinguishes_missing_route_and_wrong_method() {
    let app = Router::new().route("/", get(|| async { "ok" }));
    let wrong = run(app.clone().oneshot(Request::builder().method("POST").body(Body::empty()).unwrap())).unwrap();
    let missing = run(app.oneshot(Request::builder().uri("/missing").body(Body::empty()).unwrap())).unwrap();
    assert_eq!(wrong.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}
#[test]
fn request_body_and_response_status_are_preserved() {
    let app = Router::new().route("/", post(|body: String| async move { (StatusCode::CREATED, body) }));
    let response = run(app.oneshot(Request::builder().method("POST").body(Body::from("payload")).unwrap())).unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(run(response.into_body().collect()).unwrap().to_bytes(), "payload");
}
#[test]
fn extraction_error_produces_client_error() {
    let app = Router::new().route("/item/{id}", get(|Path(id): Path<u64>| async move { id.to_string() }));
    let response = run(app.oneshot(Request::builder().uri("/item/not-a-number").body(Body::empty()).unwrap())).unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
#[http_routing::debug_handler]
async fn generated_handler() -> &'static str { "macro" }
#[derive(Clone, http_routing::extract::FromRef)]
struct State { value: String }
#[test]
fn procedural_macros_target_the_new_single_owner() {
    use http_routing::extract::FromRef;
    assert_eq!(String::from_ref(&State { value: "state".into() }), "state");
    let response = run(Router::new().route("/", get(generated_handler)).oneshot(Request::new(Body::empty()))).unwrap();
    assert_eq!(run(response.into_body().collect()).unwrap().to_bytes(), "macro");
}
