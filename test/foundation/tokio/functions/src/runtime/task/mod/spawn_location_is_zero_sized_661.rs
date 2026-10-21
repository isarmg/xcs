#[cfg(test)]
    #[test]
    fn spawn_location_is_zero_sized() {
        assert_eq!(std::mem::size_of::<SpawnLocation>(), 0);
    }
