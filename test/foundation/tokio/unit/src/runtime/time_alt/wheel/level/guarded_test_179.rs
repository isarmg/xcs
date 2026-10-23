#[cfg(all(test))]
mod test {
    use super::*;

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/functions/src/runtime/time_alt/wheel/level/test_slot_for_183.rs"));
}
