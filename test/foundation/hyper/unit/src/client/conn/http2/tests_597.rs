#[cfg(test)]
mod tests {

    #[tokio::test]
    #[ignore] // only compilation is checked
    async fn send_sync_executor_of_non_send_futures() {
        #[derive(Clone)]
        struct LocalTokioExecutor;

        impl<F> crate::rt::Executor<F> for LocalTokioExecutor
        where
            F: std::future::Future + 'static, // not requiring `Send`
        {
            fn execute(&self, fut: F) {
                // This will spawn into the currently running `LocalSet`.
                tokio::task::spawn_local(fut);
            }
        }

        #[allow(unused)]
        async fn run(io: impl crate::rt::Read + crate::rt::Write + Unpin + 'static) {
            let (_sender, conn) = crate::client::conn::http2::handshake::<
                _,
                _,
                http_body::Empty<bytes::Bytes>,
            >(LocalTokioExecutor, io)
            .await
            .unwrap();

            tokio::task::spawn_local(async move {
                conn.await.unwrap();
            });
        }
    }

    #[tokio::test]
    #[ignore] // only compilation is checked
    async fn not_send_not_sync_executor_of_not_send_futures() {
        #[derive(Clone)]
        struct LocalTokioExecutor {
            _x: std::marker::PhantomData<std::rc::Rc<()>>,
        }

        impl<F> crate::rt::Executor<F> for LocalTokioExecutor
        where
            F: std::future::Future + 'static, // not requiring `Send`
        {
            fn execute(&self, fut: F) {
                // This will spawn into the currently running `LocalSet`.
                tokio::task::spawn_local(fut);
            }
        }

        #[allow(unused)]
        async fn run(io: impl crate::rt::Read + crate::rt::Write + Unpin + 'static) {
            let (_sender, conn) =
                crate::client::conn::http2::handshake::<_, _, http_body::Empty<bytes::Bytes>>(
                    LocalTokioExecutor {
                        _x: Default::default(),
                    },
                    io,
                )
                .await
                .unwrap();

            tokio::task::spawn_local(async move {
                conn.await.unwrap();
            });
        }
    }

    #[tokio::test]
    #[ignore] // only compilation is checked
    async fn send_not_sync_executor_of_not_send_futures() {
        #[derive(Clone)]
        struct LocalTokioExecutor {
            _x: std::marker::PhantomData<std::cell::Cell<()>>,
        }

        impl<F> crate::rt::Executor<F> for LocalTokioExecutor
        where
            F: std::future::Future + 'static, // not requiring `Send`
        {
            fn execute(&self, fut: F) {
                // This will spawn into the currently running `LocalSet`.
                tokio::task::spawn_local(fut);
            }
        }

        #[allow(unused)]
        async fn run(io: impl crate::rt::Read + crate::rt::Write + Unpin + 'static) {
            let (_sender, conn) =
                crate::client::conn::http2::handshake::<_, _, http_body::Empty<bytes::Bytes>>(
                    LocalTokioExecutor {
                        _x: Default::default(),
                    },
                    io,
                )
                .await
                .unwrap();

            tokio::task::spawn_local(async move {
                conn.await.unwrap();
            });
        }
    }

    #[tokio::test]
    #[ignore] // only compilation is checked
    async fn send_sync_executor_of_send_futures() {
        #[derive(Clone)]
        struct TokioExecutor;

        impl<F> crate::rt::Executor<F> for TokioExecutor
        where
            F: std::future::Future + 'static + Send,
            F::Output: Send + 'static,
        {
            fn execute(&self, fut: F) {
                tokio::task::spawn(fut);
            }
        }

        #[allow(unused)]
        async fn run(io: impl crate::rt::Read + crate::rt::Write + Send + Unpin + 'static) {
            let (_sender, conn) = crate::client::conn::http2::handshake::<
                _,
                _,
                http_body::Empty<bytes::Bytes>,
            >(TokioExecutor, io)
            .await
            .unwrap();

            tokio::task::spawn(async move {
                conn.await.unwrap();
            });
        }
    }

    #[tokio::test]
    #[ignore] // only compilation is checked
    async fn send_not_sync_executor_of_send_futures() {
        #[derive(Clone)]
        struct TokioExecutor {
            // !Sync
            _x: std::marker::PhantomData<std::cell::Cell<()>>,
        }

        impl<F> crate::rt::Executor<F> for TokioExecutor
        where
            F: std::future::Future + 'static + Send,
            F::Output: Send + 'static,
        {
            fn execute(&self, fut: F) {
                tokio::task::spawn(fut);
            }
        }

        #[allow(unused)]
        async fn run(io: impl crate::rt::Read + crate::rt::Write + Send + Unpin + 'static) {
            let (_sender, conn) =
                crate::client::conn::http2::handshake::<_, _, http_body::Empty<bytes::Bytes>>(
                    TokioExecutor {
                        _x: Default::default(),
                    },
                    io,
                )
                .await
                .unwrap();

            tokio::task::spawn_local(async move {
                // can't use spawn here because when executor is !Send
                conn.await.unwrap();
            });
        }
    }
}
