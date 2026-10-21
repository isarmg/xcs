#[cfg(test)]
mod tests {
    use super::Date;

    macro_rules! reflexive_display {
        ($string:expr) => (
            assert_eq!(Date::parse($string).unwrap().to_string(), $string);
        )
    }

    #[test]
    fn display() {
        reflexive_display!("2019-05-08");
        reflexive_display!("2000-01-01");
        reflexive_display!("2000-12-31");
        reflexive_display!("2090-12-31");
        reflexive_display!("1999-02-19");
        reflexive_display!("9999-12-31");
    }
}
