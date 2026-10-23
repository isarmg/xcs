use super::*;
use crate::{ApiError, Backend, BackendAction, Content, auth::Identity};
use json_codec::Value;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

struct WriterProbe {
    stopped: AtomicBool,
    entered: Mutex<Option<oneshot::Sender<()>>>,
    release: Mutex<mpsc::Receiver<()>>,
    done: Mutex<Option<oneshot::Sender<()>>>,
}
impl Backend for WriterProbe {
    fn stop_management(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }
    fn drain_management(&self) -> Result<(), ApiError> {
        assert!(self.stopped.load(Ordering::SeqCst));
        self.entered
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .send(())
            .unwrap();
        self.release.lock().unwrap().recv_timeout(DEADLINE).unwrap();
        self.done.lock().unwrap().take().unwrap().send(()).unwrap();
        Ok(())
    }
    fn call(&self, _: BackendAction, _: &Identity, _: Value) -> Result<Value, ApiError> {
        Err(ApiError::unsupported())
    }
    fn upload(
        &self,
        _: &Identity,
        _: &str,
        _: &str,
        _: &str,
        _: &crate::UploadSource,
    ) -> Result<Value, ApiError> {
        Err(ApiError::unsupported())
    }
    fn content(&self, _: &Identity, _: &str) -> Result<Content, ApiError> {
        Err(ApiError::unsupported())
    }
}
fn writer() -> (
    Arc<WriterProbe>,
    oneshot::Receiver<()>,
    mpsc::Sender<()>,
    oneshot::Receiver<()>,
) {
    let (entered, started) = oneshot::channel();
    let (release, released) = mpsc::channel();
    let (done, finished) = oneshot::channel();
    (
        Arc::new(WriterProbe {
            stopped: AtomicBool::new(false),
            entered: Mutex::new(Some(entered)),
            release: Mutex::new(released),
            done: Mutex::new(Some(done)),
        }),
        started,
        release,
        finished,
    )
}

#[tokio::test]
async fn shutdown_waits_for_native_writer_after_http_has_drained() {
    let (backend, started, release, finished) = writer();
    let (stop, shutdown) = oneshot::channel();
    let server = tokio::spawn(run_with_management(
        |stopped| async move {
            let _ = stopped.await;
            Ok(())
        },
        pending(),
        async move {
            let _ = shutdown.await;
        },
        ManagementShutdown::new(backend.clone()),
    ));
    stop.send(()).unwrap();
    notice(started).await;
    assert!(backend.stopped.load(Ordering::SeqCst));
    assert!(!server.is_finished());
    release.send(()).unwrap();
    notice(finished).await;
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn http_error_still_drains_native_writer_before_returning_error() {
    let (backend, started, release, finished) = writer();
    let server = tokio::spawn(run_with_management(
        |_| async { Err(io::Error::other("HTTP failed")) },
        pending(),
        pending(),
        ManagementShutdown::new(backend),
    ));
    notice(started).await;
    assert!(!server.is_finished());
    release.send(()).unwrap();
    notice(finished).await;
    assert_eq!(
        server.await.unwrap().unwrap_err().to_string(),
        "HTTP failed"
    );
}

#[tokio::test]
async fn cancelled_service_keeps_native_drain_in_runtime_owned_blocking_work() {
    let (backend, started, release, finished) = writer();
    let (bridge, bridge_started, bridge_dropped) = bridge();
    let server = tokio::spawn(run_with_management(
        |_| pending(),
        bridge,
        pending(),
        ManagementShutdown::new(backend.clone()),
    ));
    notice(bridge_started).await;
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
    notice(bridge_dropped).await;
    notice(started).await;
    assert!(backend.stopped.load(Ordering::SeqCst));
    release.send(()).unwrap();
    notice(finished).await;
}

#[test]
fn guard_drop_without_runtime_drains_synchronously_without_panicking() {
    let (backend, mut started, release, mut finished) = writer();
    release.send(()).unwrap();
    drop(ManagementShutdown::new(backend.clone()));
    assert!(backend.stopped.load(Ordering::SeqCst));
    assert!(started.try_recv().is_ok());
    assert!(finished.try_recv().is_ok());
}

#[test]
fn discarded_queued_blocking_closure_still_owns_native_drain() {
    let (backend, mut started, release, mut finished) = writer();
    backend.stop_management();
    release.send(()).unwrap();
    let drain = NativeDrain(Some(backend));
    // Model cancellation before Tokio polls a queued blocking closure. Merely
    // capturing the backend Arc would detach its native writer at this point.
    let queued = move || drain.run();
    drop(queued);
    assert!(started.try_recv().is_ok());
    assert!(finished.try_recv().is_ok());
}

#[test]
fn closed_runtime_rejecting_blocking_enqueue_still_drains_native_writer() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let handle = runtime.handle().clone();
    drop(runtime);
    let _entered_runtime = handle.enter();
    let (backend, mut started, release, mut finished) = writer();
    release.send(()).unwrap();
    // try_current succeeds, but the stopped blocking pool discards this job.
    drop(ManagementShutdown::new(backend.clone()));
    assert!(backend.stopped.load(Ordering::SeqCst));
    assert!(started.try_recv().is_ok());
    assert!(finished.try_recv().is_ok());
}
