#[cfg(all(test))]
mod test {
    use super::*;

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/runtime/time/wheel/level/test_slot_for_180.rs"));
}
