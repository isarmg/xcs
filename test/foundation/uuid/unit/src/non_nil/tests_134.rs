#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_nil_with_option_size() {
        assert_eq!(
            std::mem::size_of::<Option<NonNilUuid>>(),
            std::mem::size_of::<Uuid>()
        );
    }

    #[test]
    fn test_non_nil() {
        let uuid = Uuid::from_u128(0x0123456789abcdef0123456789abcdef);

        assert_eq!(Uuid::from(NonNilUuid::try_from(uuid).unwrap()), uuid);
        assert_eq!(NonNilUuid::new(uuid).unwrap(), uuid);
        assert_eq!(unsafe { NonNilUuid::new_unchecked(uuid) }, uuid);

        assert!(NonNilUuid::try_from(Uuid::nil()).is_err());
        assert!(NonNilUuid::new(Uuid::nil()).is_none());
    }

    #[test]
    fn test_non_nil_formatting() {
        let uuid = Uuid::from_u128(0x0123456789abcdef0123456789abcdef);
        let non_nil = NonNilUuid::try_from(uuid).unwrap();

        assert_eq!(format!("{uuid}"), format!("{non_nil}"));
        assert_eq!(format!("{uuid:?}"), format!("{non_nil:?}"));
    }

    #[test]
    fn test_non_nil_ord() {
        let uuid1 = Uuid::from_u128(0x0123456789abcdef0123456789abcdef);
        let uuid2 = Uuid::from_u128(0x0123456789abcdef0123456789abcdf0);

        let non_nil1 = NonNilUuid::try_from(uuid1).unwrap();
        let non_nil2 = NonNilUuid::try_from(uuid2).unwrap();

        // Ordering between NonNilUuid instances
        assert!(non_nil1 < non_nil2);
        assert!(non_nil2 > non_nil1);

        // Ordering between NonNilUuid and Uuid
        assert!(non_nil1 < uuid2);
        assert!(non_nil2 > uuid1);

        // Ordering between Uuid and NonNilUuid
        assert!(uuid1 < non_nil2);
        assert!(uuid2 > non_nil1);

        // Equality
        let non_nil1_copy = NonNilUuid::try_from(uuid1).unwrap();
        assert_eq!(non_nil1, non_nil1_copy);
        assert!(non_nil1 <= non_nil1_copy);
        assert!(non_nil1 >= non_nil1_copy);

        // Equality between NonNilUuid and Uuid
        assert_eq!(non_nil1, uuid1);
        assert_eq!(uuid1, non_nil1);
        assert!(non_nil1 >= uuid1);
        assert!(non_nil1 <= uuid1);
        assert!(uuid1 >= non_nil1);
        assert!(uuid1 <= non_nil1);
    }
}
