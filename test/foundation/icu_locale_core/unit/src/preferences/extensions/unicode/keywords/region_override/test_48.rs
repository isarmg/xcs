#[cfg(test)]
mod test {
    use super::*;
    use crate::extensions::unicode;
    use crate::extensions::unicode::subdivision_suffix;
    use crate::subtags::region;

    #[test]
    fn region_override_test() {
        let val = unicode::value!("uksct");
        let rg: RegionOverride = val.try_into().unwrap();
        assert_eq!(rg.0.region, region!("UK"));
        assert_eq!(rg.0.suffix, subdivision_suffix!("sct"));

        let val = unicode::value!("usca");
        let rg: RegionOverride = val.try_into().unwrap();
        assert_eq!(rg.0.region, region!("US"));
        assert_eq!(rg.0.suffix, subdivision_suffix!("ca"));

        let val = unicode::value!("419bel");
        let rg: RegionOverride = val.try_into().unwrap();
        assert_eq!(rg.0.region, region!("419"));
        assert_eq!(rg.0.suffix, subdivision_suffix!("bel"));

        let val = unicode::value!("uszzzz");
        let rg: RegionOverride = val.try_into().unwrap();
        assert_eq!(rg.0.region, region!("us"));
        assert_eq!(rg.0.suffix, subdivision_suffix!("zzzz"));

        for i in &["4aabel", "a4bel", "ukabcde"] {
            let val = Value::try_from_str(i).unwrap();
            let rg: Result<RegionOverride, _> = val.try_into();
            assert!(rg.is_err());
        }
    }
}
