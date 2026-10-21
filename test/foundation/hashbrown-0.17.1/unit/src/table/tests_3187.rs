#[cfg(test)]
mod tests {
    use super::HashTable;

    #[test]
    fn test_allocation_info() {
        assert_eq!(HashTable::<()>::new().allocation_size(), 0);
        assert_eq!(HashTable::<u32>::new().allocation_size(), 0);
        assert!(HashTable::<u32>::with_capacity(1).allocation_size() > core::mem::size_of::<u32>());
    }
}
