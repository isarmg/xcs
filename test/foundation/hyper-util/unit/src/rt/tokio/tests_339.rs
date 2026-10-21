#[cfg(test)]
mod tests {
    use crate::rt::TokioExecutor;
    use hyper::rt::Executor;
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn simple_execute() -> Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = oneshot::channel();
        let executor = TokioExecutor::new();
        executor.execute(async move {
            tx.send(()).unwrap();
        });
        rx.await.map_err(Into::into)
    }

    #[cfg(feature = "tracing")]
    #[tokio::test]
    async fn execute_tracing_span() {
        // The current-thread runtime keeps the subscriber active while the
        // spawned future is polled, after the caller has exited its span.
        let _subscriber = tracing::subscriber::set_default(tracing_subscriber::registry());
        let span = tracing::info_span!("caller");
        assert!(span.id().is_some());
        let (tx, rx) = oneshot::channel();

        {
            let _entered = span.enter();
            TokioExecutor::new().execute(async move {
                tx.send(tracing::Span::current().id()).unwrap();
            });
        }

        let spawned_span = rx.await.unwrap();
        if cfg!(feature = "rt-tracing-exec-force") {
            assert_eq!(spawned_span, span.id());
        } else {
            assert_eq!(spawned_span, None);
        }
    }
}
