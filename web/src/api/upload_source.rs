//! Owned upload inputs. Native descriptors are opaque, one-shot capabilities;
//! dropping any unused queue entry releases its native handle.
use super::{Error, error, invalid_response, invoke};
use serde::Deserialize;
use serde_json::{Value, json};
use std::future::Future;

pub enum UploadSource {
    Browser(web_sys::File),
    Native(NativeSource),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSource {
    pub id: String,
    pub name: String,
    pub size: String,
}
impl Drop for NativeSource {
    fn drop(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            let id = self.id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = release_sources(vec![id], invoke).await;
            });
        }
    }
}
impl UploadSource {
    pub fn name(&self) -> String {
        match self {
            Self::Browser(file) => file.name(),
            Self::Native(source) => source.name.clone(),
        }
    }
    pub fn size(&self) -> Option<u64> {
        match self {
            Self::Browser(file) => Some(file.size() as u64),
            Self::Native(source) => source.size.parse().ok(),
        }
    }
    pub fn preflight(&self) -> Result<(), Error> {
        match self {
            Self::Browser(file) => browser_size(file.size()),
            Self::Native(source) if source.id.is_empty() || source.size.parse::<u64>().is_err() => {
                Err(invalid_response())
            }
            Self::Native(_) => Ok(()),
        }
    }
}
fn browser_size(size: f64) -> Result<(), Error> {
    if !size.is_finite() || size < 0.0 || size > 64.0 * 1024.0 * 1024.0 {
        Err(error(
            "BROWSER_UPLOAD_LIMIT",
            zxs_web::i18n::t(
                "网页单次上传上限 64 MiB；大文件请使用 App 或 CLI",
                "Browser uploads are limited to 64 MiB per file. Use the app or CLI for larger files.",
            ),
        ))
    } else {
        Ok(())
    }
}
pub async fn choose_files(multiple: bool) -> Result<Vec<UploadSource>, Error> {
    choose_files_with(multiple, invoke).await
}
async fn choose_files_with<F, Fut>(multiple: bool, mut call: F) -> Result<Vec<UploadSource>, Error>
where
    F: FnMut(&'static str, Value) -> Fut,
    Fut: Future<Output = Result<Value, Error>>,
{
    let value = call("choose_files", json!({"multiple":multiple})).await?;
    // Release even a malformed response's valid handles, rather than leaking
    // earlier selections when a later item fails validation.
    match serde_json::from_value::<Vec<NativeSource>>(value.clone()) {
        Ok(sources) => Ok(sources.into_iter().map(UploadSource::Native).collect()),
        Err(_) => {
            let ids: Vec<_> = value
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v["id"].as_str())
                .collect();
            let _ = call("release_sources", json!({"sourceIds":ids})).await;
            Err(invalid_response())
        }
    }
}
#[cfg(any(target_arch = "wasm32", test))]
async fn release_sources<F, Fut>(ids: Vec<String>, call: F) -> Result<Value, Error>
where
    F: FnOnce(&'static str, Value) -> Fut,
    Fut: Future<Output = Result<Value, Error>>,
{
    call("release_sources", json!({"sourceIds":ids})).await
}
pub(super) async fn upload_native<F, Fut>(
    workspace: &str,
    item: &str,
    path: &str,
    source_id: &str,
    call: F,
) -> Result<Value, Error>
where
    F: FnOnce(&'static str, Value) -> Fut,
    Fut: Future<Output = Result<Value, Error>>,
{
    call(
        "upload_file",
        json!({"workspace":workspace,"item":item,"path":path,"sourceId":source_id}),
    )
    .await
}
#[cfg(test)]
mod tests {
    use super::*;
    fn run_ready<F: Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(value) => value,
            std::task::Poll::Pending => panic!("fake bridge unexpectedly suspended"),
        }
    }
    #[test]
    fn fake_native_picker_upload_release_and_cancel_follow_the_bridge_contract() {
        let calls = std::cell::RefCell::new(Vec::new());
        let bridge = |command: &'static str, args: Value| {
            calls.borrow_mut().push((command, args));
            std::future::ready(Ok(match command {
                "choose_files" => json!([{"id":"opaque","name":"large.bin","size":"9999999999"}]),
                "upload_file" => json!({"uploaded":true}),
                "release_sources" => Value::Null,
                _ => panic!("unexpected command"),
            }))
        };
        let sources = run_ready(choose_files_with(true, bridge)).unwrap();
        assert_eq!(sources.len(), 1);
        let UploadSource::Native(source) = &sources[0] else {
            panic!("expected native")
        };
        assert_eq!(source.name, "large.bin");
        assert!(sources[0].preflight().is_ok());
        run_ready(upload_native(
            "workspace",
            "item",
            "logical",
            &source.id,
            bridge,
        ))
        .unwrap();
        run_ready(release_sources(vec![source.id.clone()], bridge)).unwrap();
        assert_eq!(
            calls
                .borrow()
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>(),
            ["choose_files", "upload_file", "release_sources"]
        );
        assert_eq!(calls.borrow()[0].1, json!({"multiple":true}));
        assert_eq!(calls.borrow()[2].1, json!({"sourceIds":["opaque"]}));
        let cancelled = run_ready(choose_files_with(false, |_, _| {
            std::future::ready(Ok(json!([])))
        }))
        .unwrap();
        assert!(cancelled.is_empty());
    }
    #[test]
    fn malformed_picker_result_releases_all_recoverable_handles() {
        let mut calls = Vec::new();
        let result = run_ready(choose_files_with(true, |command, args| {
            calls.push((command, args));
            std::future::ready(Ok(if command == "choose_files" {
                json!([{"id":"first","name":"a","size":"4"},{"id":"bad","name":"b"}])
            } else {
                Value::Null
            }))
        }));
        assert!(result.is_err());
        assert_eq!(
            calls[1],
            ("release_sources", json!({"sourceIds":["first","bad"]}))
        );
    }
    #[test]
    fn browser_preflight_is_bounded_but_native_metadata_is_not() {
        assert!(browser_size(64.0 * 1024.0 * 1024.0).is_ok());
        assert!(browser_size(64.0 * 1024.0 * 1024.0 + 1.0).is_err());
        for size in [f64::NAN, f64::INFINITY, -1.0] {
            assert!(browser_size(size).is_err());
        }
        let source = UploadSource::Native(NativeSource {
            id: "opaque".into(),
            name: "large".into(),
            size: u64::MAX.to_string(),
        });
        assert!(source.preflight().is_ok());
    }
    #[test]
    fn fake_native_bridge_receives_only_the_capability_and_logical_target() {
        let mut future = std::pin::pin!(upload_native(
            "ws",
            "item",
            "logical/file",
            "opaque",
            |command, args| {
                assert_eq!(command, "upload_file");
                assert_eq!(
                    args,
                    json!({"workspace":"ws","item":"item","path":"logical/file","sourceId":"opaque"})
                );
                std::future::ready(Ok(json!({"uploaded":true})))
            }
        ));
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(
            matches!(future.as_mut().poll(&mut cx), std::task::Poll::Ready(Ok(value)) if value["uploaded"] == true)
        );
    }
}
