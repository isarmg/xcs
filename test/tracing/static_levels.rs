use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tracing::collector::prelude::*;
#[derive(Clone)]
struct Count(Arc<AtomicUsize>);
impl<S: tracing::Subscriber> tracing::collector::Layer<S> for Count {
    fn on_event(&self, _: &tracing::Event<'_>, _: tracing::collector::layer::Context<'_, S>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}
#[test]
fn static_level_caps_skip_field_evaluation() {
    let count = Arc::new(AtomicUsize::new(0));
    let evaluated = AtomicUsize::new(0);
    let subscriber = tracing::collector::registry().with(Count(count.clone()));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(value = evaluated.fetch_add(1, Ordering::Relaxed));
        tracing::warn!(value = evaluated.fetch_add(1, Ordering::Relaxed));
        tracing::error!(value = evaluated.fetch_add(1, Ordering::Relaxed));
    });
    let expected = if cfg!(debug_assertions) { 2 } else { 1 };
    assert_eq!(count.load(Ordering::Relaxed), expected);
    assert_eq!(evaluated.load(Ordering::Relaxed), expected);
}
