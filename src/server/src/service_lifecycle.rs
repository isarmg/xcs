//! Keep optional listeners owned by the HTTP service, including during drain.
use std::{future::Future, io};
use tokio::sync::oneshot;

pub(crate) async fn run<H, Http, Bridge, Shutdown>(
    http: H,
    bridge: Bridge,
    shutdown: Shutdown,
) -> io::Result<()>
where
    H: FnOnce(oneshot::Receiver<()>) -> Http,
    Http: Future<Output = io::Result<()>>,
    Bridge: Future<Output = ()>,
    Shutdown: Future<Output = ()>,
{
    // Closing this sender also requests HTTP shutdown if the caller cancels us.
    // The HTTP library owns its connection tasks and signal waiter separately.
    let (stop_http, stopped) = oneshot::channel();
    let mut http = Box::pin(http(stopped));
    // Do not spawn/detach this listener: dropping its future closes admission,
    // shuts down connected SMB sockets, and removes its owned socket path.
    let mut bridge = Box::pin(bridge);
    let mut bridge_finished = false;
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => {
                // Both protocols stop on the same signal, not after HTTP's
                // potentially long drain. Actual blocking operations retain
                // their permits and durability/recovery ownership until done.
                drop(stop_http);
                drop(bridge);
                return http.await;
            }
            result = &mut http => return result,
            _ = &mut bridge, if !bridge_finished => {
                // Optional bridge failure must not take management HTTP down.
                bridge_finished = true;
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../test/server/unit/service_lifecycle.rs"]
mod tests;
