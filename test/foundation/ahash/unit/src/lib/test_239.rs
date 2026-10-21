#[cfg(test)]
mod test {
    use crate::convert::Convert;
    use crate::*;
    use core::hash::Hash;
    use core::hash::Hasher;
    use std::collections::HashMap;

    #[test]
    fn test_ahash_alias_map_construction() {
        let mut map = super::HashMap::with_capacity(1234);
        map.insert(1, "test");
    }

    #[test]
    fn test_ahash_alias_set_construction() {
        let mut set = super::HashSet::with_capacity(1234);
        set.insert(1);
    }

    #[test]
    fn test_default_builder() {
        use core::hash::BuildHasherDefault;

        let mut map = HashMap::<u32, u64, BuildHasherDefault<AHasher>>::default();
        map.insert(1, 3);
    }

    #[test]
    fn test_builder() {
        let mut map = HashMap::<u32, u64, RandomState>::default();
        map.insert(1, 3);
    }

    #[test]
    fn test_conversion() {
        let input: &[u8] = b"dddddddd";
        let bytes: u64 = as_array!(input, 8).convert();
        assert_eq!(bytes, 0x6464646464646464);
    }

    #[test]
    fn test_non_zero() {
        let mut hasher1 = AHasher::new_with_keys(0, 0);
        let mut hasher2 = AHasher::new_with_keys(0, 0);
        "foo".hash(&mut hasher1);
        "bar".hash(&mut hasher2);
        assert_ne!(hasher1.finish(), 0);
        assert_ne!(hasher2.finish(), 0);
        assert_ne!(hasher1.finish(), hasher2.finish());

        let mut hasher1 = AHasher::new_with_keys(0, 0);
        let mut hasher2 = AHasher::new_with_keys(0, 0);
        3_u64.hash(&mut hasher1);
        4_u64.hash(&mut hasher2);
        assert_ne!(hasher1.finish(), 0);
        assert_ne!(hasher2.finish(), 0);
        assert_ne!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_non_zero_hash_one() {
        let hasher_build = RandomState::with_seeds(0, 0, 0, 0);

        let h1 = hasher_build.hash_one("foo");
        let h2 = hasher_build.hash_one("bar");
        assert_ne!(h1, 0);
        assert_ne!(h2, 0);
        assert_ne!(h1, h2);

        let h1 = hasher_build.hash_one(3_u64);
        let h2 = hasher_build.hash_one(4_u64);
        assert_ne!(h1, 0);
        assert_ne!(h2, 0);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_ahasher_construction() {
        let _ = AHasher::new_with_keys(1234, 5678);
    }

    #[test]
    fn test_hash_one_references() {
        let hasher_build = RandomState::with_seeds(0, 0, 0, 0);
        let h1 = hasher_build.hash_one(1u64);
        let h2 = hasher_build.hash_one(&1u64);

        assert_eq!(h1, h2);

        let h1 = hasher_build.hash_one(1_u64);
        let h2 = hasher_build.hash_one(&1_u64);

        assert_eq!(h1, h2);

        let h1 = hasher_build.hash_one(1u128);
        let h2 = hasher_build.hash_one(&1u128);

        assert_eq!(h1, h2);
    }
}
