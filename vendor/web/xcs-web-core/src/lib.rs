#![deny(missing_docs)]

//! Browser CSR runtime used by the public `xcs-web` crate.
//! Use `xcs_web::prelude::*` and the DOM builders from that facade.
//! See README.md for the original third-party copyright and license notice.
//! This product build contains only CSR; server rendering, hydration and server functions are excluded.

#![cfg_attr(all(feature = "nightly", rustc_nightly), feature(fn_traits))]
#![cfg_attr(all(feature = "nightly", rustc_nightly), feature(unboxed_closures))]

extern crate self as xcs_web;

/// Exports all the core types of the library.
pub mod prelude {
    // Traits
    // These should always be exported from the prelude
    pub use reactive_graph::prelude::*;
    pub use tachys::prelude::*;

    // Collect the public CSR types for the facade's prelude.
    mod export_types {
        pub use crate::{
            callback::*, children::*, component::*, control_flow::*, error::*, into_view::*,
            mount::*, text_prop::*,
        };
        pub use leptos_dom::helpers::*;
        pub use oco_ref::*;
        pub use reactive_graph::{
            actions::*,
            computed::*,
            effect::*,
            graph::untrack,
            owner::*,
            signal::*,
            wrappers::{read::*, write::*},
        };
        pub use tachys::{
            reactive_graph::{bind::BindAttribute, node_ref::*, Suspend},
            view::{fragment::Fragment, template::ViewTemplate},
        };
        pub use xcs_web_macro::*;
    }
    pub use export_types::*;
}

/// A standard way to wrap functions and closures to pass them to components.
pub use reactive_graph::callback;

/// Types that can be passed as the `children` prop of a component.
pub mod children;

/// Wrapper for intercepting component attributes.
pub mod attribute_interceptor;

#[doc(hidden)]
/// Traits used to implement component constructors.
pub mod component;

/// Tools for handling errors.
pub mod error {
    pub use throw_error::*;
}

/// CSR control-flow components such as `<Show>`, `<For>`, and `<ShowLet>`.
pub mod control_flow {
    pub use crate::{animated_show::*, for_loop::*, show::*, show_let::*};
}
mod animated_show;

mod for_loop;
mod show;
mod show_let;

/// A component that allows rendering a component somewhere else.
pub mod portal;

/// Types for reactive string properties for components.
pub mod text_prop;

#[doc(hidden)]
pub use typed_builder;
#[doc(hidden)]
pub use typed_builder_macro;
pub use xcs_web_macro::*;
mod into_view;
pub use into_view::IntoView;
#[doc(inline)]
pub use leptos_dom;
mod provider;
#[doc(inline)]
pub use tachys;
/// Mount a browser CSR tree into the DOM and retain its reactive owner.
pub mod mount;
#[doc(inline)]
pub use oco_ref as oco;
mod from_form_data;
#[doc(inline)]
pub use either_of as either;
#[doc(inline)]
pub use reactive_graph as reactive;

/// Provide and access data along the reactive graph, sharing data without directly passing arguments.
pub mod context {
    pub use crate::provider::*;
    pub use reactive_graph::owner::{provide_context, use_context};
}

/// HTML attribute types.
#[doc(inline)]
pub use tachys::html::attribute as attr;
/// HTML element types.
#[doc(inline)]
pub use tachys::html::element as html;
/// HTML event types.
#[doc(no_inline)]
pub use tachys::html::event as ev;
/// MathML element types.
#[doc(inline)]
pub use tachys::mathml as math;
/// SVG element types.
#[doc(inline)]
pub use tachys::svg;

/// Utilities for simple isomorphic logging to the console or terminal.
pub mod logging {
    pub use leptos_dom::{debug_error, debug_log, debug_warn, error, log, warn};
}

/// Utilities for working with asynchronous tasks.
pub mod task {
    use any_spawner::Executor;
    use reactive_graph::computed::ScopedFuture;
    use std::future::Future;

    /// Spawns a thread-safe [`Future`].
    ///
    /// This will be run with the current reactive owner and observer using a [`ScopedFuture`].
    #[track_caller]
    #[inline(always)]
    pub fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
        let fut = ScopedFuture::new(fut);

        #[cfg(not(target_family = "wasm"))]
        Executor::spawn(fut);

        #[cfg(target_family = "wasm")]
        Executor::spawn_local(fut);
    }

    /// Spawns a [`Future`] that cannot be sent across threads.
    #[track_caller]
    #[inline(always)]
    pub fn spawn_local(fut: impl Future<Output = ()> + 'static) {
        Executor::spawn_local(fut)
    }

    /// Waits until the next "tick" of the current async executor.
    pub async fn tick() {
        Executor::tick().await
    }

    pub use reactive_graph::{spawn_local_scoped, spawn_local_scoped_with_cancellation};
}

// these reexports are used in islands
#[cfg(feature = "islands")]
#[doc(hidden)]
pub use serde;
#[doc(hidden)]
pub use serde_json;
#[cfg(feature = "tracing")]
#[doc(hidden)]
pub use tracing;
#[doc(hidden)]
pub use wasm_bindgen;
#[doc(hidden)]
pub use web_sys;

#[doc(hidden)]
pub mod __reexports {
    pub use send_wrapper;
    pub use wasm_bindgen_futures;
}
