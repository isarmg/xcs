//! One-shot native file selections. IPC carries capabilities, never filesystem paths.
use crate::{Bridge, BridgeError, StagedImport, session_changed};
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashMap, path::Path};
use zxs_client::Client;

pub struct PickedSource {
    file: tokio::fs::File,
    name: String,
    size: u64,
    // The temporary import is deleted only after its selected file is dropped.
    _import: Option<StagedImport>,
}
impl PickedSource {
    pub async fn from_path(path: &Path, name: String) -> Result<Self, BridgeError> {
        zxs_naming::validate_name(&name).map_err(|error| BridgeError {
            code: "INVALID_FILENAME".into(),
            message: error.message,
            retryable: false,
            status: 0,
        })?;
        let file = Client::open_upload_source(path).await?;
        let size = file.metadata().await.map_err(zxs_client::Error::Io)?.len();
        Ok(Self {
            file,
            name,
            size,
            _import: None,
        })
    }
    pub async fn from_import(import: StagedImport, name: String) -> Result<Self, BridgeError> {
        let mut source = Self::from_path(import.path(), name).await?;
        source._import = Some(import);
        Ok(source)
    }
}
#[derive(Serialize)]
pub struct Selection {
    pub id: String,
    pub name: String,
    // File lengths stay exact across the JavaScript boundary.
    pub size: String,
}
#[derive(Default)]
pub(crate) struct Sources(HashMap<String, (u64, PickedSource)>);
impl Sources {
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
}
fn invalid_source() -> BridgeError {
    BridgeError {
        code: "INVALID_UPLOAD_SOURCE".into(),
        message: "Select the file again before uploading".into(),
        retryable: false,
        status: 0,
    }
}
impl Bridge {
    pub async fn register_sources(
        &self,
        revision: u64,
        sources: Vec<PickedSource>,
    ) -> Result<Vec<Selection>, BridgeError> {
        // Hold the session read lock through insertion, so logout cannot clear then
        // race a stale picker registration. No source lock is held across await.
        let connection = self.connection.read().await;
        if !connection.work_current(revision) {
            return Err(session_changed());
        }
        let mut registry = self.sources.lock().unwrap();
        if sources.len() > 128 || registry.0.len().saturating_add(sources.len()) > 128 {
            return Err(invalid_source());
        }
        let mut selected = Vec::with_capacity(sources.len());
        for source in sources {
            let id = zxs_contract::new_id("source");
            selected.push(Selection {
                id: id.clone(),
                name: source.name.clone(),
                size: source.size.to_string(),
            });
            registry.0.insert(id, (revision, source));
        }
        Ok(selected)
    }
    pub(crate) async fn send_prepared_upload(
        &self,
        revision: u64,
        prepared: zxs_client::PreparedUpload,
    ) -> Result<Value, BridgeError> {
        self.ensure_current(revision).await?;
        let result = prepared.send().await;
        self.ensure_current(revision).await?;
        result.map_err(Into::into)
    }
    pub fn release_sources(&self, ids: &[String]) {
        let mut registry = self.sources.lock().unwrap();
        for id in ids {
            registry.0.remove(id);
        }
    }
    /// Consume a selection once. A failed transfer requires reselection; original
    /// user files are never deleted, and failed workspaces/receipts remain intact.
    pub async fn upload_selected(
        &self,
        workspace: &str,
        item: &str,
        path: &str,
        source_id: &str,
    ) -> Result<Value, BridgeError> {
        let (client, revision) = self.snapshot().await?;
        let (selected_revision, source) = self
            .sources
            .lock()
            .unwrap()
            .0
            .remove(source_id)
            .ok_or_else(invalid_source)?;
        if selected_revision != revision {
            return Err(session_changed());
        }
        let PickedSource { file, _import, .. } = source;
        let prepared = client
            .prepare_file_upload(workspace, item, path, file)
            .await;
        self.ensure_current(revision).await?;
        self.send_prepared_upload(revision, prepared?).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn connected() -> Bridge {
        let bridge = Bridge::default();
        let mut connection = bridge.connection.write().await;
        let attempt = connection.begin();
        connection.install(attempt, Client::connect("http://127.0.0.1:1").unwrap());
        drop(connection);
        bridge
    }
    #[tokio::test]
    async fn selections_expose_only_capabilities_and_are_released_with_owned_imports() {
        let bridge = connected().await;
        let root = tempfile::tempdir().unwrap();
        let imports = root.path().join("imports");
        let source = PickedSource::from_import(
            crate::stage_import(&imports, &mut Cursor::new(b"private")).unwrap(),
            "file.txt".into(),
        )
        .await
        .unwrap();
        let revision = bridge.work_revision().await.unwrap();
        let selected = bridge
            .register_sources(revision, vec![source])
            .await
            .unwrap();
        let value = serde_json::to_value(&selected).unwrap();
        assert_eq!(value[0].as_object().unwrap().len(), 3);
        assert_eq!(value[0]["size"], "7");
        assert!(!value.to_string().contains(root.path().to_str().unwrap()));
        assert_eq!(std::fs::read_dir(&imports).unwrap().count(), 1);
        bridge.release_sources(&[selected[0].id.clone()]);
        bridge.release_sources(&[selected[0].id.clone()]);
        assert_eq!(std::fs::read_dir(&imports).unwrap().count(), 0);
    }
    #[tokio::test]
    async fn stale_picker_and_session_replacement_drop_private_imports() {
        let bridge = connected().await;
        let root = tempfile::tempdir().unwrap();
        let imports = root.path().join("imports");
        let revision = bridge.work_revision().await.unwrap();
        let source = PickedSource::from_import(
            crate::stage_import(&imports, &mut Cursor::new(b"private")).unwrap(),
            "file.txt".into(),
        )
        .await
        .unwrap();
        bridge.disconnect().await;
        assert!(
            bridge
                .register_sources(revision, vec![source])
                .await
                .is_err()
        );
        assert_eq!(std::fs::read_dir(&imports).unwrap().count(), 0);
    }
    #[tokio::test]
    async fn arbitrary_paths_and_replayed_handles_cannot_upload_and_failure_preserves_user_file() {
        let bridge = connected().await;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("file.txt");
        std::fs::write(&path, b"original").unwrap();
        assert_eq!(
            bridge
                .upload_selected("w", "i", "file.txt", path.to_str().unwrap())
                .await
                .unwrap_err()
                .code,
            "INVALID_UPLOAD_SOURCE"
        );
        let source = PickedSource::from_path(&path, "file.txt".into())
            .await
            .unwrap();
        let selected = bridge
            .register_sources(bridge.work_revision().await.unwrap(), vec![source])
            .await
            .unwrap();
        assert!(
            bridge
                .upload_selected("w", "i", "file.txt", &selected[0].id)
                .await
                .is_err()
        );
        assert_eq!(
            bridge
                .upload_selected("w", "i", "file.txt", &selected[0].id)
                .await
                .unwrap_err()
                .code,
            "INVALID_UPLOAD_SOURCE"
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
    }
    #[tokio::test]
    async fn prepared_upload_cannot_send_after_logout_begins() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client =
            Client::connect(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let bridge = Bridge::default();
        {
            let mut connection = bridge.connection.write().await;
            let attempt = connection.begin();
            connection.install(attempt, client.clone());
        }
        let revision = bridge.work_revision().await.unwrap();
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), b"selected file").unwrap();
        let file = Client::open_upload_source(source.path()).await.unwrap();
        let prepared = client
            .prepare_file_upload("workspace", "item", "file.txt", file)
            .await
            .unwrap();
        let server = tokio::spawn(async move {
            let Ok(Ok((mut socket, _))) =
                tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
                    .await
            else {
                return false;
            };
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .await
                .unwrap();
            true
        });
        // Deterministically hold the completed hash at the exact dispatch boundary.
        bridge.connection.write().await.begin_logout();
        assert_eq!(
            bridge
                .send_prepared_upload(revision, prepared)
                .await
                .unwrap_err()
                .code,
            "SESSION_CHANGED"
        );
        assert!(
            !server.await.unwrap(),
            "upload bytes were dispatched after logout began"
        );
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn selected_descriptor_is_streamed_even_if_the_original_path_is_replaced() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                headers.push(socket.read_u8().await.unwrap());
            }
            let headers = String::from_utf8(headers).unwrap();
            let length: usize = headers
                .lines()
                .find_map(|line| {
                    line.to_lowercase()
                        .strip_prefix("content-length: ")
                        .map(str::to_owned)
                })
                .unwrap()
                .parse()
                .unwrap();
            let mut body = vec![0; length];
            socket.read_exact(&mut body).await.unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .await
                .unwrap();
            body
        });
        let bridge = Bridge::default();
        let mut connection = bridge.connection.write().await;
        let attempt = connection.begin();
        connection.install(attempt, Client::connect(&endpoint).unwrap());
        drop(connection);
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("chosen.txt");
        std::fs::write(&path, b"chosen bytes").unwrap();
        let source = PickedSource::from_path(&path, "chosen.txt".into())
            .await
            .unwrap();
        let selected = bridge
            .register_sources(bridge.work_revision().await.unwrap(), vec![source])
            .await
            .unwrap();
        std::fs::rename(&path, root.path().join("moved.txt")).unwrap();
        std::fs::write(&path, b"unselected replacement").unwrap();
        bridge
            .upload_selected("workspace", "item", "chosen.txt", &selected[0].id)
            .await
            .unwrap();
        assert_eq!(server.await.unwrap(), b"chosen bytes");
        assert_eq!(std::fs::read(&path).unwrap(), b"unselected replacement");
        assert_eq!(
            bridge
                .upload_selected("workspace", "item", "chosen.txt", &selected[0].id)
                .await
                .unwrap_err()
                .code,
            "INVALID_UPLOAD_SOURCE"
        );
    }
}
