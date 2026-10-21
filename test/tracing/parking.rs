use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use tracing::{collector::{layer::{Context, SubscriberExt}, registry::LookupSpan, Layer}, Event, Subscriber};

#[derive(Clone)]
struct CountEvents(Arc<AtomicUsize>);
impl<S> Layer<S> for CountEvents where S: Subscriber + for<'a> LookupSpan<'a> {
    fn on_new_span(&self, _: &tracing::span::Attributes<'_>, id: &tracing::span::Id, ctx: Context<'_, S>) {
        ctx.span(id).unwrap().extensions_mut().insert(23usize);
    }
    fn on_event(&self, _: &Event<'_>, ctx: Context<'_, S>) {
        let span = ctx.lookup_current().unwrap();
        assert_eq!(span.extensions().get::<usize>(), Some(&23));
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn parking_registry_preserves_span_extensions_across_threads() {
    let count = Arc::new(AtomicUsize::new(0));
    let dispatch = tracing::Dispatch::new(tracing::collector::registry().with(CountEvents(count.clone())));
    let threads: Vec<_> = (0..4).map(|worker| {
        let dispatch = dispatch.clone();
        std::thread::spawn(move || tracing::dispatcher::with_default(&dispatch, || {
            for iteration in 0..32 {
                let span = tracing::info_span!("worker", worker, iteration);
                let _entered = span.enter();
                tracing::info!("inside");
            }
        }))
    }).collect();
    for thread in threads { thread.join().unwrap(); }
    assert_eq!(count.load(Ordering::SeqCst), 128);
}
