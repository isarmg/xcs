#[cfg(test)]
mod tests {
    use super::SeqLock;

    #[test]
    fn test_abort() {
        static LK: SeqLock = SeqLock::new();
        let before = LK.optimistic_read().unwrap();
        {
            let guard = LK.write();
            guard.abort();
        }
        let after = LK.optimistic_read().unwrap();
        assert_eq!(before, after, "aborted write does not update the stamp");
    }
}
