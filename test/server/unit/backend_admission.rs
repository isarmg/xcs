use super::*;
use http_routing::{body::Body, http::StatusCode};
use service_stack::ServiceExt;
use std::sync::{
    Condvar,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
    started: tokio::sync::Semaphore,
    calls: AtomicUsize,
    finished: tokio::sync::Semaphore,
}
impl Gate {
    fn release(&self) {
        *self.open.lock().unwrap() = true;
        self.changed.notify_all();
    }
}
// Always unblock workers if a test assertion fails, too.
struct Release(Arc<Gate>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}
struct BackendProbe {
    gate: Arc<Gate>,
    outcome: u8,
}
impl Backend for BackendProbe {
    fn call(&self, _: BackendAction, _: &Identity, _: Value) -> Result<Value, ApiError> {
        self.gate.calls.fetch_add(1, Ordering::SeqCst);
        self.gate.started.add_permits(1);
        let open = self.gate.open.lock().unwrap();
        drop(self.gate.changed.wait_while(open, |open| !*open).unwrap());
        self.gate.finished.add_permits(1);
        match self.outcome {
            1 => Err(ApiError::conflict("TEST_CONFLICT", "backend error")),
            2 => panic!("test backend panic"),
            _ => Ok(json!({})),
        }
    }
    fn upload(&self, _: &Identity, _: &str, _: &str, _: &str, _: &Path) -> Result<Value, ApiError> {
        Err(ApiError::unsupported())
    }
    fn content(&self, _: &Identity, _: &str) -> Result<Content, ApiError> {
        Err(ApiError::unsupported())
    }
}
fn fixture(root: &Path, outcome: u8, open: bool) -> (Router, Arc<Gate>, Release) {
    let gate = Arc::new(Gate {
        open: Mutex::new(open),
        changed: Condvar::new(),
        started: tokio::sync::Semaphore::new(0),
        calls: AtomicUsize::new(0),
        finished: tokio::sync::Semaphore::new(0),
    });
    let auth = AuthStore::initialize(root, "admin", "a-long-test-password").unwrap();
    let app = router(
        Arc::new(BackendProbe {
            gate: gate.clone(),
            outcome,
        }),
        auth,
        ServerConfig::loopback(root),
    )
    .unwrap();
    (app, gate.clone(), Release(gate))
}
async fn request(app: Router, path: &str) -> Response {
    app.oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}
async fn overloaded(app: Router, path: &str) {
    let response = tokio::time::timeout(Duration::from_secs(5), request(app, path))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = http_routing::body::to_bytes(response.into_body(), 8192)
        .await
        .unwrap();
    let body: Value = json_codec::from_slice(&body).unwrap();
    assert_eq!(body["error"]["code"], "RESOURCE_LIMIT_EXCEEDED");
    assert_eq!(body["error"]["retryable"], true);
}

#[tokio::test]
async fn backend_admission_saturates_without_waiters_and_survives_http_cancellation() {
    let root = tempfile::tempdir().unwrap();
    let (app, gate, _release) = fixture(root.path(), 0, false);
    let mut jobs = Vec::new();
    for _ in 0..HTTP_BACKEND_SLOTS {
        let app = app.clone();
        jobs.push(tokio::spawn(async move {
            request(app, "/api/capabilities").await
        }));
    }
    tokio::time::timeout(
        Duration::from_secs(5),
        gate.started.acquire_many(HTTP_BACKEND_SLOTS as u32),
    )
    .await
    .unwrap()
    .unwrap()
    .forget();
    overloaded(app.clone(), "/api/capabilities").await;
    overloaded(app.clone(), "/readyz").await;
    assert_eq!(
        request(app.clone(), "/healthz").await.status(),
        StatusCode::OK
    );
    assert_eq!(gate.calls.load(Ordering::SeqCst), HTTP_BACKEND_SLOTS);
    let cancelled = jobs.pop().unwrap();
    cancelled.abort();
    assert!(cancelled.await.unwrap_err().is_cancelled());
    overloaded(app.clone(), "/api/capabilities").await;
    assert_eq!(gate.calls.load(Ordering::SeqCst), HTTP_BACKEND_SLOTS);
    gate.release();
    for job in jobs {
        assert_eq!(job.await.unwrap().status(), StatusCode::OK);
    }
    gate.finished
        .acquire_many(HTTP_BACKEND_SLOTS as u32)
        .await
        .unwrap()
        .forget();
    *gate.open.lock().unwrap() = false;
    // Refill the entire pool, proving even the cancelled request's job releases
    // its permit. Retry only until that job has actually returned, not by sleep.
    let mut jobs = Vec::new();
    for _ in 0..HTTP_BACKEND_SLOTS {
        let app = app.clone();
        jobs.push(tokio::spawn(async move {
            loop {
                let response = request(app.clone(), "/api/capabilities").await;
                if response.status() != StatusCode::TOO_MANY_REQUESTS {
                    return response;
                }
                tokio::task::yield_now().await;
            }
        }));
    }
    tokio::time::timeout(
        Duration::from_secs(5),
        gate.started.acquire_many(HTTP_BACKEND_SLOTS as u32),
    )
    .await
    .unwrap()
    .unwrap()
    .forget();
    overloaded(app, "/readyz").await;
    gate.release();
    for job in jobs {
        assert_eq!(job.await.unwrap().status(), StatusCode::OK);
    }
    assert_eq!(gate.calls.load(Ordering::SeqCst), HTTP_BACKEND_SLOTS * 2);
}

#[tokio::test]
async fn backend_admission_releases_after_success_error_and_panic() {
    for (outcome, status) in [
        (0, StatusCode::OK),
        (1, StatusCode::CONFLICT),
        (2, StatusCode::INTERNAL_SERVER_ERROR),
    ] {
        let root = tempfile::tempdir().unwrap();
        let (app, gate, _release) = fixture(root.path(), outcome, true);
        for _ in 0..HTTP_BACKEND_SLOTS * 2 {
            assert_eq!(
                request(app.clone(), "/api/capabilities").await.status(),
                status
            );
        }
        assert_eq!(gate.calls.load(Ordering::SeqCst), HTTP_BACKEND_SLOTS * 2);
    }
}
