use crate::collector::fmt::{format::Writer, time::FormatTime, writer::WriteAdaptor};
use date_time::{OffsetDateTime, format_description::well_known, formatting::Formattable};
use std::fmt;
#[cfg_attr(docsrs, doc(cfg(feature = "time")))]
#[derive(Clone, Debug)]
/// Formats the current UTC instant using the date-time capability.
pub struct UtcTime<F> {
    format: F,
}
#[derive(Clone, Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "time")))]
/// Formats the current instant at an explicitly supplied fixed UTC offset.
/// This does not detect a system timezone or adjust for daylight saving changes.
pub struct OffsetTime<F> {
    offset: date_time::UtcOffset,
    format: F,
}

// === impl UtcTime ===

impl UtcTime<well_known::Rfc3339> {
    /// Uses the RFC 3339 timestamp format.
    pub fn rfc_3339() -> Self {
        Self::new(well_known::Rfc3339)
    }
}

impl<F: Formattable> UtcTime<F> {
    /// Uses a well-known format or a parsed format description.
    pub fn new(format: F) -> Self {
        Self { format }
    }
}

impl<F> FormatTime for UtcTime<F>
where
    F: Formattable,
{
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        format_datetime(OffsetDateTime::now_utc(), w, &self.format)
    }
}

impl<F> Default for UtcTime<F>
where
    F: Formattable + Default,
{
    fn default() -> Self {
        Self::new(F::default())
    }
}

// === impl OffsetTime ===

impl<F: date_time::formatting::Formattable> OffsetTime<F> {
    /// Uses the supplied fixed offset and date/time format.
    pub fn new(offset: date_time::UtcOffset, format: F) -> Self {
        Self { offset, format }
    }
}

impl<F> FormatTime for OffsetTime<F>
where
    F: date_time::formatting::Formattable,
{
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        let now = OffsetDateTime::now_utc().to_offset(self.offset);
        format_datetime(now, w, &self.format)
    }
}

fn format_datetime(
    now: OffsetDateTime,
    into: &mut Writer<'_>,
    fmt: &impl Formattable,
) -> fmt::Result {
    let mut into = WriteAdaptor::new(into);
    now.format_into(&mut into, fmt)
        .map_err(|_| fmt::Error)
        .map(|_| ())
}
