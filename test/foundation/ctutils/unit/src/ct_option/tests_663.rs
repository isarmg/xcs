#[cfg(test)]
mod tests {
    use crate::{Choice, CtEq, CtOption, CtSelect};

    /// Example wrapped value for testing
    const VALUE: u8 = 42;

    /// Example option which is like `Option::Some`
    const SOME: CtOption<u8> = CtOption::new(VALUE, Choice::TRUE);

    /// Example option which is like `Option::None`
    const NONE: CtOption<u8> = CtOption::new(VALUE, Choice::FALSE);

    /// Another option containing a different value
    const OTHER: CtOption<u8> = CtOption::new(VALUE + 1, Choice::TRUE);

    /// Dummy error type
    #[derive(Debug, Eq, PartialEq)]
    struct Error;

    #[test]
    fn map_macro() {
        assert!(map!(NONE, u16::from).is_none().to_bool());
        assert_eq!(map!(SOME, u16::from).unwrap(), u16::from(VALUE));
    }

    #[test]
    fn unwrap_or_macro() {
        // Don't actually use this! It's just a test function implemented in variable-time
        #[allow(clippy::trivially_copy_pass_by_ref)]
        const fn select_vartime(a: &u8, b: &u8, choice: Choice) -> u8 {
            if choice.to_bool() { *b } else { *a }
        }

        assert_eq!(
            unwrap_or!(NONE, OTHER.unwrap(), select_vartime),
            OTHER.unwrap()
        );
        assert_eq!(unwrap_or!(SOME, OTHER.unwrap(), select_vartime), VALUE);
    }

    #[test]
    fn ct_eq() {
        assert!(NONE.ct_eq(&NONE).to_bool());
        assert!(NONE.ct_ne(&SOME).to_bool());
        assert!(SOME.ct_ne(&NONE).to_bool());
        assert!(SOME.ct_eq(&SOME).to_bool());
        assert!(SOME.ct_ne(&OTHER).to_bool());
    }

    #[test]
    fn ct_select() {
        assert!(NONE.ct_select(&SOME, Choice::FALSE).is_none().to_bool());
        assert!(NONE.ct_select(&SOME, Choice::TRUE).ct_eq(&SOME).to_bool());
        assert!(SOME.ct_select(&NONE, Choice::FALSE).ct_eq(&SOME).to_bool());
        assert!(SOME.ct_select(&NONE, Choice::TRUE).is_none().to_bool());
    }

    #[test]
    fn default() {
        assert!(NONE.ct_eq(&CtOption::default()).to_bool());
    }

    #[test]
    fn expect_some() {
        assert_eq!(SOME.expect("should succeed"), VALUE);
    }

    #[test]
    #[should_panic]
    fn expect_none() {
        let _ = NONE.expect("should panic");
    }

    #[test]
    fn into_option() {
        assert_eq!(SOME.into_option(), Some(VALUE));
        assert_eq!(NONE.into_option(), None);
    }

    #[test]
    fn into_option_copied() {
        assert_eq!(SOME.into_option_copied(), Some(VALUE));
        assert_eq!(NONE.into_option_copied(), None);
    }

    #[test]
    fn is_some() {
        assert!(SOME.is_some().to_bool());
        assert!(!NONE.is_some().to_bool());
    }

    #[test]
    fn is_none() {
        assert!(!SOME.is_none().to_bool());
        assert!(NONE.is_none().to_bool());
    }

    #[test]
    fn and() {
        assert!(SOME.and(NONE).is_none().to_bool());
        assert_eq!(SOME.and(OTHER).unwrap(), OTHER.unwrap());
    }

    #[test]
    fn and_then() {
        assert!(NONE.and_then(|_| NONE).is_none().to_bool());
        assert!(NONE.and_then(|_| SOME).is_none().to_bool());

        let ret = SOME.and_then(|value| {
            assert_eq!(VALUE, value);
            OTHER
        });
        assert!(ret.ct_eq(&OTHER).to_bool());
    }

    #[test]
    fn filter() {
        assert!(NONE.filter(|_| Choice::TRUE).ct_eq(&NONE).to_bool());
        assert!(NONE.filter(|_| Choice::FALSE).ct_eq(&NONE).to_bool());
        assert!(SOME.filter(|_| Choice::FALSE).ct_eq(&NONE).to_bool());

        let ret = SOME.filter(|&value| {
            assert_eq!(VALUE, value);
            Choice::TRUE
        });
        assert_eq!(ret.unwrap(), VALUE);
    }

    #[test]
    fn filter_by() {
        assert!(NONE.filter_by(Choice::FALSE).is_none().to_bool());
        assert!(NONE.filter_by(Choice::TRUE).is_none().to_bool());
        assert!(SOME.filter_by(Choice::FALSE).ct_eq(&NONE).to_bool());
        assert_eq!(SOME.filter_by(Choice::TRUE).unwrap(), VALUE);
    }

    #[test]
    fn insert() {
        let mut example = NONE;
        assert!(example.is_none().to_bool());

        let ret = example.insert(42);
        assert_eq!(ret, &42);
        assert!(example.is_some().to_bool());
    }

    #[test]
    fn insert_if() {
        let mut example = NONE;
        assert!(example.is_none().to_bool());

        example.insert_if(&42, Choice::FALSE);
        assert!(example.is_none().to_bool());

        example.insert_if(&42, Choice::TRUE);
        assert_eq!(example.unwrap(), 42);
    }

    #[test]
    fn map() {
        assert!(NONE.map(|value| value + 1).ct_eq(&NONE).to_bool());
        assert!(SOME.map(|value| value + 1).ct_eq(&OTHER).to_bool());
    }

    #[test]
    fn map_or() {
        let example = 52;
        assert_eq!(NONE.map_or(example, |value| value + 1), example);
        assert_eq!(SOME.map_or(example, |value| value + 1), VALUE + 1);
    }

    #[test]
    fn map_or_default() {
        assert_eq!(NONE.map_or_default(|value| value + 1), Default::default());
        assert_eq!(SOME.map_or_default(|value| value + 1), VALUE + 1);
    }

    #[test]
    fn ok_or() {
        assert_eq!(NONE.ok_or(Error), Err(Error));
        assert_eq!(SOME.ok_or(Error), Ok(VALUE));
    }

    #[test]
    fn ok_or_else() {
        assert_eq!(NONE.ok_or_else(|| Error), Err(Error));
        assert_eq!(SOME.ok_or_else(|| Error), Ok(VALUE));
    }

    #[test]
    fn or() {
        assert!(NONE.or(NONE).is_none().to_bool());
        assert!(SOME.or(NONE).ct_eq(&SOME).to_bool());
        assert!(NONE.or(SOME).ct_eq(&SOME).to_bool());
        assert!(SOME.or(OTHER).ct_eq(&SOME).to_bool());
    }

    #[test]
    fn some() {
        assert!(CtOption::some(VALUE).ct_eq(&SOME).to_bool());
    }

    #[test]
    fn unwrap_some() {
        assert_eq!(SOME.unwrap(), VALUE);
    }

    #[test]
    #[should_panic]
    fn unwrap_none() {
        let _ = NONE.unwrap();
    }

    #[test]
    fn unwrap_or() {
        let example = 52;
        assert_eq!(NONE.unwrap_or(example), example);
        assert_eq!(SOME.unwrap_or(example), VALUE);
    }

    #[test]
    fn unwrap_or_default() {
        assert_eq!(NONE.unwrap_or_default(), Default::default());
        assert_eq!(SOME.unwrap_or_default(), VALUE);
    }

    #[test]
    fn xor() {
        assert!(NONE.xor(NONE).is_none().to_bool());
        assert!(SOME.xor(NONE).ct_eq(&SOME).to_bool());
        assert!(NONE.xor(SOME).ct_eq(&SOME).to_bool());
        assert!(SOME.xor(OTHER).is_none().to_bool());
    }

    #[test]
    fn zip() {
        assert!(NONE.zip(NONE).is_none().to_bool());
        assert!(NONE.zip(SOME).is_none().to_bool());
        assert!(SOME.zip(NONE).is_none().to_bool());
        assert_eq!(SOME.zip(OTHER).unwrap(), (SOME.unwrap(), OTHER.unwrap()));
    }

    #[test]
    fn zip_with() {
        assert!(NONE.zip_with(NONE, |a, b| a + b).is_none().to_bool());
        assert!(NONE.zip_with(SOME, |a, b| a + b).is_none().to_bool());
        assert!(SOME.zip_with(NONE, |a, b| a + b).is_none().to_bool());
        assert_eq!(
            SOME.zip_with(OTHER, |a, b| a + b).unwrap(),
            SOME.unwrap() + OTHER.unwrap()
        );
    }
}
