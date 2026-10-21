use futures::channel::oneshot::{self, Canceled};
use futures::executor::block_on;
use futures::future;
use std::sync::mpsc::{channel, TryRecvError};

// mod support;
// use support::*;

fn unselect<T, E, A, B>(r: Result<Either<(T, B), (T, A)>, Either<(E, B), (E, A)>>) -> Result<T, E> {
    match r {
        Ok(Either::Left((t, _))) | Ok(Either::Right((t, _))) => Ok(t),
        Err(Either::Left((e, _))) | Err(Either::Right((e, _))) => Err(e),
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/futures/functions/tests_disabled/all/result_smoke_16.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/futures/functions/tests_disabled/all/test_empty_65.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/futures/functions/tests_disabled/all/test_ok_85.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/futures/functions/tests_disabled/all/flatten_91.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/futures/functions/tests_disabled/all/smoke_oneshot_108.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/futures/functions/tests_disabled/all/select_cancels_144.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/futures/functions/tests_disabled/all/join_cancels_190.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/futures/functions/tests_disabled/all/join_incomplete_236.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/futures/functions/tests_disabled/all/select2_277.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/futures/functions/tests_disabled/all/option_396.rs"));
