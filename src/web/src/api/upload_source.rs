//! Browser-selected files with bounded buffering before upload.
use super::{Error, error};

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
        Some(self.0.size() as u64)
    }
    pub fn preflight(&self) -> Result<(), Error> {
        browser_size(self.0.size())
    }
    pub(super) fn into_file(self) -> web_sys::File {
        self.0
    }
}

fn browser_size(size: f64) -> Result<(), Error> {
    if !size.is_finite() || size < 0.0 || size > 64.0 * 1024.0 * 1024.0 {
        Err(error(
            "BROWSER_UPLOAD_LIMIT",
            web_interface::i18n::t(
                "网页单文件上传上限 64 MiB；较大文件请通过 SMB 访问",
                "Browser uploads are limited to 64 MiB per file. Access larger files through SMB.",
            ),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_preflight_accepts_the_limit_and_rejects_unbounded_buffering() {
        assert!(browser_size(0.0).is_ok());
        assert!(browser_size(64.0 * 1024.0 * 1024.0).is_ok());
        assert!(browser_size(64.0 * 1024.0 * 1024.0 + 1.0).is_err());
        for size in [f64::NAN, f64::INFINITY, -1.0] {
            assert!(browser_size(size).is_err());
        }
    }
}
