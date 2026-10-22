#[cfg(all(test))]
mod test {
    use super::*;

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio-util/functions/src/time/wheel/mod/test_level_for_273.rs"));
}
