#[cfg(all(feature = "tokio", test))]
mod tests {
    use crate::adapters::{
        rt::{TokioExecutor, TokioIo},
        server::conn::auto,
    };
    use http::{Request, Response};
    use http_body::Body;
    use http_body::{BodyExt, Empty, Full};
    use http_transport::{body, body::Bytes, client, service::service_fn};
    use std::{convert::Infallible, error::Error as StdError, net::SocketAddr, time::Duration};
    use tokio::{
        net::{TcpListener, TcpStream},
        pin,
    };

    const BODY: &[u8] = b"Hello, world!";

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hyper-util/functions/src/server/conn/auto/mod/configuration_1153.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hyper-util/functions/src/server/conn/auto/mod/title_case_headers_configuration_1171.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hyper-util/functions/src/server/conn/auto/mod/preserve_header_case_configuration_1183.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hyper-util/functions/src/server/conn/auto/mod/http1_1195.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hyper-util/functions/src/server/conn/auto/mod/http2_1211.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hyper-util/functions/src/server/conn/auto/mod/http2_only_1227.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hyper-util/functions/src/server/conn/auto/mod/http2_only_fail_if_client_is_http1_1243.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hyper-util/functions/src/server/conn/auto/mod/http1_only_1255.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hyper-util/functions/src/server/conn/auto/mod/http1_only_fail_if_client_is_http2_1271.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hyper-util/functions/src/server/conn/auto/mod/graceful_shutdown_1283.rs"));

    async fn connect_h1<B>(addr: SocketAddr) -> client::conn::http1::SendRequest<B>
    where
        B: Body + Send + 'static,
        B::Data: Send,
        B::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let stream = TokioIo::new(TcpStream::connect(addr).await.unwrap());
        let (sender, connection) = client::conn::http1::handshake(stream).await.unwrap();

        tokio::spawn(connection);

        sender
    }

    async fn connect_h2<B>(addr: SocketAddr) -> client::conn::http2::SendRequest<B>
    where
        B: Body + Unpin + Send + 'static,
        B::Data: Send,
        B::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let stream = TokioIo::new(TcpStream::connect(addr).await.unwrap());
        let (sender, connection) = client::conn::http2::Builder::new(TokioExecutor::new())
            .handshake(stream)
            .await
            .unwrap();

        tokio::spawn(connection);

        sender
    }

    async fn start_server(h1_only: bool, h2_only: bool) -> SocketAddr {
        let addr: SocketAddr = ([127, 0, 0, 1], 0).into();
        let listener = TcpListener::bind(addr).await.unwrap();

        let local_addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let stream = TokioIo::new(stream);
                tokio::task::spawn(async move {
                    let mut builder = auto::Builder::new(TokioExecutor::new());
                    if h1_only {
                        builder = builder.http1_only();
                        builder.serve_connection(stream, service_fn(hello)).await
                    } else if h2_only {
                        builder = builder.http2_only();
                        builder.serve_connection(stream, service_fn(hello)).await
                    } else {
                        builder
                            .http2()
                            .max_header_list_size(4096)
                            .serve_connection_with_upgrades(stream, service_fn(hello))
                            .await
                    }
                    .unwrap();
                });
            }
        });

        local_addr
    }

    async fn hello(_req: Request<body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
        Ok(Response::new(Full::new(Bytes::from(BODY))))
    }
}
