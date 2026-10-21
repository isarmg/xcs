#[test]
fn named_group_ffdhe_group_roundtrip() {
    use NamedGroup::*;
    let ffdhe_groups = [FFDHE2048, FFDHE3072, FFDHE4096, FFDHE6144, FFDHE8192];
    for g in ffdhe_groups {
        #[allow(deprecated)]
        let roundtrip = FfdheGroup::from_named_group(g)
            .unwrap()
            .named_group();
        assert_eq!(roundtrip, Some(g));
    }
}
