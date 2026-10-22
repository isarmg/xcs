//! Browser uploads hash bounded slices, then send the original immutable File.
use super::{Error, SessionAdmission, error, invalid_response, js_error};
use js_sys::Uint8Array;
use wasm_bindgen_futures::JsFuture;

const HASH_CHUNK_BYTES: u64 = 1024 * 1024;
const MAX_EXACT_BROWSER_SIZE: u64 = (1u64 << 53) - 1;

pub struct UploadSource(web_sys::File);

impl From<web_sys::File> for UploadSource {
    fn from(file: web_sys::File) -> Self {
        Self(file)
    }
}

impl UploadSource {
    pub fn name(&self) -> String {
        self.0.name()
    }
    pub fn size(&self) -> Option<u64> {
        browser_size(self.0.size()).ok()
    }
    pub fn preflight(&self) -> Result<(), Error> {
        browser_size(self.0.size()).map(|_| ())
    }
    pub(crate) fn preflight_limit(&self, limit: u64) -> Result<(), Error> {
        validate_limit(browser_size(self.0.size())?, limit)
    }
    pub(super) async fn prepare(
        &self,
        admission: SessionAdmission,
        total_timeout_ms: u32,
    ) -> Result<UploadMetadata, Error> {
        let size = browser_size(self.0.size())?;
        let hash = hash_chunks(
            size,
            || admission.check(),
            |start, end| async move {
                let slice = self
                    .0
                    .slice_with_f64_and_f64(start as f64, end as f64)
                    .map_err(js_error)?;
                let result = JsFuture::from(slice.array_buffer()).await;
                admission.check()?;
                let buffer = result.map_err(js_error)?;
                // Never copy a hostile/unexpected oversized result into WASM memory.
                let bytes = Uint8Array::new(&buffer);
                if u64::from(bytes.length()) != end - start {
                    return Err(invalid_response());
                }
                Ok(bytes.to_vec())
            },
        )
        .await?;
        Ok(UploadMetadata {
            size,
            hash,
            total_timeout_ms,
        })
    }
    pub(super) fn into_file(self) -> web_sys::File {
        self.0
    }
}

pub(super) struct UploadMetadata {
    pub size: u64,
    pub hash: String,
    pub total_timeout_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TransferLimits {
    pub idle_timeout_ms: u32,
    pub total_timeout_ms: u32,
}

pub(super) struct UploadLimits {
    pub max_upload_bytes: u64,
    pub transfer: TransferLimits,
}

pub(crate) async fn upload_limit() -> Result<u64, Error> {
    Ok(upload_limits().await?.max_upload_bytes)
}

pub(super) async fn upload_limits() -> Result<UploadLimits, Error> {
    let value = capabilities().await?;
    Ok(UploadLimits {
        max_upload_bytes: capability_limit(&value)?,
        transfer: parse_transfer_limits(&value)?,
    })
}

// Used by the streaming download module after integration; it intentionally
// does not require uploads to be enabled for a read-only transfer.
#[allow(dead_code)]
pub(super) async fn transfer_limits() -> Result<TransferLimits, Error> {
    parse_transfer_limits(&capabilities().await?)
}

async fn capabilities() -> Result<json_codec::Value, Error> {
    let admission = SessionAdmission::capture();
    let result = super::request("GET", "/api/capabilities", None).await;
    check_completion(admission, &result)?;
    result
}

fn invalid_transfer_limits() -> Error {
    error(
        "INVALID_TRANSFER_LIMITS",
        web_interface::i18n::t(
            "服务端传输超时配置缺失或无效；未开始文件传输",
            "The server transfer timeout configuration is missing or invalid. File transfer was not started.",
        ),
    )
}

fn parse_transfer_limits(value: &json_codec::Value) -> Result<TransferLimits, Error> {
    // Seven days is the server's validated upper bound; it is also below the
    // browser timer's signed 32-bit millisecond limit. Never truncate/wrap.
    const MAX_TRANSFER_MS: u64 = 7 * 24 * 60 * 60 * 1000;
    let timeout = |key: &str| -> Result<u32, Error> {
        let millis = storage_contract::parse_u64_decimal(
            value[key].as_str().ok_or_else(invalid_transfer_limits)?,
        )
        .map_err(|_| invalid_transfer_limits())?;
        if millis == 0 || millis > MAX_TRANSFER_MS || millis > i32::MAX as u64 {
            return Err(invalid_transfer_limits());
        }
        u32::try_from(millis).map_err(|_| invalid_transfer_limits())
    };
    let limits = TransferLimits {
        idle_timeout_ms: timeout("transfer_idle_timeout_ms")?,
        total_timeout_ms: timeout("transfer_total_timeout_ms")?,
    };
    if limits.idle_timeout_ms > limits.total_timeout_ms {
        return Err(invalid_transfer_limits());
    }
    Ok(limits)
}

// decode deliberately revokes the originating credentials on HTTP 401. Keep
// that status so the UI logs out, rather than disguising it as a local failure.
pub(super) fn check_completion<T>(
    admission: SessionAdmission,
    result: &Result<T, Error>,
) -> Result<(), Error> {
    admission.check().map_err(|mut error| {
        if let Err(response) = result {
            error.status = response.status;
            error.request_id = response.request_id.clone();
        }
        error
    })
}

fn capability_limit(value: &json_codec::Value) -> Result<u64, Error> {
    if value["complete_upload"].as_bool() != Some(true) {
        return Err(invalid_response());
    }
    storage_contract::parse_u64_decimal(
        value["max_upload_bytes"]
            .as_str()
            .ok_or_else(invalid_response)?,
    )
    .map_err(|_| invalid_response())
}

fn browser_size(size: f64) -> Result<u64, Error> {
    if !size.is_finite()
        || size < 0.0
        || size.fract() != 0.0
        || size > MAX_EXACT_BROWSER_SIZE as f64
    {
        Err(error(
            "BROWSER_UPLOAD_LIMIT",
            web_interface::i18n::t(
                "浏览器无法精确表示此文件大小；请通过 SMB 访问",
                "The browser cannot represent this file size exactly. Access it through SMB.",
            ),
        ))
    } else {
        Ok(size as u64)
    }
}

fn validate_limit(size: u64, limit: u64) -> Result<(), Error> {
    if size > limit {
        Err(error(
            "UPLOAD_TOO_LARGE",
            web_interface::i18n::t(
                &format!("文件超过服务端上传上限（{limit} 字节）；请联系管理员或使用 SMB"),
                &format!(
                    "The file exceeds the server upload limit ({limit} bytes). Contact the administrator or use SMB."
                ),
            ),
        ))
    } else {
        Ok(())
    }
}

// The guard runs on both sides of every asynchronous read, including failures.
// Dropping this future cancels preparation: there is no detached hashing task.
async fn hash_chunks<F, Fut>(
    size: u64,
    check: impl Fn() -> Result<(), Error>,
    mut read: F,
) -> Result<String, Error>
where
    F: FnMut(u64, u64) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<u8>, Error>>,
{
    let mut hasher = blake3::Hasher::new();
    let mut offset = 0;
    check()?;
    while offset < size {
        check()?;
        let end = size.min(offset + HASH_CHUNK_BYTES);
        let result = read(offset, end).await;
        check()?;
        let bytes = result?;
        if bytes.len() as u64 != end - offset {
            return Err(invalid_response());
        }
        hasher.update(&bytes);
        offset = end;
    }
    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
#[path = "../../../../test/web/unit/api/upload_source/tests.rs"]
mod tests;
