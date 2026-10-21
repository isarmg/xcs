#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn snapshot(vars: &[(&str, &str)]) -> EnvSnapshot {
        EnvSnapshot::from_pairs(vars)
    }

    fn envs(cmd: &Command) -> Vec<(OsString, Option<OsString>)> {
        cmd.get_envs()
            .map(|(key, value)| (key.to_owned(), value.map(OsStr::to_owned)))
            .collect()
    }

    #[test]
    fn apply_replaces_the_whole_environment() {
        let mut cmd = Command::new("cc");
        cmd.env("CC_TEST_SET_BEFORE", "1")
            .env_remove("CC_TEST_REMOVED_BEFORE");
        snapshot(&[("CC_TEST_IN_SNAPSHOT", "1")]).apply(&mut cmd);
        assert_eq!(
            envs(&cmd),
            [("CC_TEST_IN_SNAPSHOT".into(), Some("1".into()))]
        );
    }

    #[test]
    fn build_env_wins_over_snapshot() {
        let env = BuildEnv::with_inherited(
            snapshot(&[("CC_TEST_ORDER", "inherited")]),
            vec![(
                OsStr::new("CC_TEST_ORDER").into(),
                OsStr::new("explicit").into(),
            )],
        );
        let mut cmd = Command::new("cc");
        env.apply(&mut cmd);
        assert_eq!(
            envs(&cmd),
            [("CC_TEST_ORDER".into(), Some("explicit".into()))]
        );
    }

    #[test]
    fn snapshot_lookup_shares_the_value() {
        let snapshot = snapshot(&[("CC_TEST_SHARED", "a")]);
        let value = snapshot.get(OsStr::new("CC_TEST_SHARED")).unwrap();
        assert!(Arc::ptr_eq(value, &snapshot.vars[0].1));
    }

    #[test]
    fn snapshot_lookup_follows_the_platform_case_rule() {
        let snapshot = snapshot(&[("Path", "a")]);
        assert_eq!(
            snapshot.get(OsStr::new("Path")).map(|value| &**value),
            Some(OsStr::new("a"))
        );
        let other_case = snapshot.get(OsStr::new("PATH")).map(|value| &**value);
        if cfg!(windows) {
            assert_eq!(other_case, Some(OsStr::new("a")));
        } else {
            assert_eq!(other_case, None);
        }
    }

    #[test]
    fn set_inherited_replaces_a_taken_snapshot() {
        let mut env = BuildEnv::default();
        env.inherited();
        env.set_inherited(snapshot(&[("CC_TEST_REPLACED", "1")]));
        assert_eq!(env.inherited(), &snapshot(&[("CC_TEST_REPLACED", "1")]));
    }

    /// A variable given twice takes its last value, in cc's own lookups as in
    /// the tools it runs.
    #[test]
    fn snapshot_from_vars_keeps_the_last_value() {
        let snapshot = EnvSnapshot::from_vars(
            &mut [
                ("CC_TEST_TWICE", "first"),
                ("Path", "a"),
                ("CC_TEST_TWICE", "last"),
                ("PATH", "b"),
            ]
            .into_iter()
            .map(|(key, value)| (OsStr::new(key).into(), OsStr::new(value).into())),
        );
        assert_eq!(
            snapshot
                .get(OsStr::new("CC_TEST_TWICE"))
                .map(|value| &**value),
            Some(OsStr::new("last"))
        );
        let mut cmd = Command::new("cc");
        snapshot.apply(&mut cmd);
        for key in ["CC_TEST_TWICE", "Path", "PATH"] {
            let key = OsStr::new(key);
            let in_child = cmd
                .get_envs()
                .filter(|(k, _)| is_same_key(k, key))
                .last()
                .and_then(|(_, value)| value);
            assert_eq!(snapshot.get(key).map(|value| &**value), in_child, "{key:?}");
        }
    }

    #[test]
    fn snapshot_comparison_follows_the_variables() {
        use std::cmp::Ordering;
        use std::collections::hash_map::DefaultHasher;

        fn hash(snapshot: &EnvSnapshot) -> u64 {
            let mut hasher = DefaultHasher::new();
            snapshot.hash(&mut hasher);
            hasher.finish()
        }

        let a = snapshot(&[("CC_TEST_A", "1"), ("CC_TEST_B", "2")]);
        // Same variables, captured separately.
        let same = snapshot(&[("CC_TEST_A", "1"), ("CC_TEST_B", "2")]);
        assert!(!Arc::ptr_eq(&a.vars, &same.vars));
        assert_eq!(a, same);
        assert_eq!(a.cmp(&same), Ordering::Equal);
        assert_eq!(hash(&a), hash(&same));
        assert_eq!(a, a.clone());
        assert_eq!(a.cmp(&a.clone()), Ordering::Equal);

        for other in [
            snapshot(&[("CC_TEST_A", "1")]),
            snapshot(&[("CC_TEST_A", "1"), ("CC_TEST_B", "3")]),
            // A boundary moved between name and value.
            snapshot(&[("CC_TEST_A", "1"), ("CC_TEST_B2", "")]),
            // Names are compared exactly, also on Windows.
            snapshot(&[("CC_TEST_A", "1"), ("cc_test_b", "2")]),
        ] {
            assert_ne!(a, other);
            assert_eq!(a.cmp(&other), a.vars.cmp(&other.vars));
            assert_ne!(a.cmp(&other), Ordering::Equal);
            assert_eq!(a.cmp(&other), other.cmp(&a).reverse());
        }
    }

    #[test]
    fn snapshot_debug_hides_values() {
        let debug = format!("{:?}", snapshot(&[("CC_TEST_SECRET", "hunter2")]));
        assert!(!debug.contains("hunter2"), "{debug}");
    }
}
