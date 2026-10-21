#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn check_list() {
        let mut users = Users::new();
        assert!(users.list().is_empty());
        users.refresh();
        assert!(users.list().len() >= MIN_USERS);
    }

    // This test exists to ensure that the `TryFrom<usize>` and `FromStr` traits are implemented
    // on `Uid`, `Gid` and `Pid`.
    #[allow(clippy::unnecessary_fallible_conversions)]
    #[test]
    fn check_uid_gid_from_impls() {
        use std::convert::TryFrom;
        use std::str::FromStr;

        #[cfg(not(windows))]
        {
            assert!(crate::Uid::try_from(0usize).is_ok());
            assert!(crate::Uid::from_str("0").is_ok());
        }
        #[cfg(windows)]
        {
            assert!(crate::Uid::from_str("S-1-5-18").is_ok()); // SECURITY_LOCAL_SYSTEM_RID
            assert!(crate::Uid::from_str("0").is_err());
        }

        assert!(crate::Gid::try_from(0usize).is_ok());
        assert!(crate::Gid::from_str("0").is_ok());
    }

    #[test]
    fn check_groups() {
        if !crate::IS_SUPPORTED_SYSTEM {
            return;
        }
        assert!(!Groups::new_with_refreshed_list().is_empty());
    }
}
