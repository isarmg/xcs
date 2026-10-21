#[cfg(test)]
mod tests {
    use super::TryLock;

    #[test]
    fn fmt_debug() {
        let lock = TryLock::new(5);
        assert_eq!(format!("{:?}", lock), "TryLock { value: 5 }");

        let locked = lock.try_lock().unwrap();
        assert_eq!(format!("{:?}", locked), "5");

        assert_eq!(format!("{:?}", lock), "TryLock { value: <locked> }");
    }
}
