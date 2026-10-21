#[cfg(test)]
mod tests {
    use crate::{VarZeroVec, ZeroMap, ZeroSlice, ZeroVec};

    #[test]
    #[cfg(feature = "schemars")]
    fn schema_zerovec_u32() {
        let generator = schemars::SchemaGenerator::default();
        let schema = generator.into_root_schema_for::<ZeroVec<u32>>();

        insta::assert_json_snapshot!(schema);
    }

    #[test]
    #[cfg(feature = "schemars")]
    fn schema_zerovec_char() {
        let generator = schemars::SchemaGenerator::default();
        let schema = generator.into_root_schema_for::<ZeroVec<char>>();

        insta::assert_json_snapshot!(schema);
    }

    #[test]
    #[cfg(feature = "schemars")]
    fn schema_varzerovec_str() {
        let generator = schemars::SchemaGenerator::default();
        let schema = generator.into_root_schema_for::<VarZeroVec<str>>();

        insta::assert_json_snapshot!(schema);
    }

    #[test]
    #[cfg(feature = "schemars")]
    fn schema_varzerovec_zeroslice() {
        let generator = schemars::SchemaGenerator::default();
        let schema = generator.into_root_schema_for::<VarZeroVec<ZeroSlice<u32>>>();

        insta::assert_json_snapshot!(schema);
    }

    #[test]
    fn schema_zeromap_u32_str() {
        let generator = schemars::SchemaGenerator::default();
        let schema = generator.into_root_schema_for::<ZeroMap<u32, str>>();

        insta::assert_json_snapshot!(schema);
    }
}
