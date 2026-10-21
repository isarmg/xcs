#[cfg(test)]
mod test {
    use super::Arg;
    use super::ArgAction;

    #[test]
    fn flag_display_long() {
        let mut f = Arg::new("flg").long("flag").action(ArgAction::SetTrue);
        f._build();

        assert_eq!(f.to_string(), "--flag");
    }

    #[test]
    fn flag_display_short() {
        let mut f2 = Arg::new("flg").short('f').action(ArgAction::SetTrue);
        f2._build();

        assert_eq!(f2.to_string(), "-f");
    }

    #[test]
    fn flag_display_count() {
        let mut f2 = Arg::new("flg").long("flag").action(ArgAction::Count);
        f2._build();

        assert_eq!(f2.to_string(), "--flag...");
    }

    #[test]
    fn flag_display_single_alias() {
        let mut f = Arg::new("flg")
            .long("flag")
            .visible_alias("als")
            .action(ArgAction::SetTrue);
        f._build();

        assert_eq!(f.to_string(), "--flag");
    }

    #[test]
    fn flag_display_multiple_aliases() {
        let mut f = Arg::new("flg").short('f').action(ArgAction::SetTrue);
        f.aliases = vec![
            ("alias_not_visible".into(), false),
            ("f2".into(), true),
            ("f3".into(), true),
            ("f4".into(), true),
        ];
        f._build();

        assert_eq!(f.to_string(), "-f");
    }

    #[test]
    fn flag_display_single_short_alias() {
        let mut f = Arg::new("flg").short('a').action(ArgAction::SetTrue);
        f.short_aliases = vec![('b', true)];
        f._build();

        assert_eq!(f.to_string(), "-a");
    }

    #[test]
    fn flag_display_multiple_short_aliases() {
        let mut f = Arg::new("flg").short('a').action(ArgAction::SetTrue);
        f.short_aliases = vec![('b', false), ('c', true), ('d', true), ('e', true)];
        f._build();

        assert_eq!(f.to_string(), "-a");
    }

    // Options

    #[test]
    fn option_display_multiple_occurrences() {
        let mut o = Arg::new("opt").long("option").action(ArgAction::Append);
        o._build();

        assert_eq!(o.to_string(), "--option <opt>");
    }

    #[test]
    fn option_display_multiple_values() {
        let mut o = Arg::new("opt")
            .long("option")
            .action(ArgAction::Set)
            .num_args(1..);
        o._build();

        assert_eq!(o.to_string(), "--option <opt>...");
    }

    #[test]
    fn option_display_zero_or_more_values() {
        let mut o = Arg::new("opt")
            .long("option")
            .action(ArgAction::Set)
            .num_args(0..);
        o._build();

        assert_eq!(o.to_string(), "--option [<opt>...]");
    }

    #[test]
    fn option_display_one_or_more_values() {
        let mut o = Arg::new("opt")
            .long("option")
            .action(ArgAction::Set)
            .num_args(1..);
        o._build();

        assert_eq!(o.to_string(), "--option <opt>...");
    }

    #[test]
    fn option_display_zero_or_more_values_with_value_name() {
        let mut o = Arg::new("opt")
            .short('o')
            .action(ArgAction::Set)
            .num_args(0..)
            .value_names(["file"]);
        o._build();

        assert_eq!(o.to_string(), "-o [<file>...]");
    }

    #[test]
    fn option_display_one_or_more_values_with_value_name() {
        let mut o = Arg::new("opt")
            .short('o')
            .action(ArgAction::Set)
            .num_args(1..)
            .value_names(["file"]);
        o._build();

        assert_eq!(o.to_string(), "-o <file>...");
    }

    #[test]
    fn option_display_optional_value() {
        let mut o = Arg::new("opt")
            .long("option")
            .action(ArgAction::Set)
            .num_args(0..=1);
        o._build();

        assert_eq!(o.to_string(), "--option [<opt>]");
    }

    #[test]
    fn option_display_value_names() {
        let mut o = Arg::new("opt")
            .short('o')
            .action(ArgAction::Set)
            .value_names(["file", "name"]);
        o._build();

        assert_eq!(o.to_string(), "-o <file> <name>");
    }

    #[test]
    fn option_display3() {
        let mut o = Arg::new("opt")
            .short('o')
            .num_args(1..)
            .action(ArgAction::Set)
            .value_names(["file", "name"]);
        o._build();

        assert_eq!(o.to_string(), "-o <file> [name]...");
    }

    #[test]
    fn option_display_partially_optional_values() {
        let mut o = Arg::new("opt")
            .long("example")
            .action(ArgAction::Set)
            .num_args(1..=2)
            .value_names(["FOO", "BAR"]);
        o._build();

        assert_eq!(o.to_string(), "--example <FOO> [BAR]");
    }

    #[test]
    fn option_display_partially_optional_values_require_equals() {
        let mut o = Arg::new("opt")
            .long("example")
            .action(ArgAction::Set)
            .num_args(1..=2)
            .require_equals(true)
            .value_delimiter(',')
            .value_names(["FOO", "BAR"]);
        o._build();

        assert_eq!(o.to_string(), "--example=<FOO> [BAR]");
    }

    #[test]
    fn option_display_partially_optional_values_with_extra_values() {
        let mut o = Arg::new("opt")
            .long("example")
            .action(ArgAction::Set)
            .num_args(1..=3)
            .value_names(["A", "B"]);
        o._build();

        assert_eq!(o.to_string(), "--example <A> [B]...");
    }

    #[test]
    fn option_display_single_alias() {
        let mut o = Arg::new("opt")
            .long("option")
            .action(ArgAction::Set)
            .visible_alias("als");
        o._build();

        assert_eq!(o.to_string(), "--option <opt>");
    }

    #[test]
    fn option_display_multiple_aliases() {
        let mut o = Arg::new("opt")
            .long("option")
            .action(ArgAction::Set)
            .visible_aliases(["als2", "als3", "als4"])
            .alias("als_not_visible");
        o._build();

        assert_eq!(o.to_string(), "--option <opt>");
    }

    #[test]
    fn option_display_single_short_alias() {
        let mut o = Arg::new("opt")
            .short('a')
            .action(ArgAction::Set)
            .visible_short_alias('b');
        o._build();

        assert_eq!(o.to_string(), "-a <opt>");
    }

    #[test]
    fn option_display_multiple_short_aliases() {
        let mut o = Arg::new("opt")
            .short('a')
            .action(ArgAction::Set)
            .visible_short_aliases(['b', 'c', 'd'])
            .short_alias('e');
        o._build();

        assert_eq!(o.to_string(), "-a <opt>");
    }

    // Positionals

    #[test]
    fn positional_display_multiple_values() {
        let mut p = Arg::new("pos").index(1).num_args(1..);
        p._build();

        assert_eq!(p.to_string(), "[pos]...");
    }

    #[test]
    fn positional_display_multiple_values_required() {
        let mut p = Arg::new("pos").index(1).num_args(1..).required(true);
        p._build();

        assert_eq!(p.to_string(), "<pos>...");
    }

    #[test]
    fn positional_display_zero_or_more_values() {
        let mut p = Arg::new("pos").index(1).num_args(0..);
        p._build();

        assert_eq!(p.to_string(), "[pos]...");
    }

    #[test]
    fn positional_display_zero_or_more_values_required() {
        let mut p = Arg::new("pos").index(1).num_args(0..).required(true);
        p._build();

        assert_eq!(p.to_string(), "[pos]...");
    }

    #[test]
    fn positional_display_one_or_more_values() {
        let mut p = Arg::new("pos").index(1).num_args(1..);
        p._build();

        assert_eq!(p.to_string(), "[pos]...");
    }

    #[test]
    fn positional_display_one_or_more_values_required() {
        let mut p = Arg::new("pos").index(1).num_args(1..).required(true);
        p._build();

        assert_eq!(p.to_string(), "<pos>...");
    }

    #[test]
    fn positional_display_optional_value() {
        let mut p = Arg::new("pos")
            .index(1)
            .num_args(0..=1)
            .action(ArgAction::Set);
        p._build();

        assert_eq!(p.to_string(), "[pos]");
    }

    #[test]
    fn positional_display_multiple_occurrences() {
        let mut p = Arg::new("pos").index(1).action(ArgAction::Append);
        p._build();

        assert_eq!(p.to_string(), "[pos]...");
    }

    #[test]
    fn positional_display_multiple_occurrences_required() {
        let mut p = Arg::new("pos")
            .index(1)
            .action(ArgAction::Append)
            .required(true);
        p._build();

        assert_eq!(p.to_string(), "<pos>...");
    }

    #[test]
    fn positional_display_required() {
        let mut p = Arg::new("pos").index(1).required(true);
        p._build();

        assert_eq!(p.to_string(), "<pos>");
    }

    #[test]
    fn positional_display_val_names() {
        let mut p = Arg::new("pos").index(1).value_names(["file1", "file2"]);
        p._build();

        assert_eq!(p.to_string(), "[file1] [file2]");
    }

    #[test]
    fn positional_display_val_names_required() {
        let mut p = Arg::new("pos")
            .index(1)
            .value_names(["file1", "file2"])
            .required(true);
        p._build();

        assert_eq!(p.to_string(), "<file1> <file2>");
    }
}
