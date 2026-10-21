#[cfg(all(test,any (feature = "ring" , feature = "aws-lc-rs"),any(feature = "webpki-roots")))]
mod tests {
    use std::future::poll_fn;

    use http::Uri;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpStream;
    use tower_service::Service;

    use super::*;
    use crate::{ConfigBuilderExt, HttpsConnectorBuilder, MaybeHttpsStream};

    #[tokio::test]
    async fn connects_https() {
        connect(Allow::Any, Scheme::Https)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn connects_http() {
        connect(Allow::Any, Scheme::Http)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn connects_https_only() {
        connect(Allow::Https, Scheme::Https)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn enforces_https_only() {
        let message = connect(Allow::Https, Scheme::Http)
            .await
            .unwrap_err()
            .to_string();

        assert_eq!(message, "unsupported scheme http");
    }

    async fn connect(
        allow: Allow,
        scheme: Scheme,
    ) -> Result<MaybeHttpsStream<TokioIo<TcpStream>>, BoxError> {
        let config_builder = rustls::ClientConfig::builder();
        cfg_if::cfg_if! {
            if #[cfg(feature = "rustls-platform-verifier")] {
                let config_builder = config_builder.try_with_platform_verifier()?;
            } else if #[cfg(feature = "rustls-native-certs")] {
                let config_builder = config_builder.with_native_roots().unwrap();
            } else if #[cfg(feature = "webpki-roots")] {
                let config_builder = config_builder.with_webpki_roots();
            }
        }
        let config = config_builder.with_no_client_auth();

        let builder = HttpsConnectorBuilder::new().with_tls_config(config);
        let mut service = match allow {
            Allow::Https => builder.https_only(),
            Allow::Any => builder.https_or_http(),
        }
        .enable_http1()
        .build();

        poll_fn(|cx| service.poll_ready(cx)).await?;
        service
            .call(Uri::from_static(match scheme {
                Scheme::Https => "https://google.com",
                Scheme::Http => "http://google.com",
            }))
            .await
    }

    enum Allow {
        Https,
        Any,
    }

    enum Scheme {
        Https,
        Http,
    }
}
