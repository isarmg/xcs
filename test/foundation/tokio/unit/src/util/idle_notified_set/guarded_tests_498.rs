#[cfg(all(test))]
mod tests {
    use crate::runtime::Builder;
    use crate::task::JoinSet;

    // A test that runs under miri.
    //
    // https://github.com/tokio-rs/tokio/pull/5693
    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/functions/src/util/idle_notified_set/join_set_test_506.rs"));
}
