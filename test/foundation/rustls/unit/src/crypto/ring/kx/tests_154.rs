#[cfg(test)]
mod tests {
    use std::format;

    #[test]
    fn kxgroup_fmt_yields_name() {
        assert_eq!("X25519", format!("{:?}", super::X25519));
    }
}
