#[cfg(all(test))]
mod tests {
    use super::*;
    use crate::runtime::{self, Runtime};
    use crate::sync::{oneshot, watch};

    use futures::future;

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/functions/src/signal/registry/smoke_174.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/functions/src/signal/registry/register_panics_on_invalid_input_225.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/functions/src/signal/registry/record_invalid_event_does_nothing_233.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/functions/src/signal/registry/broadcast_returns_if_at_least_one_event_fired_239.rs"));

    fn rt() -> Runtime {
        runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
    }

    async fn collect(mut rx: watch::Receiver<()>) -> Vec<()> {
        let mut ret = vec![];

        while let Ok(v) = rx.changed().await {
            ret.push(v);
        }

        ret
    }
}
