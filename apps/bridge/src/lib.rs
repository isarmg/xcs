//! Native transfer boundary: session tokens stay in Rust and file bytes never
//! travel through WebView IPC. Only fixed API routes and structured results do.
use serde::Serialize;
use serde_json::{Value, json};
use std::path::Path;
use tokio::sync::RwLock;
use xcs_client::{Client, Error};
mod cache;
mod connection;
pub use cache::{StagedImport, cache_target, prepare_cache_target, stage_import};

#[derive(Clone, Debug, Serialize)]
pub struct BridgeError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub status: u16,
}
impl From<Error> for BridgeError {
    fn from(error: Error) -> Self {
        match error {
            Error::Api {
                status,
                code,
                message,
                retryable,
            } => Self {
                status,
                code,
                message,
                retryable,
            },
            Error::Address(message) => Self {
                code: "INVALID_SERVER".into(),
                message,
                retryable: false,
                status: 0,
            },
            Error::Io(error) => Self {
                code: if error.kind() == std::io::ErrorKind::AlreadyExists {
                    "DESTINATION_EXISTS"
                } else {
                    "LOCAL_IO_ERROR"
                }
                .into(),
                message: error.to_string(),
                retryable: false,
                status: 0,
            },
            Error::Transport(_) => Self {
                code: "TRANSPORT_ERROR".into(),
                message: "连接中断；提交结果请查询原操作 ID".into(),
                retryable: true,
                status: 0,
            },
            Error::Response(message) => Self {
                code: "INVALID_RESPONSE".into(),
                message,
                retryable: false,
                status: 0,
            },
        }
    }
}
fn disconnected() -> BridgeError {
    BridgeError {
        code: "NOT_CONNECTED".into(),
        message: "请先登录服务器".into(),
        retryable: false,
        status: 401,
    }
}
fn session_changed() -> BridgeError {
    BridgeError {
        code: "SESSION_CHANGED".into(),
        message: "The connection changed while the operation was running".into(),
        retryable: false,
        status: 0,
    }
}
fn public_session(session: &xcs_client::Session) -> Value {
    json!({"principal":session.principal,"admin":session.admin,"expires_at":session.expires_at.to_string()})
}

#[derive(Default)]
pub struct Bridge {
    connection: RwLock<connection::Connection>,
}
impl Bridge {
    pub async fn connect(
        &self,
        endpoint: &str,
        username: &str,
        password: &str,
    ) -> Result<Value, BridgeError> {
        let mut client = Client::connect(endpoint)?;
        let attempt = self.connection.write().await.begin();
        let session = client.login(username, password).await?;
        let result = public_session(&session);
        // The session is committed only after successful authentication. Neither
        // password nor token is returned to the WebView or persisted to disk.
        let installed = self
            .connection
            .write()
            .await
            .install(attempt, client.clone());
        if !installed {
            let _ = client.logout().await;
            return Err(session_changed());
        }
        Ok(result)
    }
    async fn snapshot(&self) -> Result<(Client, u64), BridgeError> {
        self.connection
            .read()
            .await
            .snapshot()
            .ok_or_else(disconnected)
    }
    async fn ensure_current(&self, revision: u64) -> Result<(), BridgeError> {
        if self.connection.read().await.current(revision) {
            Ok(())
        } else {
            Err(session_changed())
        }
    }
    pub async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, BridgeError> {
        if !matches!(method, "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD") {
            return Err(BridgeError {
                code: "INVALID_METHOD".into(),
                message: "不支持的请求方法".into(),
                retryable: false,
                status: 0,
            });
        }
        let route = path.split('?').next().unwrap_or(path);
        // Authentication must go through connect so tokens remain in Rust and
        // the installed session participates in connection generation checks.
        if route == "/v1/auth/login" {
            return Err(BridgeError {
                code: "NATIVE_LOGIN_REQUIRED".into(),
                message: "Use the native connection command to sign in".into(),
                retryable: false,
                status: 0,
            });
        }
        let (client, revision) = self.snapshot().await?;
        let mut result = client.request(method, path, body).await;
        if route == "/v1/auth/me"
            && let Ok(Value::Object(value)) = &mut result
        {
            // Native requests use the Rust-held bearer, never cookie CSRF.
            value.remove("token");
            value.remove("csrf_token");
        }
        let mut connection = self.connection.write().await;
        if !connection.current(revision) {
            return Err(session_changed());
        }
        // A failed logout is unconfirmed: keep the credential so the locked
        // UI can retry revocation. An explicit 401 confirms it is already invalid.
        if (method == "POST"
            && route == "/v1/auth/logout"
            && (result.is_ok() || matches!(&result, Err(Error::Api { status: 401, .. }))))
            || (method == "POST"
                && route == "/v1/auth/password"
                && result.as_ref().is_ok_and(|value| {
                    value["changed"] == true && value["reauthentication_required"] == true
                }))
        {
            connection.clear_if(revision);
        }
        result.map_err(Into::into)
    }
    pub async fn upload(
        &self,
        workspace: &str,
        item: &str,
        path: &str,
        source: &Path,
    ) -> Result<Value, BridgeError> {
        let (client, revision) = self.snapshot().await?;
        let result = client.upload_file(workspace, item, path, source).await;
        self.ensure_current(revision).await?;
        result.map_err(Into::into)
    }
    pub async fn download(&self, version: &str, target: &Path) -> Result<Value, BridgeError> {
        let (client, revision) = self.snapshot().await?;
        let result = client.download_file(version, target).await;
        self.ensure_current(revision).await?;
        result?;
        Ok(json!({"state":"DOWNLOADED","target":target,"verified":true}))
    }
    pub async fn commit_workspace(
        &self,
        workspace: &str,
        operation_id: &str,
        message: &str,
        expected_head: &str,
    ) -> Result<Value, BridgeError> {
        let (client, revision) = self.snapshot().await?;
        let result = client
            .commit_workspace(
                workspace,
                operation_id,
                message,
                if expected_head.is_empty() {
                    None
                } else {
                    Some(expected_head)
                },
            )
            .await;
        self.ensure_current(revision).await?;
        result.map_err(Into::into)
    }

    /// Reusing an opened cache entry always verifies it against the immutable
    /// server version. External editor saves therefore cannot masquerade as the
    /// original version; they must go through a workspace upload and commit.
    pub async fn verify_cached(&self, version: &str, target: &Path) -> Result<(), BridgeError> {
        let (client, revision) = self.snapshot().await?;
        let result = async {
            let metadata = client
                .request(
                    "GET",
                    &format!("/v1/versions/{}", xcs_client::segment(version)),
                    None,
                )
                .await?;
            let expected = metadata
                .get("content_hash")
                .or_else(|| metadata.get("version").and_then(|v| v.get("content_hash")))
                .and_then(Value::as_str)
                .ok_or_else(|| BridgeError {
                    code: "INVALID_VERSION".into(),
                    message: "版本缺少完整性摘要".into(),
                    retryable: false,
                    status: 0,
                })?
                .to_owned();
            let path = target.to_owned();
            let actual = tokio::task::spawn_blocking(move || -> Result<String, std::io::Error> {
                // Reject directories, provider special files, and redirected cache
                // entries before opening: a FIFO could otherwise block forever.
                let metadata = std::fs::symlink_metadata(&path)?;
                if !metadata.is_file() || metadata.file_type().is_symlink() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "cache entry must be a regular file",
                    ));
                }
                let mut file = std::fs::File::open(path)?;
                let mut hasher = blake3::Hasher::new();
                std::io::copy(&mut file, &mut hasher)?;
                Ok(hasher.finalize().to_hex().to_string())
            })
            .await
            .map_err(|_| BridgeError {
                code: "CACHE_VERIFY".into(),
                message: "缓存校验任务中断".into(),
                retryable: false,
                status: 0,
            })?
            .map_err(Error::Io)?;
            if actual != expected {
                return Err(BridgeError {
                    code: "CHECKSUM_MISMATCH".into(),
                    message: "缓存内容已变化；请通过工作区保存新版本".into(),
                    retryable: false,
                    status: 0,
                });
            }
            Ok(())
        }
        .await;
        self.ensure_current(revision).await?;
        result
    }
    pub async fn disconnect(&self) {
        self.connection.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ipc_session_does_not_expose_bearer_or_csrf_credentials() {
        let session = xcs_client::Session {
            token: "private-bearer-token".into(),
            csrf_token: "private-csrf-token".into(),
            principal: "alice".into(),
            admin: false,
            expires_at: 9_007_199_254_740_993,
        };
        let public = public_session(&session);
        let encoded = public.to_string();
        assert!(!encoded.contains(&session.token));
        assert!(!encoded.contains(&session.csrf_token));
        assert_eq!(public["expires_at"], "9007199254740993");
        assert_eq!(public.as_object().unwrap().len(), 3);
    }
}

#[cfg(test)]
mod connection_tests;
