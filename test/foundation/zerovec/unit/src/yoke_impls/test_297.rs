#[cfg(test)]
#[allow(non_camel_case_types, non_snake_case)]
mod test {
    use super::*;
    use crate::{VarZeroSlice, ZeroSlice};
    use databake::*;

    // Note: The following derives cover Yoke as well as Serde and databake. These may partially
    // duplicate tests elsewhere in this crate, but they are here for completeness.

    #[derive(yoke::Yokeable, zerofrom::ZeroFrom)]
    #[cfg_attr(feature = "serialization", derive(data_serialization::Deserialize, data_serialization::Serialize))]
    #[cfg_attr(feature = "databake", derive(databake::Bake))]
    #[cfg_attr(feature = "databake", databake(path = zerovec::yoke_impls::test))]
    struct DeriveTest_ZeroVec<'data> {
        #[cfg_attr(feature = "serialization", serialization(borrow))]
        pub _data: ZeroVec<'data, u16>,
    }

    #[test]
    fn bake_ZeroVec() {
        test_bake!(
            DeriveTest_ZeroVec<'static>,
            crate::yoke_impls::test::DeriveTest_ZeroVec {
                _data: crate::ZeroVec::new(),
            },
            zerovec,
        );
    }

    #[derive(yoke::Yokeable)]
    #[cfg_attr(feature = "serialization", derive(data_serialization::Deserialize, data_serialization::Serialize))]
    #[cfg_attr(feature = "databake", derive(databake::Bake))]
    #[cfg_attr(feature = "databake", databake(path = zerovec::yoke_impls::test))]
    struct DeriveTest_ZeroSlice<'data> {
        #[cfg_attr(feature = "serialization", serialization(borrow))]
        pub _data: &'data ZeroSlice<u16>,
    }

    #[test]
    fn bake_ZeroSlice() {
        test_bake!(
            DeriveTest_ZeroSlice<'static>,
            crate::yoke_impls::test::DeriveTest_ZeroSlice {
                _data: crate::ZeroSlice::new_empty(),
            },
            zerovec,
        );
    }

    #[derive(yoke::Yokeable, zerofrom::ZeroFrom)]
    #[cfg_attr(feature = "serialization", derive(data_serialization::Deserialize, data_serialization::Serialize))]
    #[cfg_attr(feature = "databake", derive(databake::Bake))]
    #[cfg_attr(feature = "databake", databake(path = zerovec::yoke_impls::test))]
    struct DeriveTest_VarZeroVec<'data> {
        #[cfg_attr(feature = "serialization", serialization(borrow))]
        pub _data: VarZeroVec<'data, str>,
    }

    #[test]
    fn bake_VarZeroVec() {
        test_bake!(
            DeriveTest_VarZeroVec<'static>,
            crate::yoke_impls::test::DeriveTest_VarZeroVec {
                _data: crate::vecs::VarZeroVec16::new(),
            },
            zerovec,
        );
    }

    #[derive(yoke::Yokeable)]
    #[cfg_attr(feature = "serialization", derive(data_serialization::Deserialize, data_serialization::Serialize))]
    #[cfg_attr(feature = "databake", derive(databake::Bake))]
    #[cfg_attr(feature = "databake", databake(path = zerovec::yoke_impls::test))]
    struct DeriveTest_VarZeroSlice<'data> {
        #[cfg_attr(feature = "serialization", serialization(borrow))]
        pub _data: &'data VarZeroSlice<str>,
    }

    #[test]
    fn bake_VarZeroSlice() {
        test_bake!(
            DeriveTest_VarZeroSlice<'static>,
            crate::yoke_impls::test::DeriveTest_VarZeroSlice {
                _data: crate::vecs::VarZeroSlice16::new_empty()
            },
            zerovec,
        );
    }

    #[derive(yoke::Yokeable, zerofrom::ZeroFrom)]
    #[cfg_attr(feature = "serialization", derive(data_serialization::Deserialize, data_serialization::Serialize))]
    #[cfg_attr(feature = "databake", derive(databake::Bake))]
    #[cfg_attr(feature = "databake", databake(path = zerovec::yoke_impls::test))]
    #[yoke(prove_covariance_manually)]
    struct DeriveTest_ZeroMap<'data> {
        #[cfg_attr(feature = "serialization", serialization(borrow))]
        pub _data: ZeroMap<'data, [u8], str>,
    }

    #[test]
    fn bake_ZeroMap() {
        test_bake!(
            DeriveTest_ZeroMap<'static>,
            crate::yoke_impls::test::DeriveTest_ZeroMap {
                _data: unsafe {
                    #[allow(unused_unsafe)]
                    crate::ZeroMap::from_parts_unchecked(
                        crate::vecs::VarZeroVec16::new(),
                        crate::vecs::VarZeroVec16::new(),
                    )
                },
            },
            zerovec,
        );
    }

    #[derive(yoke::Yokeable)]
    #[cfg_attr(feature = "serialization", derive(data_serialization::Deserialize, data_serialization::Serialize))]
    #[cfg_attr(feature = "databake", derive(databake::Bake))]
    #[cfg_attr(feature = "databake", databake(path = zerovec::yoke_impls::test))]
    #[yoke(prove_covariance_manually)]
    struct DeriveTest_ZeroMapBorrowed<'data> {
        #[cfg_attr(feature = "serialization", serialization(borrow))]
        pub _data: ZeroMapBorrowed<'data, [u8], str>,
    }

    #[test]
    fn bake_ZeroMapBorrowed() {
        test_bake!(
            DeriveTest_ZeroMapBorrowed<'static>,
            crate::yoke_impls::test::DeriveTest_ZeroMapBorrowed {
                _data: unsafe {
                    #[allow(unused_unsafe)]
                    crate::maps::ZeroMapBorrowed::from_parts_unchecked(
                        crate::vecs::VarZeroSlice16::new_empty(),
                        crate::vecs::VarZeroSlice16::new_empty(),
                    )
                },
            },
            zerovec,
        );
    }

    #[derive(yoke::Yokeable, zerofrom::ZeroFrom)]
    #[cfg_attr(feature = "serialization", derive(data_serialization::Deserialize, data_serialization::Serialize))]
    #[cfg_attr(feature = "databake", derive(databake::Bake))]
    #[cfg_attr(feature = "databake", databake(path = zerovec::yoke_impls::test))]
    #[yoke(prove_covariance_manually)]
    struct DeriveTest_ZeroMapWithULE<'data> {
        #[cfg_attr(feature = "serialization", serialization(borrow))]
        pub _data: ZeroMap<'data, ZeroSlice<u32>, str>,
    }

    #[test]
    fn bake_ZeroMapWithULE() {
        test_bake!(
            DeriveTest_ZeroMapWithULE<'static>,
            crate::yoke_impls::test::DeriveTest_ZeroMapWithULE {
                _data: unsafe {
                    #[allow(unused_unsafe)]
                    crate::ZeroMap::from_parts_unchecked(
                        crate::vecs::VarZeroVec16::new(),
                        crate::vecs::VarZeroVec16::new(),
                    )
                },
            },
            zerovec,
        );
    }

    #[derive(yoke::Yokeable, zerofrom::ZeroFrom)]
    #[cfg_attr(feature = "serialization", derive(data_serialization::Deserialize, data_serialization::Serialize))]
    #[cfg_attr(feature = "databake", derive(databake::Bake))]
    #[cfg_attr(feature = "databake", databake(path = zerovec::yoke_impls::test))]
    #[yoke(prove_covariance_manually)]
    struct DeriveTest_ZeroMap2d<'data> {
        #[cfg_attr(feature = "serialization", serialization(borrow))]
        pub _data: ZeroMap2d<'data, u16, u16, str>,
    }

    #[test]
    fn bake_ZeroMap2d() {
        test_bake!(
            DeriveTest_ZeroMap2d<'static>,
            crate::yoke_impls::test::DeriveTest_ZeroMap2d {
                _data: unsafe {
                    #[allow(unused_unsafe)]
                    crate::ZeroMap2d::from_parts_unchecked(
                        crate::ZeroVec::new(),
                        crate::ZeroVec::new(),
                        crate::ZeroVec::new(),
                        crate::vecs::VarZeroVec16::new(),
                    )
                },
            },
            zerovec,
        );
    }

    #[derive(yoke::Yokeable)]
    #[cfg_attr(feature = "serialization", derive(data_serialization::Deserialize, data_serialization::Serialize))]
    #[cfg_attr(feature = "databake", derive(databake::Bake))]
    #[cfg_attr(feature = "databake", databake(path = zerovec::yoke_impls::test))]
    #[yoke(prove_covariance_manually)]
    struct DeriveTest_ZeroMap2dBorrowed<'data> {
        #[cfg_attr(feature = "serialization", serialization(borrow))]
        pub _data: ZeroMap2dBorrowed<'data, u16, u16, str>,
    }

    #[test]
    fn bake_ZeroMap2dBorrowed() {
        test_bake!(
            DeriveTest_ZeroMap2dBorrowed<'static>,
            crate::yoke_impls::test::DeriveTest_ZeroMap2dBorrowed {
                _data: unsafe {
                    #[allow(unused_unsafe)]
                    crate::maps::ZeroMap2dBorrowed::from_parts_unchecked(
                        crate::ZeroSlice::new_empty(),
                        crate::ZeroSlice::new_empty(),
                        crate::ZeroSlice::new_empty(),
                        crate::vecs::VarZeroSlice16::new_empty(),
                    )
                },
            },
            zerovec,
        );
    }
}
