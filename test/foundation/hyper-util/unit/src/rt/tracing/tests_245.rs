#[cfg(test)]
mod tests {
    use super::{CurrentSpanExecutor, MkSpanExecutor, WithSpanExecutor};
    use hyper::rt::Executor;
    use std::{
        cell::RefCell,
        future::poll_fn,
        pin::Pin,
        sync::{Arc, Mutex},
        task::Poll,
    };

    #[derive(Default)]
    struct DeferredExecutor<'a> {
        future: RefCell<Option<Pin<Box<dyn Future<Output = ()> + 'a>>>>,
    }

    impl<'a, F: Future<Output = ()> + 'a> Executor<F> for &DeferredExecutor<'a> {
        fn execute(&self, future: F) {
            *self.future.borrow_mut() = Some(Box::pin(future));
        }
    }

    #[test]
    fn current_span_executor_propagates_span_from_execute_on_each_poll() {
        let _subscriber = tracing::subscriber::set_default(tracing_subscriber::registry());
        let construction_span = tracing::info_span!("construction");
        let execution_span = tracing::info_span!("execution");
        let polling_span = tracing::info_span!("polling");
        assert!(execution_span.id().is_some());

        // Borrowing a local executor and future also checks that the wrapper
        // does not impose Send or 'static bounds on the inner executor.
        let polls = RefCell::new(0);
        let inner = DeferredExecutor::default();
        let executor = construction_span.in_scope(|| CurrentSpanExecutor::new(&inner));
        execution_span.in_scope(|| {
            executor.execute(poll_fn(|_| {
                assert_eq!(tracing::Span::current().id(), execution_span.id());
                *polls.borrow_mut() += 1;
                if *polls.borrow() == 1 {
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            }));
        });

        let _entered = polling_span.enter();
        let mut task = tokio_test::task::spawn(inner.future.borrow_mut().take().unwrap());
        assert!(task.poll().is_pending());
        assert_eq!(tracing::Span::current().id(), polling_span.id());
        assert!(task.poll().is_ready());
        assert_eq!(tracing::Span::current().id(), polling_span.id());
        assert_eq!(*polls.borrow(), 2);
    }

    #[test]
    fn with_span_executor_propagates_given_span_on_each_poll() {
        let _subscriber = tracing::subscriber::set_default(tracing_subscriber::registry());
        let construction_span = tracing::info_span!("construction");
        let execution_span = tracing::info_span!("execution");
        let polling_span = tracing::info_span!("polling");
        let with_span = tracing::info_span!("with");
        assert!(execution_span.id().is_some());

        // Borrowing a local executor and future also checks that the wrapper
        // does not impose Send or 'static bounds on the inner executor.
        let polls = RefCell::new(0);
        let inner = DeferredExecutor::default();
        let executor =
            construction_span.in_scope(|| WithSpanExecutor::new(&inner, with_span.clone()));
        execution_span.in_scope(|| {
            executor.execute(poll_fn(|_| {
                // Execution happens within the given span.
                assert_eq!(tracing::Span::current().id(), with_span.id());
                *polls.borrow_mut() += 1;
                if *polls.borrow() == 1 {
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            }));
        });

        let _entered = polling_span.enter();
        let mut task = tokio_test::task::spawn(inner.future.borrow_mut().take().unwrap());
        assert!(task.poll().is_pending());
        assert_eq!(tracing::Span::current().id(), polling_span.id());
        assert!(task.poll().is_ready());
        assert_eq!(tracing::Span::current().id(), polling_span.id());
        assert_eq!(*polls.borrow(), 2);
    }

    #[test]
    fn with_span_executor_current_propagates_construction_span() {
        let _subscriber = tracing::subscriber::set_default(tracing_subscriber::registry());
        let construction_span = tracing::info_span!("construction");
        let execution_span = tracing::info_span!("execution");
        let polling_span = tracing::info_span!("polling");
        assert!(execution_span.id().is_some());

        // Borrowing a local executor and future also checks that the wrapper
        // does not impose Send or 'static bounds on the inner executor.
        let polls = RefCell::new(0);
        let inner = DeferredExecutor::default();
        let executor = construction_span.in_scope(|| WithSpanExecutor::current(&inner));
        execution_span.in_scope(|| {
            executor.execute(poll_fn(|_| {
                // Execution happens within the given span.
                assert_eq!(tracing::Span::current().id(), construction_span.id());
                *polls.borrow_mut() += 1;
                if *polls.borrow() == 1 {
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            }));
        });

        let _entered = polling_span.enter();
        let mut task = tokio_test::task::spawn(inner.future.borrow_mut().take().unwrap());
        assert!(task.poll().is_pending());
        assert_eq!(tracing::Span::current().id(), polling_span.id());
        assert!(task.poll().is_ready());
        assert_eq!(tracing::Span::current().id(), polling_span.id());
        assert_eq!(*polls.borrow(), 2);
    }

    #[test]
    fn mk_span_executor_current_propagates_child_span() {
        let _subscriber = tracing::subscriber::set_default(tracing_subscriber::registry());
        let construction_span = tracing::info_span!("construction");
        let execution_span = tracing::info_span!("execution");
        let polling_span = tracing::info_span!("polling");
        assert!(execution_span.id().is_some());

        // A callback that creates a new child of the given span.
        let mk = || tracing::info_span!(parent: tracing::Span::current(), "child");

        // Borrowing a local executor and future also checks that the wrapper
        // does not impose Send or 'static bounds on the inner executor.
        let polls = RefCell::new(0);
        let inner = DeferredExecutor::default();
        let executor = construction_span.in_scope(|| MkSpanExecutor::new(&inner, mk));
        execution_span.in_scope(|| {
            executor.execute(poll_fn(|_| {
                // Execution happens within the created child span.
                let span = tracing::Span::current();
                assert_eq!(span.metadata().unwrap().name(), "child");
                *polls.borrow_mut() += 1;
                if *polls.borrow() == 1 {
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            }));
        });

        let _entered = polling_span.enter();
        let mut task = tokio_test::task::spawn(inner.future.borrow_mut().take().unwrap());
        assert!(task.poll().is_pending());
        assert_eq!(tracing::Span::current().id(), polling_span.id());
        assert!(task.poll().is_ready());
        assert_eq!(tracing::Span::current().id(), polling_span.id());
        assert_eq!(*polls.borrow(), 2);
    }

    /// A subscriber that records causal `follows_from` relationships.
    struct FollowsFromSubscriber<S> {
        inner: S,
        follows_from: Arc<Mutex<Vec<FollowsFrom>>>,
    }

    /// A tuple representing a causal relationship between two spans.
    ///
    /// This means that the span with the former id followed from the span with the latter id.
    type FollowsFrom = (tracing::span::Id, tracing::span::Id);

    impl<S> FollowsFromSubscriber<S> {
        fn new(inner: S) -> Self {
            Self {
                inner,
                follows_from: Default::default(),
            }
        }

        /// Returns a reference to the set of relationships observed.
        fn follows_from(&self) -> Arc<Mutex<Vec<FollowsFrom>>> {
            Arc::clone(&self.follows_from)
        }
    }

    impl<S> tracing::Subscriber for FollowsFromSubscriber<S>
    where
        S: tracing::Subscriber,
    {
        fn record_follows_from(&self, span: &tracing::span::Id, follows: &tracing::span::Id) {
            self.follows_from
                .lock()
                .unwrap()
                .push((span.clone(), follows.clone()));
            self.inner.record_follows_from(span, follows);
        }

        fn current_span(&self) -> tracing_core::span::Current {
            self.inner.current_span()
        }

        // Other methods delegate to `inner`...

        fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
            self.inner.enabled(metadata)
        }

        fn enter(&self, span: &tracing::span::Id) {
            self.inner.enter(span);
        }

        fn event(&self, event: &tracing::Event<'_>) {
            self.inner.event(event);
        }

        fn exit(&self, span: &tracing::span::Id) {
            self.inner.exit(span);
        }

        fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            self.inner.new_span(span)
        }

        fn record(&self, span: &tracing::span::Id, values: &tracing::span::Record<'_>) {
            self.inner.record(span, values);
        }
    }

    #[test]
    fn mk_span_executor_current_propagates_causal_span_relationships() {
        // Use a subscriber that records `follows_from` relationships.
        let subscriber = FollowsFromSubscriber::new(tracing_subscriber::registry());
        let relationships = subscriber.follows_from();
        let _subscriber = tracing::subscriber::set_default(subscriber);

        let construction_span = tracing::info_span!("construction");
        let execution_a_span = tracing::info_span!("execution_a");
        let execution_b_span = tracing::info_span!("execution_b");
        let polling_span = tracing::info_span!("polling");

        // A callback that creates a span that `follows_from` the execution span.
        let mk = || {
            let span = tracing::info_span!("spawned");
            span.follows_from(tracing::Span::current());
            span
        };

        let polls = RefCell::new(0);
        let inner = DeferredExecutor::default();
        let executor = construction_span.in_scope(|| MkSpanExecutor::new(&inner, mk));
        execution_a_span.in_scope(|| {
            executor.execute(poll_fn(|_| {
                // Execution happens within the created child span.
                let span = tracing::Span::current();
                assert_eq!(span.metadata().unwrap().name(), "spawned");
                *polls.borrow_mut() += 1;
                if *polls.borrow() == 1 {
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            }));
        });

        let _entered = polling_span.enter();
        let mut task = tokio_test::task::spawn(inner.future.borrow_mut().take().unwrap());
        assert!(task.poll().is_pending());
        assert_eq!(tracing::Span::current().id(), polling_span.id());
        assert_eq!(relationships.lock().unwrap().len(), 1);
        assert!(task.poll().is_ready());
        assert_eq!(tracing::Span::current().id(), polling_span.id());
        assert_eq!(*polls.borrow(), 2);
        assert_eq!(relationships.lock().unwrap().len(), 1);

        execution_b_span.in_scope(|| {
            executor.execute(poll_fn(|_| {
                let span = tracing::Span::current();
                assert_eq!(span.metadata().unwrap().name(), "spawned");
                Poll::Ready(())
            }));
        });

        let _entered = polling_span.enter();
        let mut task = tokio_test::task::spawn(inner.future.borrow_mut().take().unwrap());
        assert!(task.poll().is_ready());

        // The first task followed from the `execution_a` span. The second task
        // followed from the `execution_b` span.
        let relationships = relationships.lock().unwrap();
        assert_eq!(relationships.len(), 2);
        assert_eq!(relationships[0].1, execution_a_span.id().unwrap());
        assert_eq!(relationships[1].1, execution_b_span.id().unwrap());
    }
}
