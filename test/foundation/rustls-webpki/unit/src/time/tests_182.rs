#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_before_unix_epoch() {
        assert_eq!(
            DAYS_BEFORE_UNIX_EPOCH_AD,
            days_before_year_ad(UNIX_EPOCH_YEAR)
        );
    }

    #[test]
    fn test_days_before_year_since_unix_epoch() {
        assert_eq!(Ok(0), days_before_year_since_unix_epoch(UNIX_EPOCH_YEAR));
        assert_eq!(
            Ok(365),
            days_before_year_since_unix_epoch(UNIX_EPOCH_YEAR + 1)
        );
        assert_eq!(
            Err(Error::BadDerTime),
            days_before_year_since_unix_epoch(UNIX_EPOCH_YEAR - 1)
        );
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2017, 1), 31);
        assert_eq!(days_in_month(2017, 2), 28);
        assert_eq!(days_in_month(2017, 3), 31);
        assert_eq!(days_in_month(2017, 4), 30);
        assert_eq!(days_in_month(2017, 5), 31);
        assert_eq!(days_in_month(2017, 6), 30);
        assert_eq!(days_in_month(2017, 7), 31);
        assert_eq!(days_in_month(2017, 8), 31);
        assert_eq!(days_in_month(2017, 9), 30);
        assert_eq!(days_in_month(2017, 10), 31);
        assert_eq!(days_in_month(2017, 11), 30);
        assert_eq!(days_in_month(2017, 12), 31);

        // leap cases
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2004, 2), 29);
        assert_eq!(days_in_month(2016, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
    }

    #[test]
    fn test_time_from_ymdhms_utc() {
        // 1969-12-31 00:00:00
        assert_eq!(
            Err(Error::BadDerTime),
            time_from_ymdhms_utc(UNIX_EPOCH_YEAR - 1, 1, 1, 0, 0, 0)
        );

        // 1969-12-31 23:59:59
        assert_eq!(
            Err(Error::BadDerTime),
            time_from_ymdhms_utc(UNIX_EPOCH_YEAR - 1, 12, 31, 23, 59, 59)
        );

        // 1970-01-01 00:00:00
        assert_eq!(
            UnixTime::since_unix_epoch(Duration::from_secs(0)),
            time_from_ymdhms_utc(UNIX_EPOCH_YEAR, 1, 1, 0, 0, 0).unwrap()
        );

        // 1970-01-01 00:00:01
        assert_eq!(
            UnixTime::since_unix_epoch(Duration::from_secs(1)),
            time_from_ymdhms_utc(UNIX_EPOCH_YEAR, 1, 1, 0, 0, 1).unwrap()
        );

        // 1971-01-01 00:00:00
        assert_eq!(
            UnixTime::since_unix_epoch(Duration::from_secs(365 * 86400)),
            time_from_ymdhms_utc(UNIX_EPOCH_YEAR + 1, 1, 1, 0, 0, 0).unwrap()
        );

        // year boundary
        assert_eq!(
            UnixTime::since_unix_epoch(Duration::from_secs(1_483_228_799)),
            time_from_ymdhms_utc(2016, 12, 31, 23, 59, 59).unwrap()
        );
        assert_eq!(
            UnixTime::since_unix_epoch(Duration::from_secs(1_483_228_800)),
            time_from_ymdhms_utc(2017, 1, 1, 0, 0, 0).unwrap()
        );

        // not a leap year
        assert_eq!(
            UnixTime::since_unix_epoch(Duration::from_secs(1_492_449_162)),
            time_from_ymdhms_utc(2017, 4, 17, 17, 12, 42).unwrap()
        );

        // leap year, post-feb
        assert_eq!(
            UnixTime::since_unix_epoch(Duration::from_secs(1_460_913_162)),
            time_from_ymdhms_utc(2016, 4, 17, 17, 12, 42).unwrap()
        );
    }
}
