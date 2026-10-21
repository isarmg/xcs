#[cfg(all(test))]
mod test {
    use super::*;

    #[cfg(all(target_family = "wasm", not(target_os = "wasi")))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    fn get() -> Budget {
        context::budget(|cell| cell.get()).unwrap_or(Budget::unconstrained())
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/task/coop/mod/budgeting_506.rs"));
}
