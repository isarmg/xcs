use std::sync::{Arc, Mutex};
use tracing::collector::prelude::*;
#[derive(Clone)]
struct Events(Arc<Mutex<Vec<String>>>);
impl<S: tracing::Subscriber> tracing::collector::Layer<S> for Events {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing::collector::layer::Context<'_, S>) {
        self.0
            .lock()
            .unwrap()
            .push(event.metadata().target().into());
    }
}
#[test]
fn log_bridge_does_not_recurse_with_facade_log_fallback() {
    tracing::log_bridge::LogTracer::init().unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing::collector::registry().with(Events(events.clone()));
    tracing::subscriber::with_default(subscriber, || {
        log::warn!(target: "legacy", "one");
        tracing::warn!(target: "structured", "two");
    });
    assert_eq!(*events.lock().unwrap(), vec!["log", "structured"]);
}
