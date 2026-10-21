#[cfg(all(test, feature = "alloc", feature = "password-hash"))]
mod tests {

    use super::*;

    #[test]
    fn params_builder_bad_values() {
        assert_eq!(
            ParamsBuilder::new().m_cost(Params::MIN_M_COST - 1).build(),
            Err(Error::MemoryTooLittle)
        );
        assert_eq!(
            ParamsBuilder::new().t_cost(Params::MIN_T_COST - 1).build(),
            Err(Error::TimeTooSmall)
        );
        assert_eq!(
            ParamsBuilder::new().p_cost(Params::MIN_P_COST - 1).build(),
            Err(Error::ThreadsTooFew)
        );
        assert_eq!(
            ParamsBuilder::new()
                .m_cost(Params::DEFAULT_P_COST * 8 - 1)
                .build(),
            Err(Error::MemoryTooLittle)
        );
        assert_eq!(
            ParamsBuilder::new()
                .m_cost((Params::MAX_P_COST + 1) * 8)
                .p_cost(Params::MAX_P_COST + 1)
                .build(),
            Err(Error::ThreadsTooMany)
        );
    }

    #[test]
    fn associated_data_too_long() {
        let ret = AssociatedData::new(&[0u8; Params::MAX_DATA_LEN + 1]);
        assert_eq!(ret, Err(Error::AdTooLong));
    }

    #[test]
    fn keyid_too_long() {
        let ret = KeyId::new(&[0u8; Params::MAX_KEYID_LEN + 1]);
        assert_eq!(ret, Err(Error::KeyIdTooLong));
    }
}
