//! One route boundary for native HTTP and browser/native WebView transports.
use crate::{Error, Result};

/// Accept only this service's `/v1/` routes before URL parsing can normalize
/// dot segments or strip control characters. Query values are not path parts.
pub fn validate_api_path(value: &str) -> Result<()> {
    let path = value.split('?').next().unwrap_or_default();
    let invalid = !path.starts_with("/v1/")
        || value.chars().any(char::is_control)
        || value.contains(['#', '\\'])
        || path.split('/').any(|part| {
            let part = part.to_ascii_lowercase().replace("%2e", ".");
            matches!(part.as_str(), "." | "..") || part.contains("%2f") || part.contains("%5c")
        });
    if invalid {
        Err(Error::new(
            "INVALID_API_PATH",
            "expected a service-local /v1/ API route",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_normalization_cannot_escape_the_api_boundary() {
        for value in [
            "/v1/../private",
            "/v1/%2e%2e/private",
            "/v1/.%2E/private",
            "/v1/%2e/private",
            "/v1/a%2Fb",
            "/v1/a%5Cb",
            "/v1/a\\b",
            "/v1/\t../private",
            "/v1/a\n",
            "/v1/a#fragment",
            "//example.org/v1/a",
            "https://example.org/v1/a",
            "/private",
            "/v1",
        ] {
            assert!(validate_api_path(value).is_err(), "accepted {value:?}");
        }
    }

    #[test]
    fn query_values_and_ordinary_encoded_names_remain_data() {
        for value in [
            "/v1/system/status",
            "/v1/files/%E6%96%87%E4%BB%B6",
            "/v1/files?path=a/../b&name=%2f%2e%5c",
            "/v1/files?path=./relative",
            "/v1/files/a..b",
            "/v1/files/%252e%252e",
        ] {
            assert!(validate_api_path(value).is_ok(), "rejected {value:?}");
        }
    }
}
