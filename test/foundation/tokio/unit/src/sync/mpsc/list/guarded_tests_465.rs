#[cfg(all(test))]
mod tests {
    use crate::sync::mpsc::unbounded::unbounded_channel_from_index;
    use crate::sync::mpsc::BLOCK_CAP;

    #[cfg(all(target_family = "wasm", not(target_os = "wasi")))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/sync/mpsc/list/wraparound_473.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/sync/mpsc/list/wraparound_unbounded_525.rs"));
}
