#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::{layer::Context, registry::LookupSpan, Layer};
    use std::{
        collections::HashMap,
        dbg, println,
        sync::{Arc, Mutex, Weak},
        vec::Vec,
    };
    use tracing::{self, subscriber::with_default};
    use tracing::primitives::{
        dispatcher,
        span::{Attributes, Id},
        Subscriber,
    };

    struct AssertionLayer;
    impl<S> Layer<S> for AssertionLayer
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_close(&self, id: Id, ctx: Context<'_, S>) {
            dbg!(format_args!("closing {:?}", id));
            assert!(&ctx.span(&id).is_some());
        }
    }

    #[test]
    fn single_layer_can_access_closed_span() {
        let subscriber = AssertionLayer.with_subscriber(Registry::default());

        with_default(subscriber, || {
            let span = tracing::debug_span!("span");
            drop(span);
        });
    }

    #[test]
    fn multiple_layers_can_access_closed_span() {
        let subscriber = AssertionLayer
            .and_then(AssertionLayer)
            .with_subscriber(Registry::default());

        with_default(subscriber, || {
            let span = tracing::debug_span!("span");
            drop(span);
        });
    }

    struct CloseLayer {
        inner: Arc<Mutex<CloseState>>,
    }

    struct CloseHandle {
        state: Arc<Mutex<CloseState>>,
    }

    #[derive(Default)]
    struct CloseState {
        open: HashMap<&'static str, Weak<()>>,
        closed: Vec<(&'static str, Weak<()>)>,
    }

    #[allow(dead_code)] // Field is exercised via checking `Arc::downgrade()`
    struct SetRemoved(Arc<()>);

    impl<S> Layer<S> for CloseLayer
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_new_span(&self, _: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
            let span = ctx.span(id).expect("Missing span; this is a bug");
            let mut lock = self.inner.lock().unwrap();
            let is_removed = Arc::new(());
            assert!(
                lock.open
                    .insert(span.name(), Arc::downgrade(&is_removed))
                    .is_none(),
                "test layer saw multiple spans with the same name, the test is probably messed up"
            );
            let mut extensions = span.extensions_mut();
            extensions.insert(SetRemoved(is_removed));
        }

        fn on_close(&self, id: Id, ctx: Context<'_, S>) {
            let span = if let Some(span) = ctx.span(&id) {
                span
            } else {
                println!(
                    "span {:?} did not exist in `on_close`, are we panicking?",
                    id
                );
                return;
            };
            let name = span.name();
            println!("close {} ({:?})", name, id);
            if let Ok(mut lock) = self.inner.lock() {
                if let Some(is_removed) = lock.open.remove(name) {
                    assert!(is_removed.upgrade().is_some());
                    lock.closed.push((name, is_removed));
                }
            }
        }
    }

    impl CloseLayer {
        fn new() -> (Self, CloseHandle) {
            let state = Arc::new(Mutex::new(CloseState::default()));
            (
                Self {
                    inner: state.clone(),
                },
                CloseHandle { state },
            )
        }
    }

    impl CloseState {
        fn is_open(&self, span: &str) -> bool {
            self.open.contains_key(span)
        }

        fn is_closed(&self, span: &str) -> bool {
            self.closed.iter().any(|(name, _)| name == &span)
        }
    }

    impl CloseHandle {
        fn assert_closed(&self, span: &str) {
            let lock = self.state.lock().unwrap();
            assert!(
                lock.is_closed(span),
                "expected {} to be closed{}",
                span,
                if lock.is_open(span) {
                    " (it was still open)"
                } else {
                    ", but it never existed (is there a problem with the test?)"
                }
            )
        }

        fn assert_open(&self, span: &str) {
            let lock = self.state.lock().unwrap();
            assert!(
                lock.is_open(span),
                "expected {} to be open{}",
                span,
                if lock.is_closed(span) {
                    " (it was still open)"
                } else {
                    ", but it never existed (is there a problem with the test?)"
                }
            )
        }

        fn assert_removed(&self, span: &str) {
            let lock = self.state.lock().unwrap();
            let is_removed = match lock.closed.iter().find(|(name, _)| name == &span) {
                Some((_, is_removed)) => is_removed,
                None => panic!(
                    "expected {} to be removed from the registry, but it was not closed {}",
                    span,
                    if lock.is_closed(span) {
                        " (it was still open)"
                    } else {
                        ", but it never existed (is there a problem with the test?)"
                    }
                ),
            };
            assert!(
                is_removed.upgrade().is_none(),
                "expected {} to have been removed from the registry",
                span
            )
        }

        fn assert_not_removed(&self, span: &str) {
            let lock = self.state.lock().unwrap();
            let is_removed = match lock.closed.iter().find(|(name, _)| name == &span) {
                Some((_, is_removed)) => is_removed,
                None if lock.is_open(span) => return,
                None => unreachable!(),
            };
            assert!(
                is_removed.upgrade().is_some(),
                "expected {} to have been removed from the registry",
                span
            )
        }

        #[allow(unused)] // may want this for future tests
        fn assert_last_closed(&self, span: Option<&str>) {
            let lock = self.state.lock().unwrap();
            let last = lock.closed.last().map(|(span, _)| span);
            assert_eq!(
                last,
                span.as_ref(),
                "expected {:?} to have closed last",
                span
            );
        }

        fn assert_closed_in_order(&self, order: impl AsRef<[&'static str]>) {
            let lock = self.state.lock().unwrap();
            let order = order.as_ref();
            for (i, name) in order.iter().enumerate() {
                assert_eq!(
                    lock.closed.get(i).map(|(span, _)| span),
                    Some(name),
                    "expected close order: {:?}, actual: {:?}",
                    order,
                    lock.closed.iter().map(|(name, _)| name).collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn spans_are_removed_from_registry() {
        let (close_layer, state) = CloseLayer::new();
        let subscriber = AssertionLayer
            .and_then(close_layer)
            .with_subscriber(Registry::default());

        // Create a `Dispatch` (which is internally reference counted) so that
        // the subscriber lives to the end of the test. Otherwise, if we just
        // passed the subscriber itself to `with_default`, we could see the span
        // be dropped when the subscriber itself is dropped, destroying the
        // registry.
        let dispatch = dispatcher::Dispatch::new(subscriber);

        dispatcher::with_default(&dispatch, || {
            let span = tracing::debug_span!("span1");
            drop(span);
            let span = tracing::info_span!("span2");
            drop(span);
        });

        state.assert_removed("span1");
        state.assert_removed("span2");

        // Ensure the registry itself outlives the span.
        drop(dispatch);
    }

    #[test]
    fn spans_are_only_closed_when_the_last_ref_drops() {
        let (close_layer, state) = CloseLayer::new();
        let subscriber = AssertionLayer
            .and_then(close_layer)
            .with_subscriber(Registry::default());

        // Create a `Dispatch` (which is internally reference counted) so that
        // the subscriber lives to the end of the test. Otherwise, if we just
        // passed the subscriber itself to `with_default`, we could see the span
        // be dropped when the subscriber itself is dropped, destroying the
        // registry.
        let dispatch = dispatcher::Dispatch::new(subscriber);

        let span2 = dispatcher::with_default(&dispatch, || {
            let span = tracing::debug_span!("span1");
            drop(span);
            let span2 = tracing::info_span!("span2");
            let span2_clone = span2.clone();
            drop(span2);
            span2_clone
        });

        state.assert_removed("span1");
        state.assert_not_removed("span2");

        drop(span2);
        state.assert_removed("span1");

        // Ensure the registry itself outlives the span.
        drop(dispatch);
    }

    #[test]
    fn span_enter_guards_are_dropped_out_of_order() {
        let (close_layer, state) = CloseLayer::new();
        let subscriber = AssertionLayer
            .and_then(close_layer)
            .with_subscriber(Registry::default());

        // Create a `Dispatch` (which is internally reference counted) so that
        // the subscriber lives to the end of the test. Otherwise, if we just
        // passed the subscriber itself to `with_default`, we could see the span
        // be dropped when the subscriber itself is dropped, destroying the
        // registry.
        let dispatch = dispatcher::Dispatch::new(subscriber);

        dispatcher::with_default(&dispatch, || {
            let span1 = tracing::debug_span!("span1");
            let span2 = tracing::info_span!("span2");

            let enter1 = span1.enter();
            let enter2 = span2.enter();

            drop(enter1);
            drop(span1);

            state.assert_removed("span1");
            state.assert_not_removed("span2");

            drop(enter2);
            state.assert_not_removed("span2");

            drop(span2);
            state.assert_removed("span1");
            state.assert_removed("span2");
        });
    }

    #[test]
    fn child_closes_parent() {
        // This test asserts that if a parent span's handle is dropped before
        // a child span's handle, the parent will remain open until child
        // closes, and will then be closed.

        let (close_layer, state) = CloseLayer::new();
        let subscriber = close_layer.with_subscriber(Registry::default());

        let dispatch = dispatcher::Dispatch::new(subscriber);

        dispatcher::with_default(&dispatch, || {
            let span1 = tracing::info_span!("parent");
            let span2 = tracing::info_span!(parent: &span1, "child");

            state.assert_open("parent");
            state.assert_open("child");

            drop(span1);
            state.assert_open("parent");
            state.assert_open("child");

            drop(span2);
            state.assert_closed("parent");
            state.assert_closed("child");
        });
    }

    #[test]
    fn child_closes_grandparent() {
        // This test asserts that, when a span is kept open by a child which
        // is *itself* kept open by a child, closing the grandchild will close
        // both the parent *and* the grandparent.
        let (close_layer, state) = CloseLayer::new();
        let subscriber = close_layer.with_subscriber(Registry::default());

        let dispatch = dispatcher::Dispatch::new(subscriber);

        dispatcher::with_default(&dispatch, || {
            let span1 = tracing::info_span!("grandparent");
            let span2 = tracing::info_span!(parent: &span1, "parent");
            let span3 = tracing::info_span!(parent: &span2, "child");

            state.assert_open("grandparent");
            state.assert_open("parent");
            state.assert_open("child");

            drop(span1);
            drop(span2);
            state.assert_open("grandparent");
            state.assert_open("parent");
            state.assert_open("child");

            drop(span3);

            state.assert_closed_in_order(["child", "parent", "grandparent"]);
        });
    }
}
