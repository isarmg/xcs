#[cfg(test)]
mod tests {
    use super::*;
    use crate::{routing::get, serve::IncomingStream, test_helpers::TestClient, Router};
    use tokio::net::TcpListener;

    #[crate::test]
    async fn socket_addr() {
        async fn handler(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> String {
            format!("{addr}")
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let app = Router::new().route("/", get(handler));
            tx.send(()).unwrap();
            crate::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        rx.await.unwrap();

        let client = reqwest::Client::new();

        let res = client.get(format!("http://{addr}")).send().await.unwrap();
        let body = res.text().await.unwrap();
        assert!(body.starts_with("127.0.0.1:"));
    }

    #[crate::test]
    async fn custom() {
        #[derive(Clone, Debug)]
        struct MyConnectInfo {
            value: &'static str,
        }

        impl Connected<IncomingStream<'_, TcpListener>> for MyConnectInfo {
            fn connect_info(_target: IncomingStream<'_, TcpListener>) -> Self {
                Self {
                    value: "it worked!",
                }
            }
        }

        async fn handler(ConnectInfo(addr): ConnectInfo<MyConnectInfo>) -> &'static str {
            addr.value
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let app = Router::new().route("/", get(handler));
            tx.send(()).unwrap();
            crate::serve(
                listener,
                app.into_make_service_with_connect_info::<MyConnectInfo>(),
            )
            .await
            .unwrap();
        });
        rx.await.unwrap();

        let client = reqwest::Client::new();

        let res = client.get(format!("http://{addr}")).send().await.unwrap();
        let body = res.text().await.unwrap();
        assert_eq!(body, "it worked!");
    }

    #[crate::test]
    async fn mock_connect_info() {
        async fn handler(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> String {
            format!("{addr}")
        }

        let app = Router::new()
            .route("/", get(handler))
            .layer(MockConnectInfo(SocketAddr::from(([0, 0, 0, 0], 1337))));

        let client = TestClient::new(app);

        let res = client.get("/").await;
        let body = res.text().await;
        assert!(body.starts_with("0.0.0.0:1337"));
    }

    #[crate::test]
    async fn both_mock_and_real_connect_info() {
        async fn handler(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> String {
            format!("{addr}")
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let app = Router::new()
                .route("/", get(handler))
                .layer(MockConnectInfo(SocketAddr::from(([0, 0, 0, 0], 1337))));

            crate::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });

        let client = reqwest::Client::new();

        let res = client.get(format!("http://{addr}")).send().await.unwrap();
        let body = res.text().await.unwrap();
        assert!(body.starts_with("127.0.0.1:"));
    }
}
