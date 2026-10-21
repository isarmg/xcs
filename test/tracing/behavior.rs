use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tracing::collector::{prelude::*, EnvFilter};

#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);
impl Write for Buffer {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing::collector::fmt::MakeWriter<'a> for Buffer {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}
fn capture(flatten: bool, run: impl FnOnce()) -> Vec<json_codec::Value> {
    let output = Buffer::default();
    let subscriber = tracing::collector::fmt()
        .json()
        .flatten_event(flatten)
        .without_time()
        .with_ansi(false)
        .with_writer(output.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, run);
    let bytes = output.0.lock().unwrap();
    String::from_utf8(bytes.clone())
        .unwrap()
        .lines()
        .map(|s| json_codec::from_str(s).unwrap())
        .collect()
}
#[test]
fn json_typed_fields_and_escaping() {
    let lines = capture(false, || {
        tracing::info!(signed = -3i64, unsigned = 7u64, flag = true, fraction = 1.25f64,
                       bytes = &b"ab"[..], text = "quote\"\n☃", r#type = "value", debug = ?vec![1, 2]);
    });
    let f = &lines[0]["fields"];
    assert_eq!(f["signed"], -3);
    assert_eq!(f["unsigned"], 7);
    assert_eq!(f["flag"], true);
    assert_eq!(f["fraction"], 1.25);
    assert_eq!(f["bytes"], json_codec::json!([97, 98]));
    assert_eq!(f["text"], "quote\"\n☃");
    assert_eq!(f["type"], "value");
    assert_eq!(f["debug"], "[1, 2]");
    assert_eq!(lines[0]["level"], "INFO");
}
#[test]
fn json_flattened_fields_and_span_context() {
    let lines = capture(true, || {
        let span = tracing::info_span!("request", id = 17u64);
        let _entered = span.enter();
        tracing::info!(answer = 42u64, "done");
    });
    assert_eq!(lines[0]["answer"], 42);
    assert_eq!(lines[0]["message"], "done");
    assert_eq!(lines[0]["span"]["id"], 17);
    assert_eq!(lines[0]["span"]["name"], "request");
    assert!(lines[0].get("fields").is_none());
}
#[test]
fn env_filter_and_layering() {
    let output = Buffer::default();
    let subscriber = tracing::collector::registry()
        .with(EnvFilter::new("warn,tracing_behavior=info"))
        .with(
            tracing::collector::fmt::layer()
                .without_time()
                .with_ansi(false)
                .with_writer(output.clone()),
        );
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(target: "tracing_behavior", "accepted");
        tracing::info!(target: "other", "rejected");
        tracing::warn!(target: "other", "warning");
    });
    let bytes = output.0.lock().unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("accepted"));
    assert!(text.contains("warning"));
    assert!(!text.contains("rejected"));
}
#[tracing::instrument]
fn instrumented(value: u64) -> u64 {
    tracing::info!("inside");
    value + 1
}
#[tracing::instrument]
async fn instrumented_async(value: u64) -> u64 {
    tracing::info!("async inside");
    value + 2
}
#[test]
fn generated_attribute_paths_sync_and_async() {
    let lines = capture(false, || {
        assert_eq!(instrumented(4), 5);
        let future = instrumented_async(5);
        let mut future = std::pin::pin!(future);
        let waker = std::task::Waker::noop();
        let mut context = std::task::Context::from_waker(waker);
        assert_eq!(
            std::future::Future::poll(future.as_mut(), &mut context),
            std::task::Poll::Ready(7)
        );
    });
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["span"]["name"], "instrumented");
    assert_eq!(lines[1]["span"]["name"], "instrumented_async");
}
#[test]
fn malformed_directives_are_rejected() {
    assert!(EnvFilter::try_new("app[span{value=\"[\"}]=nonsense").is_err());
    assert!(EnvFilter::try_new("info").is_ok());
}
#[test]
fn explicit_utc_clock_formats() {
    let output = Buffer::default();
    let subscriber = tracing::collector::fmt()
        .with_ansi(false)
        .with_timer(tracing::collector::fmt::time::UtcTime::rfc_3339())
        .with_writer(output.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, || tracing::info!("clock"));
    let bytes = output.0.lock().unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains('T') && text.contains('Z') && text.contains("clock"));
}

#[test]
fn flattened_reserved_field_order_is_explicit() {
    let lines = capture(true, || {
        tracing::info!(target: "metadata-target", level = "user-level", target = "user-target");
    });
    // Fields follow level, but event target follows fields. Keep that established
    // ordering rather than silently building a map that changes duplicate keys.
    assert_eq!(lines[0]["level"], "user-level");
    assert_eq!(lines[0]["target"], "metadata-target");
}
