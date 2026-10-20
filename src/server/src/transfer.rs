//! HTTP upload staging and immutable content streaming.
//!
//! The backend owns storage authorization and publication. This boundary owns
//! request limits, byte ranges, staging progress and the read-lease lifetime.
use crate::{ApiError, AppState, Content, auth::Identity, lock, uploads};
use axum::{
    Json,
    body::Body,
    extract::{Path as RoutePath, Query, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{Seek, SeekFrom},
    path::Path,
    sync::Arc,
};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

pub(crate) async fn upload_status(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<Identity>,
    RoutePath((workspace, item)): RoutePath<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let backend = state.backend.clone();
    let owner = identity.clone();
    let ws = workspace.clone();
    let item_id = item.clone();
    tokio::task::spawn_blocking(move || {
        backend.call(
            crate::BackendAction::WorkspaceAuthorize,
            &owner,
            json!({"workspace_id":ws,"item_id":item_id}),
        )
    })
    .await
    .map_err(ApiError::invalid)??;
    let backend = state.backend.clone();
    let owner = identity.clone();
    let ws = workspace.clone();
    let item_id = item.clone();
    let published = match tokio::task::spawn_blocking(move || {
        backend.call(
            crate::BackendAction::UploadStatus,
            &owner,
            json!({"workspace_id":ws,"item_id":item_id}),
        )
    })
    .await
    .map_err(ApiError::invalid)?
    {
        Ok(status) => Ok(status),
        Err(error) if error.status == 404 => Err(error),
        Err(error) => return Err(error),
    };
    let root = state.config.spool_dir.clone();
    let ws = workspace.clone();
    let item_id = item.clone();
    if let Some(record) = tokio::task::spawn_blocking(move || uploads::load(&root, &ws, &item_id))
        .await
        .map_err(ApiError::invalid)??
    {
        if record.principal != identity.principal && !identity.admin {
            return Err(ApiError::forbidden());
        }
        // A replacement can be durably receiving while the engine still owns
        // the previous completed generation. Do not report that old content as
        // the current upload. Equal/newer engine generations remain authoritative,
        // including their sealed manifest and completed publication state.
        let published_generation = published
            .as_ref()
            .ok()
            .and_then(|status| status.get("upload_generation"))
            .and_then(Value::as_str)
            .and_then(|generation| storage_contract::parse_u64_decimal(generation).ok());
        if published_generation.is_none_or(|generation| record.generation > generation) {
            return Ok(Json(uploads::status(&record)));
        }
    }
    published.map(Json)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UploadQuery {
    path: String,
}

/// Shared with every blocking metadata/publication task. Request cancellation
/// must not admit a retry until all tasks that can replace its record finish.
struct UploadAdmission {
    _permit: tokio::sync::OwnedSemaphorePermit,
    _claim: uploads::Claim,
}

async fn persist_upload(
    root: &Path,
    file: &tokio::fs::File,
    record: &uploads::UploadRecord,
    admission: &Arc<UploadAdmission>,
    staged: &Arc<uploads::StagedFile>,
) -> Result<(), ApiError> {
    file.sync_all().await.map_err(ApiError::io)?;
    let root = root.to_owned();
    let record = record.clone();
    let admission = admission.clone();
    let staged = staged.clone();
    tokio::task::spawn_blocking(move || {
        let _admission = admission;
        uploads::persist_staged(&root, &record, &staged)
    })
    .await
    .map_err(ApiError::invalid)?
}
pub(crate) async fn upload(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<Identity>,
    RoutePath((workspace, item)): RoutePath<(String, String)>,
    Query(query): Query<UploadQuery>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let _permit = state
        .upload_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::limit("upload capacity reached"))?;
    let key = uploads::key(&workspace, &item);
    {
        let mut claims = lock(&state.upload_claims)?;
        if !claims.insert(key.clone()) {
            return Err(ApiError::conflict(
                "FILE_WRITE_BUSY",
                "another upload owns this item",
            ));
        }
    }
    let admission = Arc::new(UploadAdmission {
        _permit,
        _claim: uploads::Claim {
            claims: state.upload_claims.clone(),
            key,
        },
    });
    name_policy::validate_path(&query.path).map_err(ApiError::invalid)?;
    let headers = request.headers();
    let parse_header = |name: &'static str| -> Result<Option<u64>, ApiError> {
        if headers.get_all(name).iter().count() > 1 {
            return Err(ApiError::invalid("duplicate upload control header"));
        }
        headers
            .get(name)
            .map(|value| {
                value.to_str().map_err(ApiError::invalid).and_then(|value| {
                    storage_contract::parse_u64_decimal(value).map_err(ApiError::invalid)
                })
            })
            .transpose()
    };
    let expected_length = parse_header("x-zxs-logical-size")?
        .or(parse_header("content-length")?)
        .ok_or_else(|| ApiError::invalid("Content-Length or X-ZXS-Logical-Size is required"))?;
    if expected_length > state.config.max_upload_bytes {
        return Err(ApiError::new(
            413,
            "RESOURCE_LIMIT_EXCEEDED",
            "file exceeds upload limit",
        ));
    }
    let generation = parse_header("x-zxs-upload-generation")?.unwrap_or(1);
    if generation == 0 {
        return Err(ApiError::invalid("upload generation must be positive"));
    }
    let expected_hash = headers
        .get("x-zxs-content-hash")
        .map(|value| value.to_str().map(str::to_owned).map_err(ApiError::invalid))
        .transpose()?;
    if expected_hash.is_none() {
        return Err(ApiError::invalid("X-ZXS-Content-Hash is required"));
    }
    if headers.get_all("x-zxs-content-hash").iter().count() != 1 {
        return Err(ApiError::invalid("duplicate content hash header"));
    }
    if expected_hash.as_ref().is_some_and(|hash| {
        hash.len() != 64
            || !hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }) {
        return Err(ApiError::invalid("invalid BLAKE3 upload hash"));
    }
    let backend = state.backend.clone();
    let owner = identity.clone();
    let ws = workspace.clone();
    let item_id = item.clone();
    tokio::task::spawn_blocking(move || {
        backend.call(
            crate::BackendAction::UploadAuthorize,
            &owner,
            json!({"workspace_id":ws,"item_id":item_id,"logical_size":expected_length.to_string()}),
        )
    })
    .await
    .map_err(ApiError::invalid)??;
    let spool = state.config.spool_dir.clone();
    let root = spool.clone();
    let ws = workspace.clone();
    let item_id = item.clone();
    let previous = tokio::task::spawn_blocking(move || uploads::load(&root, &ws, &item_id))
        .await
        .map_err(ApiError::invalid)??;
    let mut prefix = None;
    if let Some(previous) = &previous {
        if previous.principal != identity.principal || previous.path != query.path {
            return Err(ApiError::forbidden());
        }
        if generation < previous.generation {
            return Err(ApiError::conflict(
                "UPLOAD_GENERATION_CONFLICT",
                "an upload from a newer generation already exists",
            ));
        }
        if generation == previous.generation {
            if expected_length != previous.logical_size
                || expected_hash
                    .as_ref()
                    .zip(previous.content_hash.as_ref())
                    .is_some_and(|(a, b)| a != b)
            {
                return Err(ApiError::conflict(
                    "UPLOAD_GENERATION_CONFLICT",
                    "length or content hash changed; explicitly start a newer upload generation",
                ));
            }
            let root = spool.clone();
            let previous = previous.clone();
            let ownership = admission.clone();
            prefix = tokio::task::spawn_blocking(move || {
                let _ownership = ownership;
                uploads::ExistingPrefix::open(&root, &previous)
            })
            .await
            .map_err(ApiError::invalid)??;
        }
    }
    let data_file = format!(
        "upload-{}.data",
        authentication::random_token().map_err(ApiError::invalid)?
    );
    let path = spool.join(&data_file);
    let create_path = path.clone();
    let superseded = previous
        .as_ref()
        .filter(|previous| previous.generation == generation)
        .map(|previous| spool.join(&previous.data_file));
    let create_admission = admission.clone();
    let (file, staged_file) = tokio::task::spawn_blocking(move || {
        let _admission = create_admission;
        uploads::StagedFile::create(create_path, superseded)
    })
    .await
    .map_err(ApiError::invalid)??;
    let mut file = tokio::fs::File::from_std(file);
    let mut record = uploads::UploadRecord {
        workspace_id: workspace.clone(),
        item_id: item.clone(),
        principal: identity.principal.clone(),
        path: query.path.clone(),
        generation,
        logical_size: expected_length,
        content_hash: expected_hash.clone(),
        persisted_bytes: 0,
        persisted_hash: blake3::hash(b"").to_hex().to_string(),
        state: "incomplete".into(),
        data_file,
    };
    if let Some(previous) = previous
        .as_ref()
        .filter(|previous| generation > previous.generation)
    {
        let root = spool.clone();
        let previous = previous.clone();
        let ownership = (admission.clone(), staged_file.clone());
        tokio::task::spawn_blocking(move || {
            let _ownership = ownership;
            uploads::retain_generation(&root, &previous)
        })
        .await
        .map_err(ApiError::invalid)??;
    }
    if previous
        .as_ref()
        .is_none_or(|previous| generation > previous.generation)
    {
        persist_upload(&spool, &file, &record, &admission, &staged_file).await?;
    }
    let mut stream = request.into_body().into_data_stream();
    let mut length = 0u64;
    let mut last_persisted = 0u64;
    let mut hash = blake3::Hasher::new();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => {
                if prefix.as_ref().is_none_or(|prefix| prefix.complete()) {
                    record.persisted_bytes = length;
                    record.persisted_hash = hash.clone().finalize().to_hex().to_string();
                    persist_upload(&spool, &file, &record, &admission, &staged_file).await?;
                }
                return Err(ApiError::invalid(format!(
                    "upload interrupted; durable work data is retained: {error}"
                )));
            }
        };
        length = length
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| ApiError::invalid("upload length overflow"))?;
        if length > expected_length {
            return Err(ApiError::new(
                413,
                "UPLOAD_RANGE_CONFLICT",
                "received more bytes than declared; original upload is retained",
            ));
        }
        if let Some(mut existing) = prefix.take() {
            let bytes = chunk.clone();
            let ownership = (admission.clone(), staged_file.clone());
            prefix = tokio::task::spawn_blocking(move || {
                let _ownership = ownership;
                existing.check(&bytes)?;
                // Only retries pay this blocking boundary, and only until the
                // durable prefix is consumed. Fresh uploads create no read jobs.
                Ok::<_, ApiError>((!existing.complete()).then_some(existing))
            })
            .await
            .map_err(ApiError::invalid)??;
        }
        hash.update(&chunk);
        file.write_all(&chunk).await.map_err(ApiError::io)?;
        if length.saturating_sub(last_persisted) >= 1024 * 1024
            && prefix.as_ref().is_none_or(|prefix| prefix.complete())
        {
            record.persisted_bytes = length;
            record.persisted_hash = hash.clone().finalize().to_hex().to_string();
            persist_upload(&spool, &file, &record, &admission, &staged_file).await?;
            last_persisted = length;
        }
    }
    if prefix.as_ref().is_some_and(|prefix| !prefix.complete()) {
        return Err(ApiError::new(
            400,
            "UPLOAD_INCOMPLETE",
            "retry ended before the previously persisted prefix; original work data is retained",
        ));
    }
    record.persisted_bytes = length;
    record.persisted_hash = hash.clone().finalize().to_hex().to_string();
    persist_upload(&spool, &file, &record, &admission, &staged_file).await?;
    if length != expected_length {
        return Err(ApiError::new(
            400,
            "UPLOAD_INCOMPLETE",
            "upload ended before its declared length; durable work data is retained",
        ));
    }
    let actual_hash = hash.finalize().to_hex().to_string();
    if expected_hash.is_some_and(|expected| expected != actual_hash) {
        record.state = "checksum_mismatch".into();
        persist_upload(&spool, &file, &record, &admission, &staged_file).await?;
        return Err(ApiError::new(
            400,
            "CHECKSUM_MISMATCH",
            "uploaded bytes do not match the declared hash; work data is retained",
        ));
    }
    record.content_hash = Some(actual_hash);
    drop(file);
    let backend = state.backend.clone();
    let owner = identity.clone();
    let ws = workspace.clone();
    let item_id = item.clone();
    let logical_path = query.path;
    let staged = tokio::task::spawn_blocking(move || {
        // A disconnected HTTP request cannot cancel a running blocking task.
        // Keep its admission slot and per-item claim with publication itself,
        // including the final durable status, so a retry cannot race it.
        let _admission = admission;
        let staged =
            backend.upload_generation(&owner, &ws, &item_id, &logical_path, &path, generation)?;
        record.state = "complete".into();
        uploads::persist_staged(&spool, &record, &staged_file)?;
        Ok::<_, ApiError>(staged)
    })
    .await
    .map_err(ApiError::invalid)??;
    Ok(Json(staged))
}

fn parse_range(value: &str, size: u64) -> Result<(u64, u64), ApiError> {
    let value = value
        .strip_prefix("bytes=")
        .ok_or_else(|| ApiError::new(416, "RANGE_NOT_SATISFIABLE", "unsupported byte range"))?;
    if value.contains(',') || size == 0 {
        return Err(ApiError::new(
            416,
            "RANGE_NOT_SATISFIABLE",
            "only one satisfiable byte range is supported",
        ));
    }
    let (start, end) = value
        .split_once('-')
        .ok_or_else(|| invalid_range("invalid byte range"))?;
    let parse = |value: &str| {
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid_range("invalid byte range"));
        }
        value
            .parse::<u64>()
            .map_err(|_| invalid_range("invalid byte range"))
    };
    let (start, end) = if start.is_empty() {
        let suffix = parse(end)?;
        if suffix == 0 {
            return Err(ApiError::new(
                416,
                "RANGE_NOT_SATISFIABLE",
                "empty suffix range",
            ));
        }
        (size.saturating_sub(suffix), size - 1)
    } else {
        let start = parse(start)?;
        let end = if end.is_empty() {
            size - 1
        } else {
            parse(end)?.min(size - 1)
        };
        (start, end)
    };
    if start >= size || start > end {
        return Err(ApiError::new(
            416,
            "RANGE_NOT_SATISFIABLE",
            "byte range is outside the file",
        ));
    }
    Ok((start, end))
}

fn invalid_range(message: &str) -> ApiError {
    ApiError::new(416, "RANGE_NOT_SATISFIABLE", message)
}

fn unsatisfiable_range(error: ApiError, size: u64, method: &Method) -> Response {
    let mut response = error.into_response();
    response.headers_mut().insert(
        header::CONTENT_RANGE,
        HeaderValue::from_str(&format!("bytes */{size}")).unwrap(),
    );
    response
        .headers_mut()
        .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if *method == Method::HEAD {
        *response.body_mut() = Body::empty();
    }
    response
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContentQuery {
    repository_id: Option<String>,
    path: Option<String>,
}
pub(crate) async fn content(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<Identity>,
    RoutePath(version): RoutePath<String>,
    Query(query): Query<ContentQuery>,
    method: Method,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let permit = state
        .read_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::limit("download capacity reached"))?;
    let content = tokio::task::spawn_blocking(move || {
        if query.repository_id.is_some() != query.path.is_some() {
            return Err(ApiError::invalid(
                "repository_id and path must be supplied together",
            ));
        }
        let logical_name = if let (Some(repo), Some(path)) = (query.repository_id, query.path) {
            let metadata = state.backend.call(
                crate::BackendAction::FileManagerDownloadContext,
                &identity,
                json!({
                    "repository_id":repo,"path":path,"version_id":version
                }),
            )?;
            Some(
                metadata
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ApiError::new(500, "STATE_INVALID", "download context has no filename")
                    })?
                    .to_owned(),
            )
        } else {
            None
        };
        let mut content = state.backend.range_content(&identity, &version)?;
        if logical_name.is_some() {
            content.download_name = logical_name;
        }
        content.lease = Box::new((content.lease, permit));
        Ok(content)
    })
    .await
    .map_err(ApiError::invalid)??;
    content_response(content, method, headers)
}
pub(crate) async fn workspace_content(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<Identity>,
    RoutePath((workspace, item)): RoutePath<(String, String)>,
    method: Method,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let permit = state
        .read_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::limit("download capacity reached"))?;
    let content = tokio::task::spawn_blocking(move || {
        let mut content = state
            .backend
            .workspace_content(&identity, &workspace, &item)?;
        content.lease = Box::new((content.lease, permit));
        Ok::<_, ApiError>(content)
    })
    .await
    .map_err(ApiError::invalid)??;
    content_response(content, method, headers)
}
pub(crate) async fn checksums(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<Identity>,
    RoutePath(version): RoutePath<String>,
) -> Result<Response, ApiError> {
    let permit = state
        .read_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::limit("download capacity reached"))?;
    let bytes = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        state.backend.checksums(&identity, &version)
    })
    .await
    .map_err(ApiError::invalid)??;
    Ok(([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response())
}
fn content_response(
    content: Content,
    method: Method,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let manifest = content.checksum_manifest;
    let download_name = content.download_name;
    let mut file = content.file;
    let size = content.size;
    let hash = content.hash;
    let lease = content.lease;
    let etag = format!("\"{hash}\"");
    let range = if headers
        .get(header::IF_RANGE)
        .is_some_and(|value| value.to_str().ok() != Some(etag.as_str()))
    {
        None
    } else {
        headers.get(header::RANGE)
    };
    if headers.get_all(header::RANGE).iter().count() > 1 {
        return Ok(unsatisfiable_range(
            invalid_range("duplicate range header"),
            size,
            &method,
        ));
    }
    let (status, start, length) = if let Some(range) = range {
        match range
            .to_str()
            .map_err(|_| invalid_range("invalid byte range"))
            .and_then(|range| parse_range(range, size))
        {
            Ok((start, end)) => (StatusCode::PARTIAL_CONTENT, start, end - start + 1),
            Err(error) => {
                return Ok(unsatisfiable_range(error, size, &method));
            }
        }
    } else {
        (StatusCode::OK, 0, size)
    };

    let body = if method == Method::HEAD {
        Body::empty()
    } else if let Some(manifest) = manifest {
        let reader = VerifiedRange::new(file, content.fallback_files, manifest, start, length)?;
        // One bounded chunk per blocking job. The state, lease and admission
        // permit travel with that job, including after HTTP cancellation.
        let stream =
            futures_util::stream::try_unfold((reader, lease), |(mut reader, lease)| async move {
                tokio::task::spawn_blocking(move || {
                    reader.next_chunk().map(|chunk| {
                        chunk.map(|chunk| (bytes::Bytes::from(chunk), (reader, lease)))
                    })
                })
                .await
                .map_err(std::io::Error::other)?
            });
        Body::from_stream(stream)
    } else {
        file.seek(SeekFrom::Start(start)).map_err(ApiError::io)?;
        let stream = ReaderStream::new(tokio::io::AsyncReadExt::take(
            tokio::fs::File::from_std(file),
            length,
        ));
        let stream = stream.map(move |chunk| {
            let _keep_lease = &lease;
            chunk
        });
        Body::from_stream(stream)
    };
    let mut response = Response::new(body);
    *response.status_mut() = status;
    let headers = response.headers_mut();
    if let Some(name) = download_name {
        let fallback: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let mut encoded = String::new();
        for byte in name.as_bytes() {
            if byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b'-') {
                encoded.push(*byte as char);
            } else {
                encoded.push_str(&format!("%{byte:02X}"));
            }
        }
        headers.insert(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!(
                "attachment; filename=\"{fallback}\"; filename*=UTF-8''{encoded}"
            ))
            .map_err(ApiError::invalid)?,
        );
    }
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&length.to_string()).unwrap(),
    );
    headers.insert(
        header::ETAG,
        HeaderValue::from_str(&etag).map_err(ApiError::invalid)?,
    );
    if status == StatusCode::PARTIAL_CONTENT {
        headers.insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{}/{size}", start + length - 1)).unwrap(),
        );
    }
    Ok(response)
}

/// Reads the entire authenticated chunk into a private buffer and exposes only
/// verified bytes from that same buffer. Never verify then reread the payload.
struct VerifiedRange<R> {
    file: R,
    fallbacks: Vec<R>,
    manifest: storage_contract::ChecksumManifest,
    position: u64,
    end: u64,
}
impl<R: std::io::Read + Seek> VerifiedRange<R> {
    fn new(
        file: R,
        fallbacks: Vec<R>,
        manifest: storage_contract::ChecksumManifest,
        start: u64,
        length: u64,
    ) -> Result<Self, ApiError> {
        let end = start
            .checked_add(length)
            .filter(|end| *end <= manifest.logical_size)
            .ok_or_else(|| ApiError::invalid("range exceeds authenticated size"))?;
        if !storage_contract::SUPPORTED_CHECKSUM_CHUNK_BYTES.contains(&manifest.chunk_bytes)
            || manifest.chunks.len() as u64
                != manifest.logical_size.div_ceil(manifest.chunk_bytes as u64)
        {
            return Err(ApiError::invalid("invalid checksum manifest"));
        }
        Ok(Self {
            file,
            fallbacks,
            manifest,
            position: start,
            end,
        })
    }
    fn next_chunk(&mut self) -> std::io::Result<Option<Vec<u8>>> {
        if self.position == self.end {
            return Ok(None);
        }
        let chunk_bytes = self.manifest.chunk_bytes as u64;
        let index = self.position / chunk_bytes;
        let base = index * chunk_bytes;
        let count = chunk_bytes.min(self.manifest.logical_size - base) as usize;
        let mut buffer = vec![0; count];
        let expected = &self.manifest.chunks[index as usize];
        let mut verified = false;
        for file in std::iter::once(&mut self.file).chain(self.fallbacks.iter_mut()) {
            if file.seek(SeekFrom::Start(base)).is_ok()
                && file.read_exact(&mut buffer).is_ok()
                && blake3::hash(&buffer).as_bytes() == expected
            {
                verified = true;
                break;
            }
        }
        if !verified {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "CHECKSUM_MISMATCH: no replica has a verified payload chunk",
            ));
        }
        let from = (self.position - base) as usize;
        let to = (self.end - base).min(count as u64) as usize;
        self.position = base + to as u64;
        buffer.truncate(to);
        buffer.drain(..from);
        Ok(Some(buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct CountingReader {
        inner: std::io::Cursor<Vec<u8>>,
        bytes: Arc<AtomicUsize>,
    }
    impl std::io::Read for CountingReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let n = std::io::Read::read(&mut self.inner, buffer)?;
            self.bytes.fetch_add(n, Ordering::SeqCst);
            Ok(n)
        }
    }
    impl Seek for CountingReader {
        fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(position)
        }
    }
    fn counted(bytes: Vec<u8>, count: Arc<AtomicUsize>) -> CountingReader {
        CountingReader {
            inner: std::io::Cursor::new(bytes),
            bytes: count,
        }
    }
    fn manifest(bytes: &[u8]) -> storage_contract::ChecksumManifest {
        storage_contract::ChecksumManifest {
            chunk_bytes: 65536,
            logical_size: bytes.len() as u64,
            chunks: bytes
                .chunks(65536)
                .map(|chunk| *blake3::hash(chunk).as_bytes())
                .collect(),
        }
    }
    #[test]
    fn verified_ranges_read_only_touched_chunks_and_fallback_without_exposing_corruption() {
        let bytes = vec![42; 65536 * 4 + 7];
        let count = Arc::new(AtomicUsize::new(0));
        let mut reader = VerifiedRange::new(
            counted(bytes.clone(), count.clone()),
            vec![],
            manifest(&bytes),
            65535,
            2,
        )
        .unwrap();
        assert_eq!(reader.next_chunk().unwrap().unwrap(), [42]);
        assert_eq!(reader.next_chunk().unwrap().unwrap(), [42]);
        assert!(reader.next_chunk().unwrap().is_none());
        assert_eq!(count.load(Ordering::SeqCst), 2 * 65536);
        let mut corrupt = bytes.clone();
        corrupt[65536] = 0;
        let bad_count = Arc::new(AtomicUsize::new(0));
        let good_count = Arc::new(AtomicUsize::new(0));
        let mut reader = VerifiedRange::new(
            counted(corrupt.clone(), bad_count.clone()),
            vec![counted(bytes.clone(), good_count.clone())],
            manifest(&bytes),
            65537,
            1,
        )
        .unwrap();
        assert_eq!(reader.next_chunk().unwrap().unwrap(), [42]);
        assert_eq!(bad_count.load(Ordering::SeqCst), 65536);
        assert_eq!(good_count.load(Ordering::SeqCst), 65536);
        let mut reader = VerifiedRange::new(
            counted(corrupt, Arc::default()),
            vec![],
            manifest(&bytes),
            65537,
            1,
        )
        .unwrap();
        assert_eq!(
            reader.next_chunk().unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        // A short tail checks only actual remaining bytes, not a padded chunk.
        let tail_count = Arc::new(AtomicUsize::new(0));
        let mut reader = VerifiedRange::new(
            counted(bytes.clone(), tail_count.clone()),
            vec![],
            manifest(&bytes),
            (bytes.len() - 1) as u64,
            1,
        )
        .unwrap();
        assert_eq!(reader.next_chunk().unwrap().unwrap(), [42]);
        assert_eq!(tail_count.load(Ordering::SeqCst), 7);
    }

    struct ReadLease(Arc<AtomicUsize>);
    impl Drop for ReadLease {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn content(lease: Arc<AtomicUsize>) -> Content {
        let mut file = tempfile::tempfile().unwrap();
        std::io::Write::write_all(&mut file, b"0123456789").unwrap();
        Content {
            checksum_manifest: None,
            fallback_files: Vec::new(),
            file,
            size: 10,
            hash: "hash".into(),
            lease: Box::new(ReadLease(lease)),
            download_name: None,
        }
    }

    #[tokio::test]
    async fn verified_stream_preserves_lease_and_admission_until_consumption_or_drop() {
        for consume in [true, false] {
            let slots = Arc::new(tokio::sync::Semaphore::new(1));
            let permit = slots.clone().try_acquire_owned().unwrap();
            let dropped = Arc::new(AtomicUsize::new(0));
            let mut content = content(dropped.clone());
            content.checksum_manifest = Some(manifest(b"0123456789"));
            content.lease = Box::new((content.lease, permit));
            let response = content_response(content, Method::GET, HeaderMap::new()).unwrap();
            assert_eq!(dropped.load(Ordering::SeqCst), 0);
            assert_eq!(slots.available_permits(), 0);
            if consume {
                assert_eq!(
                    axum::body::to_bytes(response.into_body(), 8192)
                        .await
                        .unwrap(),
                    "0123456789"
                );
            } else {
                drop(response);
            }
            assert_eq!(dropped.load(Ordering::SeqCst), 1);
            assert_eq!(slots.available_permits(), 1);
        }
    }

    #[test]
    fn ranges_obey_boundaries_and_reject_non_decimal_syntax() {
        assert_eq!(parse_range("bytes=2-8", 5).unwrap(), (2, 4));
        assert_eq!(parse_range("bytes=-2", 5).unwrap(), (3, 4));
        assert_eq!(parse_range("bytes=3-", 5).unwrap(), (3, 4));
        assert_eq!(
            parse_range("bytes=-18446744073709551615", 5).unwrap(),
            (0, 4)
        );
        assert_eq!(
            parse_range("bytes=0-", u64::MAX).unwrap(),
            (0, u64::MAX - 1)
        );
        for invalid in [
            "bytes=5-",
            "bytes=0-1,3-4",
            "bytes=-0",
            "bytes=missing",
            "bytes=+1-2",
            "bytes=1-+2",
            "bytes=18446744073709551616-",
            "bytes= 1-2",
        ] {
            assert_eq!(
                parse_range(invalid, 5).unwrap_err().status,
                416,
                "{invalid}"
            );
        }
        assert_eq!(parse_range("bytes=0-0", 0).unwrap_err().status, 416);
    }

    #[tokio::test]
    async fn malformed_duplicate_and_unsatisfiable_ranges_include_size_and_head_is_empty() {
        for method in [Method::GET, Method::HEAD] {
            for invalid in ["bytes=missing", "bytes=+1-2", "bytes=20-"] {
                let mut headers = HeaderMap::new();
                headers.insert(header::RANGE, HeaderValue::from_str(invalid).unwrap());
                let response =
                    content_response(content(Arc::default()), method.clone(), headers).unwrap();
                assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
                assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes */10");
                assert_eq!(response.headers()[header::ACCEPT_RANGES], "bytes");
                let body = axum::body::to_bytes(response.into_body(), 8192)
                    .await
                    .unwrap();
                assert_eq!(body.is_empty(), method == Method::HEAD);
            }
            let mut headers = HeaderMap::new();
            headers.append(header::RANGE, HeaderValue::from_static("bytes=0-1"));
            headers.append(header::RANGE, HeaderValue::from_static("bytes=2-3"));
            let response =
                content_response(content(Arc::default()), method.clone(), headers).unwrap();
            assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
            assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes */10");
        }
    }

    #[tokio::test]
    async fn download_lease_survives_until_body_consumption_or_cancellation() {
        for consume in [true, false] {
            let dropped = Arc::new(AtomicUsize::new(0));
            let response =
                content_response(content(dropped.clone()), Method::GET, HeaderMap::new()).unwrap();
            assert_eq!(dropped.load(Ordering::SeqCst), 0);
            if consume {
                assert_eq!(
                    axum::body::to_bytes(response.into_body(), 8192)
                        .await
                        .unwrap(),
                    "0123456789"
                );
            } else {
                drop(response);
            }
            assert_eq!(dropped.load(Ordering::SeqCst), 1);
        }
    }

    #[tokio::test]
    async fn stale_if_range_returns_complete_content_and_head_retains_length() {
        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, HeaderValue::from_static("bytes=2-3"));
        headers.insert(header::IF_RANGE, HeaderValue::from_static("\"stale\""));
        let response = content_response(content(Arc::default()), Method::GET, headers).unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            axum::body::to_bytes(response.into_body(), 8192)
                .await
                .unwrap(),
            "0123456789"
        );
        let dropped = Arc::new(AtomicUsize::new(0));
        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, HeaderValue::from_static("bytes=2-3"));
        let response = content_response(content(dropped.clone()), Method::HEAD, headers).unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()[header::CONTENT_LENGTH], "2");
        assert!(
            axum::body::to_bytes(response.into_body(), 8192)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    }
}
