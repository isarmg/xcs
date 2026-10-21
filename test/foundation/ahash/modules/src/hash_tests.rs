use crate::RandomState;

/// Tests that some non-trivial transformation takes place.
#[test]
pub fn test_input_processed() {
    let build_hasher = RandomState::with_seeds(2, 2, 2, 2);
    assert_ne!(0, build_hasher.hash_one(0_u64));
    assert_ne!(1, build_hasher.hash_one(0_u64));
    assert_ne!(2, build_hasher.hash_one(0_u64));
    assert_ne!(3, build_hasher.hash_one(0_u64));
    assert_ne!(4, build_hasher.hash_one(0_u64));
    assert_ne!(5, build_hasher.hash_one(0_u64));

    assert_ne!(0, build_hasher.hash_one(1_u64));
    assert_ne!(1, build_hasher.hash_one(1_u64));
    assert_ne!(2, build_hasher.hash_one(1_u64));
    assert_ne!(3, build_hasher.hash_one(1_u64));
    assert_ne!(4, build_hasher.hash_one(1_u64));
    assert_ne!(5, build_hasher.hash_one(1_u64));

    let xored = build_hasher.hash_one(0_u64) ^ build_hasher.hash_one(1_u64);
    assert_ne!(0, xored);
    assert_ne!(1, xored);
    assert_ne!(2, xored);
    assert_ne!(3, xored);
    assert_ne!(4, xored);
    assert_ne!(5, xored);
}

#[test]
pub fn test_ref_independent() {
    let build_hasher = RandomState::with_seeds(1, 2, 3, 4);
    assert_eq!(build_hasher.hash_one(&&1_u8), build_hasher.hash_one(&1_u8));
    assert_eq!(
        build_hasher.hash_one(&&2_u16),
        build_hasher.hash_one(&2_u16)
    );
    assert_eq!(
        build_hasher.hash_one(&&3_u32),
        build_hasher.hash_one(&3_u32)
    );
    assert_eq!(
        build_hasher.hash_one(&&4_u64),
        build_hasher.hash_one(&4_u64)
    );
    assert_eq!(
        build_hasher.hash_one(&&5_u128),
        build_hasher.hash_one(&5_u128)
    );
    assert_eq!(
        build_hasher.hash_one(&"test"),
        build_hasher.hash_one("test")
    );
    assert_eq!(
        build_hasher.hash_one(&"test"),
        build_hasher.hash_one(&"test".to_string())
    );

    let build_hasher = RandomState::with_seeds(10, 20, 30, 40);
    assert_eq!(build_hasher.hash_one(&&&1_u8), build_hasher.hash_one(&1_u8));
    assert_eq!(
        build_hasher.hash_one(&&&2_u16),
        build_hasher.hash_one(&2_u16)
    );
    assert_eq!(
        build_hasher.hash_one(&&&3_u32),
        build_hasher.hash_one(&3_u32)
    );
    assert_eq!(
        build_hasher.hash_one(&&&4_u64),
        build_hasher.hash_one(&4_u64)
    );
    assert_eq!(
        build_hasher.hash_one(&&&5_u128),
        build_hasher.hash_one(&5_u128)
    );
    assert_eq!(
        build_hasher.hash_one(&&"test"),
        build_hasher.hash_one("test")
    );
    assert_eq!(
        build_hasher.hash_one(&&"test"),
        build_hasher.hash_one(&"test".to_string())
    );
}
