#[test]
fn test_calendar_aliases() {
    fn test(s: &str, expectation: CalendarAlgorithm) {
        let v = Value::try_from_str(s).unwrap();
        let parsed = CalendarAlgorithm::try_from(&v).unwrap();
        assert_eq!(parsed, expectation, "{s} must parse to {expectation:?}");
    }

    test("ethiopic-amete-alem", CalendarAlgorithm::Ethioaa);
    test("ethiopic", CalendarAlgorithm::Ethiopic);
    test("ethioaa", CalendarAlgorithm::Ethioaa);
    test(
        "islamicc",
        CalendarAlgorithm::Hijri(Some(HijriCalendarAlgorithm::Civil)),
    );
    test(
        "islamic-civil",
        CalendarAlgorithm::Hijri(Some(HijriCalendarAlgorithm::Civil)),
    );
    test(
        "islamic-rgsa",
        CalendarAlgorithm::Hijri(Some(HijriCalendarAlgorithm::Rgsa)),
    );
    test("islamic", CalendarAlgorithm::Hijri(None));
}
