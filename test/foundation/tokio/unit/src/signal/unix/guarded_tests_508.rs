#[cfg(all(test))]
mod tests {
    use super::*;

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/signal/unix/signal_enable_error_on_invalid_input_512.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/signal/unix/signal_enable_error_on_forbidden_input_526.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/signal/unix/from_c_int_540.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/signal/unix/into_c_int_545.rs"));
}
