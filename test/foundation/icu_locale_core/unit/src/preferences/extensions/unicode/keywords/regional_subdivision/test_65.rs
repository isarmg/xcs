#[cfg(test)]
mod test {
    use super::*;
    use crate::extensions::unicode;
    use crate::extensions::unicode::subdivision_suffix;
    use crate::subtags::region;

    #[test]
    fn region_subdivision_test() {
        let val = unicode::value!("uksct");
        let rg: RegionalSubdivision = val.try_into().unwrap();
        assert_eq!(rg.region, region!("UK"));
        assert_eq!(rg.suffix, subdivision_suffix!("sct"));

        for i in &["4aabel", "a4bel", "ukabcde"] {
            let val = Value::try_from_str(i).unwrap();
            let rg: Result<RegionalSubdivision, _> = val.try_into();
            assert!(rg.is_err());
        }
    }
}
