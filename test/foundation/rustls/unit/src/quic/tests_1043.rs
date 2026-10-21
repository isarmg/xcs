#[cfg(test)]
mod tests {
    use std::prelude::v1::*;

    use super::PacketKey;
    use crate::quic::HeaderProtectionKey;

    #[test]
    fn auto_traits() {
        fn assert_auto<T: Send + Sync>() {}
        assert_auto::<Box<dyn PacketKey>>();
        assert_auto::<Box<dyn HeaderProtectionKey>>();
    }
}
