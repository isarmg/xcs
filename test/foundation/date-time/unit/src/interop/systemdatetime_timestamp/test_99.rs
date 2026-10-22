#[cfg(test)]
mod test {
    use super::*;
    use crate::ext::NumericalDuration;

    #[test]
    fn systime_to_timestamp() -> crate::Result<()> {
        let ts = Timestamp::from(SystemTime::UNIX_EPOCH + 5.seconds());
        assert_eq!(ts, Timestamp::from_seconds(5)?);

        let ts = Timestamp::from(SystemTime::UNIX_EPOCH - 5.seconds());
        assert_eq!(ts, Timestamp::from_seconds(-5)?);

        let ts = Timestamp::from(SystemTime::UNIX_EPOCH + 5.seconds() + 1.milliseconds());
        assert_eq!(ts, Timestamp::from_milliseconds(5_001)?);

        let ts = Timestamp::from(SystemTime::UNIX_EPOCH - 5.seconds() - 1.milliseconds());
        assert_eq!(ts, Timestamp::from_milliseconds(-5_001)?);

        Ok(())
    }
}
