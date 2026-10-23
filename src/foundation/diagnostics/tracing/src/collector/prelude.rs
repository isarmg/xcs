//! The `tracing-subscriber` prelude.
//!
//! This brings into scope a number of extension traits that define methods on
//! types defined here and in other crates.

pub use crate::collector::field::{
    MakeExt as __tracing_subscriber_field_MakeExt,
    RecordFields as __tracing_subscriber_field_RecordFields,
};
pub use crate::collector::layer::{
    Layer as __tracing_subscriber_Layer, SubscriberExt as __tracing_subscriber_SubscriberExt,
};

pub use crate::collector::util::SubscriberInitExt as _;

feature! {
    #![all(feature = "fmt", feature = "std")]
    pub use crate::collector::fmt::writer::MakeWriterExt as _;
}
