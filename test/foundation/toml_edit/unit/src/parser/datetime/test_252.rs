#[cfg(test)]
#[cfg(feature = "parse")]
#[cfg(feature = "display")]
mod test {
    use super::*;

    #[test]
    fn offset_date_time() {
        let inputs = [
            (
                "1979-05-27T07:32:00Z",
                Datetime {
                    date: Some(Date {
                        year: 1979,
                        month: 5,
                        day: 27,
                    }),
                    time: Some(Time {
                        hour: 7,
                        minute: 32,
                        second: 0,
                        nanosecond: 0,
                    }),
                    offset: Some(Offset::Z),
                },
            ),
            (
                "1979-05-27T00:32:00-07:00",
                Datetime {
                    date: Some(Date {
                        year: 1979,
                        month: 5,
                        day: 27,
                    }),
                    time: Some(Time {
                        hour: 0,
                        minute: 32,
                        second: 0,
                        nanosecond: 0,
                    }),
                    offset: Some(Offset::Custom { minutes: -7 * 60 }),
                },
            ),
            (
                "1979-05-27T00:32:00-00:36",
                Datetime {
                    date: Some(Date {
                        year: 1979,
                        month: 5,
                        day: 27,
                    }),
                    time: Some(Time {
                        hour: 0,
                        minute: 32,
                        second: 0,
                        nanosecond: 0,
                    }),
                    offset: Some(Offset::Custom { minutes: -36 }),
                },
            ),
            (
                "1979-05-27T00:32:00.999999",
                Datetime {
                    date: Some(Date {
                        year: 1979,
                        month: 5,
                        day: 27,
                    }),
                    time: Some(Time {
                        hour: 0,
                        minute: 32,
                        second: 0,
                        nanosecond: 999999000,
                    }),
                    offset: None,
                },
            ),
        ];
        for (input, expected) in inputs {
            dbg!(input);
            let actual = date_time.parse(new_input(input)).unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn local_date_time() {
        let inputs = [
            (
                "1979-05-27T07:32:00",
                Datetime {
                    date: Some(Date {
                        year: 1979,
                        month: 5,
                        day: 27,
                    }),
                    time: Some(Time {
                        hour: 7,
                        minute: 32,
                        second: 0,
                        nanosecond: 0,
                    }),
                    offset: None,
                },
            ),
            (
                "1979-05-27T00:32:00.999999",
                Datetime {
                    date: Some(Date {
                        year: 1979,
                        month: 5,
                        day: 27,
                    }),
                    time: Some(Time {
                        hour: 0,
                        minute: 32,
                        second: 0,
                        nanosecond: 999999000,
                    }),
                    offset: None,
                },
            ),
        ];
        for (input, expected) in inputs {
            dbg!(input);
            let actual = date_time.parse(new_input(input)).unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn local_date() {
        let inputs = [
            (
                "1979-05-27",
                Datetime {
                    date: Some(Date {
                        year: 1979,
                        month: 5,
                        day: 27,
                    }),
                    time: None,
                    offset: None,
                },
            ),
            (
                "2017-07-20",
                Datetime {
                    date: Some(Date {
                        year: 2017,
                        month: 7,
                        day: 20,
                    }),
                    time: None,
                    offset: None,
                },
            ),
        ];
        for (input, expected) in inputs {
            dbg!(input);
            let actual = date_time.parse(new_input(input)).unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn local_time() {
        let inputs = [
            (
                "07:32:00",
                Datetime {
                    date: None,
                    time: Some(Time {
                        hour: 7,
                        minute: 32,
                        second: 0,
                        nanosecond: 0,
                    }),
                    offset: None,
                },
            ),
            (
                "00:32:00.999999",
                Datetime {
                    date: None,
                    time: Some(Time {
                        hour: 0,
                        minute: 32,
                        second: 0,
                        nanosecond: 999999000,
                    }),
                    offset: None,
                },
            ),
        ];
        for (input, expected) in inputs {
            dbg!(input);
            let actual = date_time.parse(new_input(input)).unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn time_fraction_truncated() {
        let input = "1987-07-05T17:45:00.123456789012345Z";
        date_time.parse(new_input(input)).unwrap();
    }
}
