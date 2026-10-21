use super::{Format, FormatEvent, FormatFields, FormatTime, Writer};
use crate::collector::{
    field::{RecordFields, VisitOutput},
    fmt::{
        fmt_layer::{FmtContext, FormattedFields},
        writer::WriteAdaptor,
    },
    registry::LookupSpan,
};
use alloc::{
    collections::BTreeMap,
    fmt::{self, Write},
    format,
    string::String,
};
use data_serialization::ser::{SerializeMap, Serializer as _};
use json_codec::Serializer;
use tracing::primitives::{
    field::{self, Field},
    span::Record,
    Event, Subscriber,
};

#[cfg(feature = "log-bridge")]
use tracing::log_bridge::NormalizeEvent;

// Write event fields directly to the output serializer. This avoids an
// intermediate JSON value tree and preserves primitive types and errors.
struct EventFields<'a, 'b>(&'a Event<'b>);

impl data_serialization::Serialize for EventFields<'_, '_> {
    fn serialize<S: data_serialization::Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        record_event(self.0, &mut map)?;
        map.end()
    }
}

fn record_event<M: SerializeMap>(event: &Event<'_>, map: &mut M) -> Result<(), M::Error> {
    let mut visitor = EventVisitor {
        map,
        result: Ok(()),
    };
    event.record(&mut visitor);
    visitor.result
}

struct EventVisitor<'a, M: SerializeMap> {
    map: &'a mut M,
    result: Result<(), M::Error>,
}

impl<M: SerializeMap> EventVisitor<'_, M> {
    fn entry<T: data_serialization::Serialize + ?Sized>(&mut self, field: &Field, value: &T) {
        if self.result.is_err() {
            return;
        }
        let name = field.name();
        #[cfg(feature = "log-bridge")]
        if name.starts_with("log.") {
            return;
        }
        self.result = self
            .map
            .serialize_entry(name.strip_prefix("r#").unwrap_or(name), value);
    }
}

struct DebugField<'a>(&'a dyn fmt::Debug);
impl data_serialization::Serialize for DebugField<'_> {
    fn serialize<S: data_serialization::Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format_args!("{:?}", self.0))
    }
}

struct BytesField<'a>(&'a [u8]);
impl data_serialization::Serialize for BytesField<'_> {
    fn serialize<S: data_serialization::Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.0)
    }
}

impl<M: SerializeMap> field::Visit for EventVisitor<'_, M> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.entry(field, &value);
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.entry(field, &value);
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.entry(field, &value);
    }
    fn record_i128(&mut self, field: &Field, value: i128) {
        self.entry(field, &value);
    }
    fn record_u128(&mut self, field: &Field, value: u128) {
        self.entry(field, &value);
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.entry(field, &value);
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.entry(field, value);
    }
    fn record_bytes(&mut self, field: &Field, value: &[u8]) {
        self.entry(field, &BytesField(value));
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.entry(field, &DebugField(value));
    }
}

/// Marker for [`Format`] that indicates that the newline-delimited JSON log
/// format should be used.
///
/// This formatter is intended for production use with systems where structured
/// logs are consumed as JSON by analysis and viewing tools. The JSON output is
/// not optimized for human readability; instead, it should be pretty-printed
/// using external JSON tools such as `jq`, or using a JSON log viewer.
///
/// # Example Output
///
/// <pre><font color="#4E9A06"><b>:;</b></font> <font color="#4E9A06">cargo</font> run --example fmt-json
/// <font color="#4E9A06"><b>    Finished</b></font> dev [unoptimized + debuginfo] target(s) in 0.08s
/// <font color="#4E9A06"><b>     Running</b></font> `target/debug/examples/fmt-json`
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.821315Z&quot;,&quot;level&quot;:&quot;INFO&quot;,&quot;fields&quot;:{&quot;message&quot;:&quot;preparing to shave yaks&quot;,&quot;number_of_yaks&quot;:3},&quot;target&quot;:&quot;fmt_json&quot;}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.821422Z&quot;,&quot;level&quot;:&quot;INFO&quot;,&quot;fields&quot;:{&quot;message&quot;:&quot;shaving yaks&quot;},&quot;target&quot;:&quot;fmt_json::yak_shave&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.821495Z&quot;,&quot;level&quot;:&quot;TRACE&quot;,&quot;fields&quot;:{&quot;message&quot;:&quot;hello! I&apos;m gonna shave a yak&quot;,&quot;excitement&quot;:&quot;yay!&quot;},&quot;target&quot;:&quot;fmt_json::yak_shave&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;},{&quot;yak&quot;:1,&quot;name&quot;:&quot;shave&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.821546Z&quot;,&quot;level&quot;:&quot;TRACE&quot;,&quot;fields&quot;:{&quot;message&quot;:&quot;yak shaved successfully&quot;},&quot;target&quot;:&quot;fmt_json::yak_shave&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;},{&quot;yak&quot;:1,&quot;name&quot;:&quot;shave&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.821598Z&quot;,&quot;level&quot;:&quot;DEBUG&quot;,&quot;fields&quot;:{&quot;yak&quot;:1,&quot;shaved&quot;:true},&quot;target&quot;:&quot;yak_events&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.821637Z&quot;,&quot;level&quot;:&quot;TRACE&quot;,&quot;fields&quot;:{&quot;yaks_shaved&quot;:1},&quot;target&quot;:&quot;fmt_json::yak_shave&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.821684Z&quot;,&quot;level&quot;:&quot;TRACE&quot;,&quot;fields&quot;:{&quot;message&quot;:&quot;hello! I&apos;m gonna shave a yak&quot;,&quot;excitement&quot;:&quot;yay!&quot;},&quot;target&quot;:&quot;fmt_json::yak_shave&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;},{&quot;yak&quot;:2,&quot;name&quot;:&quot;shave&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.821727Z&quot;,&quot;level&quot;:&quot;TRACE&quot;,&quot;fields&quot;:{&quot;message&quot;:&quot;yak shaved successfully&quot;},&quot;target&quot;:&quot;fmt_json::yak_shave&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;},{&quot;yak&quot;:2,&quot;name&quot;:&quot;shave&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.821773Z&quot;,&quot;level&quot;:&quot;DEBUG&quot;,&quot;fields&quot;:{&quot;yak&quot;:2,&quot;shaved&quot;:true},&quot;target&quot;:&quot;yak_events&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.821806Z&quot;,&quot;level&quot;:&quot;TRACE&quot;,&quot;fields&quot;:{&quot;yaks_shaved&quot;:2},&quot;target&quot;:&quot;fmt_json::yak_shave&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.821909Z&quot;,&quot;level&quot;:&quot;TRACE&quot;,&quot;fields&quot;:{&quot;message&quot;:&quot;hello! I&apos;m gonna shave a yak&quot;,&quot;excitement&quot;:&quot;yay!&quot;},&quot;target&quot;:&quot;fmt_json::yak_shave&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;},{&quot;yak&quot;:3,&quot;name&quot;:&quot;shave&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.821956Z&quot;,&quot;level&quot;:&quot;WARN&quot;,&quot;fields&quot;:{&quot;message&quot;:&quot;could not locate yak&quot;},&quot;target&quot;:&quot;fmt_json::yak_shave&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;},{&quot;yak&quot;:3,&quot;name&quot;:&quot;shave&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.822006Z&quot;,&quot;level&quot;:&quot;DEBUG&quot;,&quot;fields&quot;:{&quot;yak&quot;:3,&quot;shaved&quot;:false},&quot;target&quot;:&quot;yak_events&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.822041Z&quot;,&quot;level&quot;:&quot;ERROR&quot;,&quot;fields&quot;:{&quot;message&quot;:&quot;failed to shave yak&quot;,&quot;yak&quot;:3,&quot;error&quot;:&quot;missing yak&quot;},&quot;target&quot;:&quot;fmt_json::yak_shave&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.822079Z&quot;,&quot;level&quot;:&quot;TRACE&quot;,&quot;fields&quot;:{&quot;yaks_shaved&quot;:2},&quot;target&quot;:&quot;fmt_json::yak_shave&quot;,&quot;spans&quot;:[{&quot;yaks&quot;:3,&quot;name&quot;:&quot;shaving_yaks&quot;}]}
/// {&quot;timestamp&quot;:&quot;2022-02-15T18:47:10.822117Z&quot;,&quot;level&quot;:&quot;INFO&quot;,&quot;fields&quot;:{&quot;message&quot;:&quot;yak shaving completed&quot;,&quot;all_yaks_shaved&quot;:false},&quot;target&quot;:&quot;fmt_json&quot;}
/// </pre>
///
/// # Options
///
/// This formatter exposes additional options to configure the structure of the
/// output JSON objects:
///
/// - [`Json::flatten_event`] can be used to enable flattening event fields into
///   the root
/// - [`Json::with_current_span`] can be used to control logging of the current
///   span
/// - [`Json::with_span_list`] can be used to control logging of the span list
///   object.
///
/// By default, event fields are not flattened, and both current span and span
/// list are logged.
///
/// [`Json::flatten_event`]: Json::flatten_event()
/// [`Json::with_current_span`]: Json::with_current_span()
/// [`Json::with_span_list`]: Json::with_span_list()
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Json {
    pub(crate) flatten_event: bool,
    pub(crate) display_current_span: bool,
    pub(crate) display_span_list: bool,
}

impl Json {
    /// If set to `true` event metadata will be flattened into the root object.
    pub fn flatten_event(&mut self, flatten_event: bool) {
        self.flatten_event = flatten_event;
    }

    /// If set to `false`, formatted events won't contain a field for the current span.
    pub fn with_current_span(&mut self, display_current_span: bool) {
        self.display_current_span = display_current_span;
    }

    /// If set to `false`, formatted events won't contain a list of all currently
    /// entered spans. Spans are logged in a list from root to leaf.
    pub fn with_span_list(&mut self, display_span_list: bool) {
        self.display_span_list = display_span_list;
    }
}

struct SerializableContext<'a, 'b, Span, N>(
    &'b crate::collector::layer::Context<'a, Span>,
    std::marker::PhantomData<N>,
)
where
    Span: Subscriber + for<'lookup> crate::collector::registry::LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static;

impl<Span, N> data_serialization::ser::Serialize for SerializableContext<'_, '_, Span, N>
where
    Span: Subscriber + for<'lookup> crate::collector::registry::LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn serialize<Ser>(&self, serializer_o: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: data_serialization::ser::Serializer,
    {
        use data_serialization::ser::SerializeSeq;
        let mut serializer = serializer_o.serialize_seq(None)?;

        if let Some(leaf_span) = self.0.lookup_current() {
            for span in leaf_span.scope().from_root() {
                serializer.serialize_element(&SerializableSpan(&span, self.1))?;
            }
        }

        serializer.end()
    }
}

struct SerializableSpan<'a, 'b, Span, N>(
    &'b crate::collector::registry::SpanRef<'a, Span>,
    std::marker::PhantomData<N>,
)
where
    Span: for<'lookup> crate::collector::registry::LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static;

impl<Span, N> data_serialization::ser::Serialize for SerializableSpan<'_, '_, Span, N>
where
    Span: for<'lookup> crate::collector::registry::LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: data_serialization::ser::Serializer,
    {
        let mut serializer = serializer.serialize_map(None)?;

        let ext = self.0.extensions();
        let data = ext
            .get::<FormattedFields<N>>()
            .expect("Unable to find FormattedFields in extensions; this is a bug");

        // TODO: let's _not_ do this, but this resolves
        // https://github.com/tokio-rs/tracing/issues/391.
        // We should probably rework this to use a `json_codec::Value` or something
        // similar in a JSON-specific layer, but I'd (david)
        // rather have a uglier fix now rather than shipping broken JSON.
        match json_codec::from_str::<json_codec::Value>(data) {
            Ok(json_codec::Value::Object(fields)) => {
                for field in fields {
                    serializer.serialize_entry(&field.0, &field.1)?;
                }
            }
            // We have fields for this span which are valid JSON but not an object.
            // This is probably a bug, so panic if we're in debug mode
            Ok(_) if cfg!(debug_assertions) => panic!(
                "span '{}' had malformed fields! this is a bug.\n  error: invalid JSON object\n  fields: {:?}",
                self.0.metadata().name(),
                data
            ),
            // If we *aren't* in debug mode, it's probably best not to
            // crash the program, let's log the field found but also an
            // message saying it's type  is invalid
            Ok(value) => {
                serializer.serialize_entry("field", &value)?;
                serializer.serialize_entry("field_error", "field was no a valid object")?
            }
            // We have previously recorded fields for this span
            // should be valid JSON. However, they appear to *not*
            // be valid JSON. This is almost certainly a bug, so
            // panic if we're in debug mode
            Err(e) if cfg!(debug_assertions) => panic!(
                "span '{}' had malformed fields! this is a bug.\n  error: {}\n  fields: {:?}",
                self.0.metadata().name(),
                e,
                data
            ),
            // If we *aren't* in debug mode, it's probably best not
            // crash the program, but let's at least make sure it's clear
            // that the fields are not supposed to be missing.
            Err(e) => serializer.serialize_entry("field_error", &format!("{}", e))?,
        };
        serializer.serialize_entry("name", self.0.metadata().name())?;
        serializer.end()
    }
}

impl<S, N, T> FormatEvent<S, N> for Format<Json, T>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let mut timestamp = String::new();
        self.timer.format_time(&mut Writer::new(&mut timestamp))?;

        #[cfg(feature = "log-bridge")]
        let normalized_meta = event.normalized_metadata();
        #[cfg(feature = "log-bridge")]
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());
        #[cfg(not(feature = "log-bridge"))]
        let meta = event.metadata();

        let mut visit = || {
            let mut serializer = Serializer::new(WriteAdaptor::new(&mut writer));

            let mut serializer = serializer.serialize_map(None)?;

            if self.display_timestamp {
                serializer.serialize_entry("timestamp", &timestamp)?;
            }

            if self.display_level {
                serializer.serialize_entry("level", &meta.level().as_str())?;
            }

            let format_field_marker: std::marker::PhantomData<N> = std::marker::PhantomData;

            let current_span = if self.format.display_current_span || self.format.display_span_list
            {
                event
                    .parent()
                    .and_then(|id| ctx.span(id))
                    .or_else(|| ctx.lookup_current())
            } else {
                None
            };

            if self.format.flatten_event {
                record_event(event, &mut serializer)?;
            } else {
                serializer.serialize_entry("fields", &EventFields(event))?;
            }

            if self.display_target {
                serializer.serialize_entry("target", meta.target())?;
            }

            if self.display_filename {
                if let Some(filename) = meta.file() {
                    serializer.serialize_entry("filename", filename)?;
                }
            }

            if self.display_line_number {
                if let Some(line_number) = meta.line() {
                    serializer.serialize_entry("line_number", &line_number)?;
                }
            }

            if self.format.display_current_span {
                if let Some(ref span) = current_span {
                    serializer
                        .serialize_entry("span", &SerializableSpan(span, format_field_marker))
                        .unwrap_or(());
                }
            }

            if self.format.display_span_list && current_span.is_some() {
                serializer.serialize_entry(
                    "spans",
                    &SerializableContext(&ctx.ctx, format_field_marker),
                )?;
            }

            if self.display_thread_name {
                let current_thread = std::thread::current();
                match current_thread.name() {
                    Some(name) => {
                        serializer.serialize_entry("threadName", name)?;
                    }
                    // fall-back to thread id when name is absent and ids are not enabled
                    None if !self.display_thread_id => {
                        serializer
                            .serialize_entry("threadName", &format!("{:?}", current_thread.id()))?;
                    }
                    _ => {}
                }
            }

            if self.display_thread_id {
                serializer
                    .serialize_entry("threadId", &format!("{:?}", std::thread::current().id()))?;
            }

            serializer.end()
        };

        visit().map_err(|_| fmt::Error)?;
        writeln!(writer)
    }
}

impl Default for Json {
    fn default() -> Json {
        Json {
            flatten_event: false,
            display_current_span: true,
            display_span_list: true,
        }
    }
}

/// The JSON [`FormatFields`] implementation.
///
#[derive(Debug)]
pub struct JsonFields {
    // reserve the ability to add fields to this without causing a breaking
    // change in the future.
    _private: (),
}

impl JsonFields {
    /// Returns a new JSON [`FormatFields`] implementation.
    ///
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for JsonFields {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> FormatFields<'a> for JsonFields {
    /// Format the provided `fields` to the provided `writer`, returning a result.
    fn format_fields<R: RecordFields>(&self, mut writer: Writer<'_>, fields: R) -> fmt::Result {
        let mut v = JsonVisitor::new(&mut writer);
        fields.record(&mut v);
        v.finish()
    }

    /// Record additional field(s) on an existing span.
    ///
    /// By default, this appends a space to the current set of fields if it is
    /// non-empty, and then calls `self.format_fields`. If different behavior is
    /// required, the default implementation of this method can be overridden.
    fn add_fields(
        &self,
        current: &'a mut FormattedFields<Self>,
        fields: &Record<'_>,
    ) -> fmt::Result {
        if current.is_empty() {
            // If there are no previously recorded fields, we can just reuse the
            // existing string.
            let mut writer = current.as_writer();
            let mut v = JsonVisitor::new(&mut writer);
            fields.record(&mut v);
            v.finish()?;
            return Ok(());
        }

        // If fields were previously recorded on this span, we need to parse
        // the current set of fields as JSON, add the new fields, and
        // re-serialize them. Otherwise, if we just appended the new fields
        // to a previously serialized JSON object, we would end up with
        // malformed JSON.
        //
        // XXX(eliza): this is far from efficient, but unfortunately, it is
        // necessary as long as the JSON formatter is implemented on top of
        // an interface that stores all formatted fields as strings.
        //
        // We should consider reimplementing the JSON formatter as a
        // separate layer, rather than a formatter for the `fmt` layer —
        // then, we could store fields as JSON values, and add to them
        // without having to parse and re-serialize.
        let mut new = String::new();
        let map: BTreeMap<&'_ str, json_codec::Value> =
            json_codec::from_str(current).map_err(|_| fmt::Error)?;
        let mut v = JsonVisitor::new(&mut new);
        v.values = map;
        fields.record(&mut v);
        v.finish()?;
        current.fields = new;

        Ok(())
    }
}

/// The [visitor] produced by [`JsonFields`]'s [`MakeVisitor`] implementation.
///
/// [visitor]: crate::collector::field::Visit
/// [`MakeVisitor`]: crate::collector::field::MakeVisitor
pub struct JsonVisitor<'a> {
    values: BTreeMap<&'a str, json_codec::Value>,
    writer: &'a mut dyn Write,
}

impl fmt::Debug for JsonVisitor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("JsonVisitor {{ values: {:?} }}", self.values))
    }
}

impl<'a> JsonVisitor<'a> {
    /// Returns a new default visitor that formats to the provided `writer`.
    ///
    /// # Arguments
    /// - `writer`: the writer to format to.
    /// - `is_empty`: whether or not any fields have been previously written to
    ///   that writer.
    pub fn new(writer: &'a mut dyn Write) -> Self {
        Self {
            values: BTreeMap::new(),
            writer,
        }
    }
}

impl crate::collector::field::VisitFmt for JsonVisitor<'_> {
    fn writer(&mut self) -> &mut dyn fmt::Write {
        self.writer
    }
}

impl crate::collector::field::VisitOutput<fmt::Result> for JsonVisitor<'_> {
    fn finish(self) -> fmt::Result {
        let inner = || {
            let mut serializer = Serializer::new(WriteAdaptor::new(self.writer));
            let mut ser_map = serializer.serialize_map(None)?;

            for (k, v) in self.values {
                ser_map.serialize_entry(k, &v)?;
            }

            ser_map.end()
        };

        if inner().is_err() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

impl field::Visit for JsonVisitor<'_> {
    /// Visit a double precision floating point value.
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.values
            .insert(field.name(), json_codec::Value::from(value));
    }

    /// Visit a signed 64-bit integer value.
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.values
            .insert(field.name(), json_codec::Value::from(value));
    }

    /// Visit an unsigned 64-bit integer value.
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.values
            .insert(field.name(), json_codec::Value::from(value));
    }

    /// Visit a boolean value.
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.values
            .insert(field.name(), json_codec::Value::from(value));
    }

    /// Visit a string value.
    fn record_str(&mut self, field: &Field, value: &str) {
        self.values
            .insert(field.name(), json_codec::Value::from(value));
    }

    fn record_bytes(&mut self, field: &Field, value: &[u8]) {
        self.values
            .insert(field.name(), json_codec::Value::from(value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        match field.name() {
            // Skip fields that are actually log metadata that have already been handled
            #[cfg(feature = "log-bridge")]
            name if name.starts_with("log.") => (),
            name if name.starts_with("r#") => {
                self.values
                    .insert(&name[2..], json_codec::Value::from(format!("{:?}", value)));
            }
            name => {
                self.values
                    .insert(name, json_codec::Value::from(format!("{:?}", value)));
            }
        };
    }
}
#[cfg(test)]
include!(core::concat!(
    core::env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/tracing-family/collector/unit/src/fmt/format/json/test_533.rs"
));

#[cfg(test)]
include!(core::concat!(
    core::env!("CARGO_MANIFEST_DIR"),
    "/../../../test/tracing/stream_map.rs"
));
