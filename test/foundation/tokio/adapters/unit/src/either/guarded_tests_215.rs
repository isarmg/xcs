#[cfg(all(test))]
mod tests {
    use super::*;
    use tokio::io::{repeat, AsyncReadExt, Repeat};
    use tokio_stream::{once, Once, StreamExt};

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/adapters/functions/src/either/either_is_stream_221.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tokio/adapters/functions/src/either/either_is_async_read_228.rs"));
}
