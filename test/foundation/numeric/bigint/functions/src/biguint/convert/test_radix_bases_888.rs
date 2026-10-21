#[test]
fn test_radix_bases() {
    for radix in 3u32..256 {
        if !radix.is_power_of_two() {
            let (base, power) = get_radix_base(radix);
            let radix = BigDigit::from(radix);
            let power = u32::try_from(power).unwrap();
            assert_eq!(base, radix.pow(power));
            assert!(radix.checked_pow(power + 1).is_none());
        }
    }
}
