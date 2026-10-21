#[cfg(test)]
mod tests {
    use i32;
    use std::{
        format,
        time::{Duration, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn test_datetime() {
        let case = |expected: &str, secs: i64, micros: u32| {
            let timestamp = if secs >= 0 {
                UNIX_EPOCH + Duration::new(secs as u64, micros * 1_000)
            } else {
                (UNIX_EPOCH - Duration::new(!secs as u64 + 1, 0)) + Duration::new(0, micros * 1_000)
            };
            assert_eq!(
                expected,
                format!("{}", DateTime::from(timestamp)),
                "secs: {}, micros: {}",
                secs,
                micros
            )
        };

        // Mostly generated with:
        //  - date -jur <secs> +"%Y-%m-%dT%H:%M:%S.000000Z"
        //  - http://unixtimestamp.50x.eu/

        case("1970-01-01T00:00:00.000000Z", 0, 0);

        case("1970-01-01T00:00:00.000001Z", 0, 1);
        case("1970-01-01T00:00:00.500000Z", 0, 500_000);
        case("1970-01-01T00:00:01.000001Z", 1, 1);
        case("1970-01-01T00:01:01.000001Z", 60 + 1, 1);
        case("1970-01-01T01:01:01.000001Z", 60 * 60 + 60 + 1, 1);
        case(
            "1970-01-02T01:01:01.000001Z",
            24 * 60 * 60 + 60 * 60 + 60 + 1,
            1,
        );

        case("1969-12-31T23:59:59.000000Z", -1, 0);
        case("1969-12-31T23:59:59.000001Z", -1, 1);
        case("1969-12-31T23:59:59.500000Z", -1, 500_000);
        case("1969-12-31T23:58:59.000001Z", -60 - 1, 1);
        case("1969-12-31T22:58:59.000001Z", -60 * 60 - 60 - 1, 1);
        case(
            "1969-12-30T22:58:59.000001Z",
            -24 * 60 * 60 - 60 * 60 - 60 - 1,
            1,
        );

        case("2038-01-19T03:14:07.000000Z", i32::MAX as i64, 0);
        case("2038-01-19T03:14:08.000000Z", i32::MAX as i64 + 1, 0);
        case("1901-12-13T20:45:52.000000Z", i32::MIN as i64, 0);
        case("1901-12-13T20:45:51.000000Z", i32::MIN as i64 - 1, 0);

        // Skipping these tests on windows as std::time::SystemTime range is low
        // on Windows compared with that of Unix which can cause the following
        // high date value tests to panic
        #[cfg(not(target_os = "windows"))]
        {
            case("+292277026596-12-04T15:30:07.000000Z", i64::MAX, 0);
            case("+292277026596-12-04T15:30:06.000000Z", i64::MAX - 1, 0);
            case("-292277022657-01-27T08:29:53.000000Z", i64::MIN + 1, 0);
        }

        case("1900-01-01T00:00:00.000000Z", -2208988800, 0);
        case("1899-12-31T23:59:59.000000Z", -2208988801, 0);
        case("2345-06-07T08:09:01.000000Z", 11847456541, 0);

        // Skipping pre-1601 dates on Windows: as of Rust 1.94, SystemTime
        // subtraction panics when the result would be before the Windows
        // FILETIME epoch (1601-01-01). See Rust 1.94.0 compatibility notes.
        #[cfg(not(target_os = "windows"))]
        {
            case("1234-05-06T07:08:09.000000Z", -23215049511, 0);
            case("0000-01-01T00:00:00.000000Z", -62167219200, 0);
            case("-0001-12-31T23:59:59.000000Z", -62167219201, 0);
            case("-1234-05-06T07:08:09.000000Z", -101097651111, 0);
            case("-2345-06-07T08:09:01.000000Z", -136154620259, 0);
        }
    }
}
