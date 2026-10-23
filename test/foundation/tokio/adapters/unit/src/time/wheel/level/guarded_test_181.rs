#[cfg(all(test))]
mod test {
    use super::*;

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/adapters/functions/src/time/wheel/level/test_slot_for_185.rs"));
}
