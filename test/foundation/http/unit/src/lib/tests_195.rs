#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn request_satisfies_send_sync() {
        assert_send_sync::<Request<()>>();
    }

    #[test]
    fn response_satisfies_send_sync() {
        assert_send_sync::<Response<()>>();
    }
}
