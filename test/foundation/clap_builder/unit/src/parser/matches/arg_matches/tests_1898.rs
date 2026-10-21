#[cfg(test)]
mod tests {
    use super::*;

    use crate::ArgAction;

    #[test]
    fn check_auto_traits() {
        static_assertions::assert_impl_all!(ArgMatches: Send, Sync, Unpin);
    }

    #[test]
    fn test_default_raw_values() {
        let mut values: RawValues<'_> = Default::default();
        assert_eq!(values.next(), None);
    }

    #[test]
    fn test_default_indices() {
        let mut indices: Indices<'_> = Indices::default();
        assert_eq!(indices.next(), None);
    }

    #[test]
    fn test_default_indices_with_shorter_lifetime() {
        let matches = ArgMatches::default();
        let mut indices = matches.indices_of("").unwrap_or_default();
        assert_eq!(indices.next(), None);
    }

    #[test]
    fn values_exact_size() {
        let l = crate::Command::new("test")
            .arg(
                crate::Arg::new("POTATO")
                    .action(ArgAction::Set)
                    .num_args(1..)
                    .required(true),
            )
            .try_get_matches_from(["test", "one"])
            .unwrap()
            .get_many::<String>("POTATO")
            .expect("present")
            .count();
        assert_eq!(l, 1);
    }

    #[test]
    fn os_values_exact_size() {
        let l = crate::Command::new("test")
            .arg(
                crate::Arg::new("POTATO")
                    .action(ArgAction::Set)
                    .num_args(1..)
                    .value_parser(crate::builder::ValueParser::os_string())
                    .required(true),
            )
            .try_get_matches_from(["test", "one"])
            .unwrap()
            .get_many::<OsString>("POTATO")
            .expect("present")
            .count();
        assert_eq!(l, 1);
    }

    #[test]
    fn indices_exact_size() {
        let l = crate::Command::new("test")
            .arg(
                crate::Arg::new("POTATO")
                    .action(ArgAction::Set)
                    .num_args(1..)
                    .required(true),
            )
            .try_get_matches_from(["test", "one"])
            .unwrap()
            .indices_of("POTATO")
            .expect("present")
            .len();
        assert_eq!(l, 1);
    }

    #[test]
    fn rev_iter() {
        let mut matches = crate::Command::new("myprog")
            .arg(crate::Arg::new("a").short('a').action(ArgAction::Append))
            .arg(crate::Arg::new("b").short('b').action(ArgAction::Append))
            .try_get_matches_from(vec!["myprog", "-a1", "-b1", "-b3"])
            .unwrap();

        let a_index = matches
            .indices_of("a")
            .expect("missing aopt indices")
            .collect::<Vec<_>>();
        dbg!(&a_index);
        let a_value = matches
            .remove_many::<String>("a")
            .expect("missing aopt values");
        dbg!(&a_value);
        let a = a_index.into_iter().zip(a_value).rev().collect::<Vec<_>>();
        dbg!(a);

        let b_index = matches
            .indices_of("b")
            .expect("missing aopt indices")
            .collect::<Vec<_>>();
        dbg!(&b_index);
        let b_value = matches
            .remove_many::<String>("b")
            .expect("missing aopt values");
        dbg!(&b_value);
        let b = b_index.into_iter().zip(b_value).rev().collect::<Vec<_>>();
        dbg!(b);
    }

    #[test]
    fn delete_id_without_returning() {
        let mut matches = crate::Command::new("myprog")
            .arg(crate::Arg::new("a").short('a').action(ArgAction::Append))
            .arg(crate::Arg::new("b").short('b').action(ArgAction::Append))
            .arg(crate::Arg::new("c").short('c').action(ArgAction::Append))
            .try_get_matches_from(vec!["myprog", "-b1", "-a1", "-b2"])
            .unwrap();
        let matches_ids_count = matches.ids().count();
        assert_eq!(matches_ids_count, 2);

        let _ = matches
            .try_clear_id("d")
            .expect_err("should fail due to there is no arg 'd'");

        let c_was_presented = matches
            .try_clear_id("c")
            .expect("doesn't fail because there is no matches for 'c' argument");
        assert!(!c_was_presented);
        let matches_ids_count = matches.ids().count();
        assert_eq!(matches_ids_count, 2);

        let b_was_presented = matches.try_clear_id("b").unwrap();
        assert!(b_was_presented);
        let matches_ids_count = matches.ids().count();
        assert_eq!(matches_ids_count, 1);

        let a_was_presented = matches.try_clear_id("a").unwrap();
        assert!(a_was_presented);
        let matches_ids_count = matches.ids().count();
        assert_eq!(matches_ids_count, 0);
    }
}
