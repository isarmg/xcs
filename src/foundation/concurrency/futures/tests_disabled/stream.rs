use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::executor::{block_on, block_on_stream};
use futures::future::{err, ok};
use futures::stream::{empty, iter_ok, poll_fn, Peekable};

// mod support;
// use support::*;

pub struct Iter<I> {
    iter: I,
}

pub fn iter<J, T, E>(i: J) -> Iter<J::IntoIter>
where
    J: IntoIterator<Item = Result<T, E>>,
{
    Iter { iter: i.into_iter() }
}

impl<I, T, E> Stream for Iter<I>
where
    I: Iterator<Item = Result<T, E>>,
{
    type Item = T;
    type Error = E;

    fn poll_next(&mut self, _: &mut Context<'_>) -> Poll<Option<T>, E> {
        match self.iter.next() {
            Some(Ok(e)) => Ok(Poll::Ready(Some(e))),
            Some(Err(e)) => Err(e),
            None => Ok(Poll::Ready(None)),
        }
    }
}

fn list() -> Box<Stream<Item = i32, Error = u32> + Send> {
    let (tx, rx) = mpsc::channel(1);
    tx.send(Ok(1)).and_then(|tx| tx.send(Ok(2))).and_then(|tx| tx.send(Ok(3))).forget();
    Box::new(rx.then(|r| r.unwrap()))
}

fn err_list() -> Box<Stream<Item = i32, Error = u32> + Send> {
    let (tx, rx) = mpsc::channel(1);
    tx.send(Ok(1)).and_then(|tx| tx.send(Ok(2))).and_then(|tx| tx.send(Err(3))).forget();
    Box::new(rx.then(|r| r.unwrap()))
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/map_49.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/map_err_54.rs"));

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct FromErrTest(u32);

impl From<u32> for FromErrTest {
    fn from(i: u32) -> Self {
        Self(i)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/from_err_68.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/fold_73.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/filter_79.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/filter_map_84.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/and_then_92.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/then_98.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/or_else_103.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/flatten_108.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/skip_113.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/skip_passes_errors_through_118.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/skip_while_128.rs"));
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/take_132.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/take_while_137.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/take_passes_errors_through_142.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/peekable_155.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/fuse_160.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/buffered_171.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/unordered_210.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/zip_249.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/peek_258.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/wait_282.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/chunks_287.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/chunks_panic_on_cap_zero_299.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/forward_305.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/concat_320.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/concat2_329.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/stream_poll_fn_341.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/inspect_356.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/futures/functions/tests_disabled/stream/inspect_err_363.rs"));
