use date_time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};
use std::{
    io,
    sync::{Arc, Mutex},
};
use tracing::collector::fmt::{
    format::Writer,
    time::{FormatTime, OffsetTime, UtcTime},
};

#[derive(Clone)]
struct Capture(Arc<Mutex<Vec<u8>>>);
impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn check_timer<T: FormatTime + Send + Sync + 'static>(timer: T, offset: UtcOffset) {
    let before = OffsetDateTime::now_utc();
    let mut formatted = String::new();
    timer.format_time(&mut Writer::new(&mut formatted)).unwrap();
    let direct = OffsetDateTime::parse(&formatted, &Rfc3339).unwrap();
    assert_eq!(direct.offset(), offset);
    assert!(direct >= before);

    let output = Capture(Arc::new(Mutex::new(Vec::new())));
    let writer = output.clone();
    let subscriber = tracing::collector::fmt()
        .json()
        .with_timer(timer)
        .with_writer(move || writer.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(answer = 42u64, "date-time consumer")
    });
    let after = OffsetDateTime::now_utc();
    let bytes = output.0.lock().unwrap();
    let event: json_codec::Value = json_codec::from_slice(&bytes).unwrap();
    let stamp = OffsetDateTime::parse(event["timestamp"].as_str().unwrap(), &Rfc3339).unwrap();
    assert_eq!(stamp.offset(), offset);
    assert!(stamp >= before && stamp <= after);
    assert_eq!(event["fields"]["answer"], 42);
    assert_eq!(event["fields"]["message"], "date-time consumer");
}

#[test]
fn tracing_utc_timer_formats_real_utc_and_emits_parseable_json() {
    check_timer(UtcTime::rfc_3339(), UtcOffset::UTC);
}

#[test]
fn tracing_fixed_offset_timer_emits_plus_five_thirty_without_local_timezone() {
    let offset = UtcOffset::from_hms(5, 30, 0).unwrap();
    check_timer(OffsetTime::new(offset, Rfc3339), offset);
}
