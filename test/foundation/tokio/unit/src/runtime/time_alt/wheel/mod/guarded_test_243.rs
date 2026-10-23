#[cfg(all(test))]
mod test {
    use super::*;

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/functions/src/runtime/time_alt/wheel/mod/test_level_for_247.rs"));
}
