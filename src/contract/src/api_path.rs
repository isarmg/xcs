//! One route boundary for operational CLI HTTP and browser HTTP transports.
use crate::{Error, Result};

/// Accept only this service's `/api/` routes before URL parsing can normalize
/// dot segments or strip control characters. Query values are not path parts.
pub fn validate_api_path(value: &str) -> Result<()> {
    let path = value.split('?').next().unwrap_or_default();
    let invalid = !path.starts_with("/api/")
        || value.chars().any(char::is_control)
        || value.contains(['#', '\\'])
        || path.split('/').any(|part| {
            let part = part.to_ascii_lowercase().replace("%2e", ".");
            matches!(part.as_str(), "." | "..") || part.contains("%2f") || part.contains("%5c")
        });
    if invalid {
        Err(Error::new(
            "INVALID_API_PATH",
            "expected a service-local /api/ API route",
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
            "/api/../private",
            "/api/%2e%2e/private",
            "/api/.%2E/private",
            "/api/%2e/private",
            "/api/a%2Fb",
            "/api/a%5Cb",
            "/api/a\\b",
            "/api/\t../private",
            "/api/a\n",
            "/api/a#fragment",
            "//example.org/api/a",
            "https://example.org/api/a",
            "/private",
            "/v1/repositories",
            "/api",
        ] {
            assert!(validate_api_path(value).is_err(), "accepted {value:?}");
        }
    }

    #[test]
    fn query_values_and_ordinary_encoded_names_remain_data() {
        for value in [
            "/api/system/status",
            "/api/files/%E6%96%87%E4%BB%B6",
            "/api/files?path=a/../b&name=%2f%2e%5c",
            "/api/files?path=./relative",
            "/api/files/a..b",
            "/api/files/%252e%252e",
        ] {
            assert!(validate_api_path(value).is_ok(), "rejected {value:?}");
        }
    }
}
