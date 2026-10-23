//! Extra utilities for spawning tasks
//!
//! This module is only available when the `rt` feature is enabled. Note that enabling the
//! `join-map` feature will automatically also enable the `rt` feature.

cfg_adapters_rt! {
    mod spawn_pinned;
    pub use spawn_pinned::LocalPoolHandle;

    pub mod task_tracker;
    #[doc(inline)]
    pub use task_tracker::TaskTracker;

    mod abort_on_drop;
    pub use abort_on_drop::{AbortOnDrop, AbortOnDropHandle};

    mod join_queue;
    pub use join_queue::JoinQueue;
}

#[cfg(feature = "adapters-join-map")]
mod join_map;
#[cfg(feature = "adapters-join-map")]
#[cfg_attr(docsrs, doc(cfg(feature = "adapters-join-map")))]
pub use join_map::{JoinMap, JoinMapKeys};
