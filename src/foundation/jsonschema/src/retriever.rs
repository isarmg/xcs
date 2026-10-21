//! Logic for retrieving external resources.
use referencing::{Retrieve, Uri};
use serde_json::Value;

#[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
use crate::HttpOptions;

/// Error that can occur when creating an HTTP retriever.
#[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
#[derive(Debug)]
pub enum HttpRetrieverError {
    /// Failed to read certificate file.
    CertificateRead {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    /// Failed to parse certificate.
    CertificateParse {
        path: std::path::PathBuf,
        source: reqwest::Error,
    },
    /// Failed to build HTTP client.
    ClientBuild(reqwest::Error),
}

#[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
impl std::fmt::Display for HttpRetrieverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CertificateRead { path, source } => {
                write!(
                    f,
                    "Failed to read certificate file '{}': {source}",
                    path.display()
                )
            }
            Self::CertificateParse { path, source } => {
                write!(
                    f,
                    "Failed to parse certificate '{}': {source}",
                    path.display()
                )
            }
            Self::ClientBuild(e) => write!(f, "Failed to build HTTP client: {e}"),
        }
    }
}

#[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
impl std::error::Error for HttpRetrieverError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CertificateRead { source, .. } => Some(source),
            Self::CertificateParse { source, .. } | Self::ClientBuild(source) => Some(source),
        }
    }
}

/// Load a certificate from a PEM file.
#[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
fn load_certificate(path: &std::path::Path) -> Result<reqwest::Certificate, HttpRetrieverError> {
    let cert_data = std::fs::read(path).map_err(|e| HttpRetrieverError::CertificateRead {
        path: path.to_path_buf(),
        source: e,
    })?;
    reqwest::Certificate::from_pem(&cert_data).map_err(|e| HttpRetrieverError::CertificateParse {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Configure an HTTP client builder with the given options.
/// Works with both `reqwest::ClientBuilder` and `reqwest::blocking::ClientBuilder`.
#[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
macro_rules! configure_http_client {
    ($builder:expr, $options:expr) => {{
        let mut builder = $builder;
        if let Some(connect_timeout) = $options.connect_timeout {
            builder = builder.connect_timeout(connect_timeout);
        }
        if let Some(timeout) = $options.timeout {
            builder = builder.timeout(timeout);
        }
        if $options.danger_accept_invalid_certs {
            builder = builder.danger_accept_invalid_certs(true);
        }
        if let Some(ref cert_path) = &$options.root_certificate {
            builder = builder.add_root_certificate(load_certificate(cert_path)?);
        }
        builder
    }};
}

#[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
fn install_tls_provider() {
    // When both features are enabled (e.g. `--all-features`), keep aws-lc-rs
    // as the effective default provider.
    #[cfg(feature = "tls-aws-lc-rs")]
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    #[cfg(all(not(feature = "tls-aws-lc-rs"), feature = "tls-ring"))]
    let _ = rustls::crypto::ring::default_provider().install_default();
}

pub(crate) struct OfflineRetriever;

impl Retrieve for OfflineRetriever {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err(format!("Retrieval is disabled, cannot fetch {uri}").into())
    }
}

#[cfg(feature = "resolve-async")]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl referencing::AsyncRetrieve for OfflineRetriever {
    async fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err(format!("Retrieval is disabled, cannot fetch {uri}").into())
    }
}

pub(crate) struct DefaultRetriever;

/// HTTP-based schema retriever with configurable client options.
///
/// This retriever fetches external schemas over HTTP/HTTPS using a
/// configured [`reqwest`](https://docs.rs/reqwest) client.
///
/// # Example
///
/// ```rust,no_run
/// use std::time::Duration;
/// use jsonschema::{HttpOptions, HttpRetriever};
///
/// let http_options = HttpOptions::new()
///     .timeout(Duration::from_secs(30));
///
/// let retriever = HttpRetriever::new(&http_options)
///     .expect("Failed to create HTTP retriever");
/// ```
#[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
#[derive(Debug)]
pub struct HttpRetriever {
    client: reqwest::blocking::Client,
}

#[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
impl HttpRetriever {
    /// Create a new HTTP retriever with the given options.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The certificate file cannot be read
    /// - The certificate is not valid PEM
    /// - The HTTP client cannot be built
    pub fn new(options: &HttpOptions) -> Result<Self, HttpRetrieverError> {
        install_tls_provider();

        let builder = configure_http_client!(reqwest::blocking::Client::builder(), options);
        Ok(Self {
            client: builder.build().map_err(HttpRetrieverError::ClientBuild)?,
        })
    }
}

#[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
impl Retrieve for HttpRetriever {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match uri.scheme().as_str() {
            "http" | "https" => Ok(self.client.get(uri.as_str()).send()?.json()?),
            "file" => {
                #[cfg(feature = "resolve-file")]
                {
                    let path = uri.path().as_str();
                    let path = {
                        #[cfg(windows)]
                        {
                            let path = path.trim_start_matches('/').replace('/', "\\");
                            std::path::PathBuf::from(path)
                        }
                        #[cfg(not(windows))]
                        {
                            std::path::PathBuf::from(path)
                        }
                    };
                    let file = std::fs::File::open(path)?;
                    Ok(serde_json::from_reader(file)?)
                }
                #[cfg(not(feature = "resolve-file"))]
                {
                    Err("`resolve-file` feature or a custom resolver is required to resolve external schemas via files".into())
                }
            }
            scheme => Err(format!("Unknown scheme {scheme}").into()),
        }
    }
}

/// Async HTTP-based schema retriever with configurable client options.
///
/// This retriever fetches external schemas over HTTP/HTTPS using a
/// configured async [`reqwest`](https://docs.rs/reqwest) client.
#[cfg(all(
    feature = "resolve-http",
    feature = "resolve-async",
    not(target_arch = "wasm32")
))]
#[derive(Debug)]
pub struct AsyncHttpRetriever {
    client: reqwest::Client,
}

#[cfg(all(
    feature = "resolve-http",
    feature = "resolve-async",
    not(target_arch = "wasm32")
))]
impl AsyncHttpRetriever {
    /// Create a new async HTTP retriever with the given options.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The certificate file cannot be read
    /// - The certificate is not valid PEM
    /// - The HTTP client cannot be built
    pub fn new(options: &HttpOptions) -> Result<Self, HttpRetrieverError> {
        install_tls_provider();

        let builder = configure_http_client!(reqwest::Client::builder(), options);
        Ok(Self {
            client: builder.build().map_err(HttpRetrieverError::ClientBuild)?,
        })
    }
}

#[cfg(all(
    feature = "resolve-http",
    feature = "resolve-async",
    not(target_arch = "wasm32")
))]
#[async_trait::async_trait]
impl referencing::AsyncRetrieve for AsyncHttpRetriever {
    async fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match uri.scheme().as_str() {
            "http" | "https" => Ok(self.client.get(uri.as_str()).send().await?.json().await?),
            "file" => {
                #[cfg(feature = "resolve-file")]
                {
                    let path = uri.path().as_str().to_string();
                    let contents = tokio::task::spawn_blocking(
                        move || -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
                            let path = {
                                #[cfg(windows)]
                                {
                                    let path = path.trim_start_matches('/').replace('/', "\\");
                                    std::path::PathBuf::from(path)
                                }
                                #[cfg(not(windows))]
                                {
                                    std::path::PathBuf::from(path)
                                }
                            };
                            let file = std::fs::File::open(path)?;
                            Ok(serde_json::from_reader(file)?)
                        },
                    )
                    .await??;
                    Ok(contents)
                }
                #[cfg(not(feature = "resolve-file"))]
                {
                    Err("`resolve-file` feature or a custom resolver is required to resolve external schemas via files".into())
                }
            }
            scheme => Err(format!("Unknown scheme {scheme}").into()),
        }
    }
}

impl Retrieve for DefaultRetriever {
    #[allow(unused)]
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            Err("External references are not supported on wasm32-unknown-unknown".into())
        }
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        match uri.scheme().as_str() {
            "http" | "https" => {
                #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
                {
                    install_tls_provider();

                    Ok(reqwest::blocking::get(uri.as_str())?.json()?)
                }
                #[cfg(all(feature = "resolve-http", target_arch = "wasm32"))]
                {
                    Err("Synchronous HTTP retrieval is not supported on wasm32 targets. Use async_validator_for with the resolve-async feature instead".into())
                }
                #[cfg(not(feature = "resolve-http"))]
                {
                    Err("`resolve-http` feature or a custom resolver is required to resolve external schemas via HTTP".into())
                }
            }
            "file" => {
                #[cfg(feature = "resolve-file")]
                {
                    let path = uri.path().as_str();
                    let path = {
                        #[cfg(windows)]
                        {
                            // Remove the leading slash and replace forward slashes with backslashes
                            let path = path.trim_start_matches('/').replace('/', "\\");
                            std::path::PathBuf::from(path)
                        }
                        #[cfg(not(windows))]
                        {
                            std::path::PathBuf::from(path)
                        }
                    };
                    let file = std::fs::File::open(path)?;
                    Ok(serde_json::from_reader(file)?)
                }
                #[cfg(not(feature = "resolve-file"))]
                {
                    Err("`resolve-file` feature or a custom resolver is required to resolve external schemas via files".into())
                }
            }
            scheme => Err(format!("Unknown scheme {scheme}").into()),
        }
    }
}

#[cfg(feature = "resolve-async")]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl referencing::AsyncRetrieve for DefaultRetriever {
    #[allow(unused)]
    async fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            Err("External references are not supported on wasm32-unknown-unknown".into())
        }
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        match uri.scheme().as_str() {
            "http" | "https" => {
                #[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
                {
                    install_tls_provider();

                    Ok(reqwest::get(uri.as_str()).await?.json().await?)
                }
                #[cfg(all(
                    feature = "resolve-http",
                    target_arch = "wasm32",
                    any(target_os = "unknown", target_os = "none"),
                ))]
                {
                    Ok(reqwest::get(uri.as_str()).await?.json().await?)
                }
                #[cfg(all(
                    feature = "resolve-http",
                    target_arch = "wasm32",
                    not(any(target_os = "unknown", target_os = "none")),
                ))]
                {
                    Err("HTTP retrieval is not supported on this wasm32 target; provide a custom `AsyncRetrieve` to resolve external schemas via HTTP".into())
                }
                #[cfg(not(feature = "resolve-http"))]
                Err("`resolve-http` feature or a custom resolver is required to resolve external schemas via HTTP".into())
            }
            "file" => {
                #[cfg(feature = "resolve-file")]
                {
                    // File operations are blocking, so we use tokio's spawn_blocking
                    let path = uri.path().as_str().to_string();
                    let contents = tokio::task::spawn_blocking(
                        move || -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
                            let path = {
                                #[cfg(windows)]
                                {
                                    let path = path.trim_start_matches('/').replace('/', "\\");
                                    std::path::PathBuf::from(path)
                                }
                                #[cfg(not(windows))]
                                {
                                    std::path::PathBuf::from(path)
                                }
                            };
                            let file = std::fs::File::open(path)?;
                            Ok(serde_json::from_reader(file)?)
                        },
                    )
                    .await??;
                    Ok(contents)
                }
                #[cfg(not(feature = "resolve-file"))]
                {
                    Err("`resolve-file` feature or a custom resolver is required to resolve external schemas via files".into())
                }
            }
            scheme => Err(format!("Unknown scheme {scheme}").into()),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32"), feature = "resolve-file"))]
use percent_encoding::{AsciiSet, CONTROLS};

#[cfg(all(test, not(target_arch = "wasm32"), feature = "resolve-file"))]
const URI_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'#')
    .add(b'?')
    .add(b'{')
    .add(b'}')
    .add(b'/')
    .add(b'%');

#[cfg(all(
    test,
    not(target_arch = "wasm32"),
    not(target_os = "windows"),
    feature = "resolve-file"
))]
const UNIX_URI_SEGMENT: &AsciiSet = &URI_SEGMENT.add(b'\\');

#[cfg(all(test, not(target_arch = "wasm32"), feature = "resolve-file"))]
pub(crate) fn path_to_uri(path: &std::path::Path) -> String {
    use percent_encoding::percent_encode;

    let mut result = "file://".to_owned();

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::ffi::OsStrExt;

        for component in path.components().skip(1) {
            result.push('/');
            result.extend(percent_encode(
                component.as_os_str().as_bytes(),
                UNIX_URI_SEGMENT,
            ));
        }
    }
    #[cfg(target_os = "windows")]
    {
        use std::path::{Component, Prefix};
        let mut components = path.components();

        match components.next() {
            Some(Component::Prefix(ref p)) => match p.kind() {
                Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                    result.push('/');
                    result.push(letter as char);
                    result.push(':');
                }
                _ => panic!("Unexpected path"),
            },
            _ => panic!("Unexpected path"),
        }

        for component in components {
            if component == Component::RootDir {
                continue;
            }

            let component = component.as_os_str().to_str().expect("Unexpected path");

            result.push('/');
            result.extend(percent_encode(component.as_bytes(), URI_SEGMENT));
        }
    }
    result
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/retriever/tests_500.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/retriever/all_async_tests_503.rs"));
