//! Streaming transfer and atomic publication of verified immutable downloads.
use super::{Client, Error, response::decode, segment};
use futures_util::StreamExt;
use serde_json::Value;
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_util::io::ReaderStream;

impl Client {
    pub async fn upload_file(
        &self,
        workspace: &str,
        item: &str,
        path: &str,
        source: &Path,
    ) -> Result<Value, Error> {
        let mut options = tokio::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        // Opening a FIFO normally blocks before metadata can reject it. Keep
        // the open nonblocking, then validate the opened descriptor (not a
        // pre-open path check that could race a replacement). O_NONBLOCK has
        // no effect on regular-file reads and local symlinks remain supported.
        options.custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32);
        let mut file = options.open(source).await?;
        let metadata = file.metadata().await?;
        if !metadata.is_file() {
            return Err(Error::Response(
                "upload source must be a regular file".into(),
            ));
        }
        let mut hasher = blake3::Hasher::new();
        let mut buffer = vec![0_u8; 256 * 1024];
        loop {
            let count = file.read(&mut buffer).await?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        file.seek(std::io::SeekFrom::Start(0)).await?;
        let content_hash = hasher.finalize().to_hex().to_string();
        let mut url = self.url(&format!(
            "/v1/workspaces/{}/files/{}/content",
            segment(workspace),
            segment(item)
        ))?;
        url.query_pairs_mut().append_pair("path", path);
        let mut request = self
            .client
            .put(url)
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .header(reqwest::header::CONTENT_LENGTH, metadata.len())
            .header("x-zxs-logical-size", metadata.len().to_string())
            .header("x-zxs-upload-generation", "1")
            .header("x-zxs-content-hash", content_hash)
            .body(reqwest::Body::wrap_stream(ReaderStream::new(file)));
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        decode(request.send().await?).await
    }
    /// Publish only a complete verified file, without replacing any existing
    /// destination. The temporary path owns cleanup on errors and cancellation.
    pub async fn download_file(&self, version: &str, target: &Path) -> Result<(), Error> {
        if tokio::fs::try_exists(target).await? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "download destination already exists",
            )
            .into());
        }
        let response = self
            .builder(
                reqwest::Method::GET,
                &format!("/v1/versions/{}/content", segment(version)),
            )?
            .send()
            .await?;
        if !response.status().is_success() {
            decode(response).await?;
            return Err(Error::Response("unexpected success envelope".into()));
        }
        let expected = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| {
                value
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
            })
            .filter(|value| {
                value.len() == 64
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            .ok_or_else(|| Error::Response("missing or invalid immutable content hash".into()))?
            .to_owned();
        let expected_length = response
            .content_length()
            .ok_or_else(|| Error::Response("missing file length".into()))?;
        let parent = target
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        if target.file_name().is_none() {
            return Err(Error::Response("invalid download target".into()));
        }
        let (temporary, cleanup) = tempfile::Builder::new()
            .prefix(".zxs-download-")
            .tempfile_in(parent)?
            .into_parts();
        let mut file = tokio::fs::File::from_std(temporary);
        let mut length = 0u64;
        let mut hash = blake3::Hasher::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            length = length
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| Error::Response("length overflow".into()))?;
            if length > expected_length {
                return Err(Error::Response("content exceeded advertised length".into()));
            }
            hash.update(&chunk);
            file.write_all(&chunk).await?;
        }
        if length != expected_length || hash.finalize().to_hex().as_str() != expected {
            return Err(Error::Response(
                "CHECKSUM_MISMATCH: download did not match the immutable version hash".into(),
            ));
        }
        file.sync_all().await?;
        drop(file);
        // No empty reservation file is exposed. Publication is an indivisible
        // no-replace operation, so a concurrent creator always keeps its file.
        cleanup
            .persist_noclobber(target)
            .map_err(|error| error.error)?;
        #[cfg(unix)]
        {
            let parent = parent.to_owned();
            tokio::task::spawn_blocking(move || std::fs::File::open(parent)?.sync_all())
                .await
                .map_err(|error| Error::Response(error.to_string()))??;
        }
        Ok(())
    }
}
