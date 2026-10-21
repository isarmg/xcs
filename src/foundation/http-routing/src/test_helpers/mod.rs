#![allow(clippy::disallowed_names)]

use crate::{extract::Request, response::Response, serve};

mod test_client;
pub use self::test_client::*;

#[cfg(test)]
#[path = "../../../../../test/foundation/http-routing/modules/src/test_helpers/tracing_helpers.rs"]
pub(crate) mod tracing_helpers;

#[cfg(test)]
#[path = "../../../../../test/foundation/http-routing/modules/src/test_helpers/counting_cloneable_state.rs"]
pub(crate) mod counting_cloneable_state;

#[cfg(test)]
pub(crate) fn assert_send<T: Send>() {}
#[cfg(test)]
pub(crate) fn assert_sync<T: Sync>() {}

#[allow(dead_code)]
pub(crate) struct NotSendSync(*const ());
