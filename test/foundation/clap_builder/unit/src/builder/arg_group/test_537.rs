#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn groups() {
        let g = ArgGroup::new("test")
            .arg("a1")
            .arg("a4")
            .args(["a2", "a3"])
            .required(true)
            .conflicts_with("c1")
            .conflicts_with_all(["c2", "c3"])
            .conflicts_with("c4")
            .requires("r1")
            .requires_all(["r2", "r3"])
            .requires("r4");

        let args: Vec<Id> = vec!["a1".into(), "a4".into(), "a2".into(), "a3".into()];
        let reqs: Vec<Id> = vec!["r1".into(), "r2".into(), "r3".into(), "r4".into()];
        let confs: Vec<Id> = vec!["c1".into(), "c2".into(), "c3".into(), "c4".into()];

        assert_eq!(g.args, args);
        assert_eq!(g.requires, reqs);
        assert_eq!(g.conflicts, confs);
    }

    #[test]
    fn test_from() {
        let g = ArgGroup::new("test")
            .arg("a1")
            .arg("a4")
            .args(["a2", "a3"])
            .required(true)
            .conflicts_with("c1")
            .conflicts_with_all(["c2", "c3"])
            .conflicts_with("c4")
            .requires("r1")
            .requires_all(["r2", "r3"])
            .requires("r4");

        let args: Vec<Id> = vec!["a1".into(), "a4".into(), "a2".into(), "a3".into()];
        let reqs: Vec<Id> = vec!["r1".into(), "r2".into(), "r3".into(), "r4".into()];
        let confs: Vec<Id> = vec!["c1".into(), "c2".into(), "c3".into(), "c4".into()];

        let g2 = ArgGroup::from(&g);
        assert_eq!(g2.args, args);
        assert_eq!(g2.requires, reqs);
        assert_eq!(g2.conflicts, confs);
    }

    // This test will *fail to compile* if ArgGroup is not Send + Sync
    #[test]
    fn arg_group_send_sync() {
        fn foo<T: Send + Sync>(_: T) {}
        foo(ArgGroup::new("test"));
    }

    #[test]
    fn arg_group_expose_is_multiple_helper() {
        let args: Vec<Id> = vec!["a1".into(), "a4".into()];

        let mut grp_multiple = ArgGroup::new("test_multiple").args(&args).multiple(true);
        assert!(grp_multiple.is_multiple());

        let mut grp_not_multiple = ArgGroup::new("test_multiple").args(&args).multiple(false);
        assert!(!grp_not_multiple.is_multiple());
    }

    #[test]
    fn arg_group_expose_get_args_helper() {
        let args: Vec<Id> = vec!["a1".into(), "a4".into()];
        let grp = ArgGroup::new("program").args(&args);

        for (pos, arg) in grp.get_args().enumerate() {
            assert_eq!(*arg, args[pos]);
        }
    }
}
