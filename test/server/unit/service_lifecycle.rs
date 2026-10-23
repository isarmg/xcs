use super::*;
use std::{future::pending, time::Duration};

const DEADLINE: Duration = Duration::from_secs(5);

struct DropNotice(Option<oneshot::Sender<()>>);
impl Drop for DropNotice {
    fn drop(&mut self) {
        if let Some(notice) = self.0.take() {
            let _ = notice.send(());
        }
    }
}

async fn notice(receiver: oneshot::Receiver<()>) {
    tokio::time::timeout(DEADLINE, receiver)
        .await
        .expect("lifecycle notification timed out")
        .expect("lifecycle notification dropped");
}

fn bridge() -> (
    impl Future<Output = ()>,
    oneshot::Receiver<()>,
    oneshot::Receiver<()>,
) {
    let (started, started_rx) = oneshot::channel();
    let (dropped, dropped_rx) = oneshot::channel();
    let guard = DropNotice(Some(dropped));
    let work = async move {
        let _guard = guard;
        started.send(()).unwrap();
        pending::<()>().await;
    };
    (work, started_rx, dropped_rx)
}

#[tokio::test]
async fn shutdown_closes_bridge_before_stalled_http_drain_finishes() {
    let (bridge, started, dropped) = bridge();
    let (stop, shutdown) = oneshot::channel();
    let (http_stopping, stopping) = oneshot::channel();
    let (release, drained) = oneshot::channel();
    let server = tokio::spawn(run(
        |stopped| async move {
            let _ = stopped.await;
            http_stopping.send(()).unwrap();
            let _ = drained.await;
            Ok(())
        },
        bridge,
        async move {
            let _ = shutdown.await;
        },
    ));
    notice(started).await;
    stop.send(()).unwrap();
    notice(dropped).await;
    notice(stopping).await;
    assert!(!server.is_finished(), "admitted HTTP work must still drain");
    release.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn http_completion_and_error_drop_owned_bridge() {
    for fail in [false, true] {
        let (bridge, started, dropped) = bridge();
        let (finish, finished) = oneshot::channel();
        let server = tokio::spawn(run(
            move |_stopped| async move {
                let _ = finished.await;
                if fail {
                    Err(io::Error::other("injected HTTP failure"))
                } else {
                    Ok(())
                }
            },
            bridge,
            pending(),
        ));
        notice(started).await;
        finish.send(()).unwrap();
        assert_eq!(server.await.unwrap().is_err(), fail);
        notice(dropped).await;
    }
}

#[tokio::test]
async fn cancellation_stops_owned_bridge_and_detached_http_signal_waiter() {
    let (bridge, started, dropped) = bridge();
    let (http_stopping, stopping) = oneshot::channel();
    let server = tokio::spawn(run(
        |stopped| async move {
            // Model the HTTP library's separately spawned signal task.
            tokio::spawn(async move {
                let _ = stopped.await;
                let _ = http_stopping.send(());
            });
            pending().await
        },
        bridge,
        pending(),
    ));
    notice(started).await;
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
    notice(dropped).await;
    notice(stopping).await;
}

#[tokio::test]
async fn optional_bridge_failure_leaves_http_until_shutdown() {
    let (stop, shutdown) = oneshot::channel();
    let (finished, bridge_finished) = oneshot::channel();
    let server = tokio::spawn(run(
        |stopped| async move {
            let _ = stopped.await;
            Ok(())
        },
        async move {
            finished.send(()).unwrap();
        },
        async move {
            let _ = shutdown.await;
        },
    ));
    notice(bridge_finished).await;
    assert!(!server.is_finished());
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

// Exercise the actual HTTP graceful-drain and Unix socket cleanup, rather than
// assuming that dropping an HTTP future cancels its spawned connection tasks.
#[cfg(feature = "core")]
#[path = "service_lifecycle/sockets.rs"]
mod sockets;

#[path = "service_lifecycle/management.rs"]
mod management;
