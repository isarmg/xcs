#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_env_wins_over_inherited_env() {
        let mut tool = Tool::with_family(
            "cc".into(),
            ToolFamily::Gnu,
            EnvSnapshot::from_pairs(&[("CC_TEST_ORDER", "inherited")]),
        );
        tool.env.push((
            OsStr::new("CC_TEST_ORDER").into(),
            OsStr::new("tool").into(),
        ));
        let cmd = tool.to_command();
        let envs: Vec<_> = cmd.get_envs().collect();
        assert_eq!(
            envs,
            [(OsStr::new("CC_TEST_ORDER"), Some(OsStr::new("tool")))]
        );
    }

    /// Two entries for one variable, so that the order matters.
    const ENV: [(&str, &str); 3] = [
        ("CC_TEST_B", "first"),
        ("CC_TEST_A", "a"),
        ("CC_TEST_B", "last"),
    ];

    fn tool_with_env() -> Tool {
        let mut tool =
            Tool::with_family("cc".into(), ToolFamily::Gnu, EnvSnapshot::from_pairs(&[]));
        tool.env = ENV
            .iter()
            .map(|(key, value)| (OsStr::new(key).into(), OsStr::new(value).into()))
            .collect();
        tool
    }

    #[test]
    fn get_envs_yields_env_in_order() {
        let tool = tool_with_env();
        let expected: Vec<_> = ENV
            .iter()
            .map(|(key, value)| (OsStr::new(key), OsStr::new(value)))
            .collect();

        let mut envs = tool.get_envs();
        assert_eq!(envs.size_hint(), (3, Some(3)));
        envs.next();
        assert_eq!(envs.len(), 2);

        assert_eq!(tool.get_envs().collect::<Vec<_>>(), expected);
        assert!(tool.get_envs().rev().eq(expected.into_iter().rev()));
    }

    #[test]
    #[allow(deprecated)]
    fn env_returns_a_copy_of_env_in_order() {
        let tool = tool_with_env();
        let expected: Vec<(OsString, OsString)> = ENV
            .iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();

        let cloned_before = tool.clone();
        assert_eq!(tool.env(), expected);
        assert!(std::ptr::eq(tool.env(), tool.env()));
        let cloned_after = tool.clone();
        assert_eq!(cloned_before.env(), expected);
        assert_eq!(cloned_after.env(), expected);
    }

    #[test]
    fn tool_keeps_auto_traits() {
        use std::panic::{RefUnwindSafe, UnwindSafe};

        fn assert_auto_traits<T: Clone + Send + Sync + Unpin + UnwindSafe + RefUnwindSafe>() {}
        assert_auto_traits::<Tool>();
    }

    #[test]
    fn command_key_tells_commands_apart() {
        fn key(command: &[&OsStr]) -> CompilerCommandKey {
            CompilerCommandKey::new(
                &mut command.iter().copied(),
                EnvSnapshot::from_pairs(&[]),
                Box::new([]),
            )
        }
        let os = OsStr::new;

        assert_eq!(key(&[os("cc"), os("-O2")]), key(&[os("cc"), os("-O2")]));

        for (a, b) in [
            (
                &[os("cc"), os("ab"), os("c")][..],
                &[os("cc"), os("a"), os("bc")][..],
            ),
            (&[os("cc"), os("")], &[os("cc")]),
            (&[os("cc"), os(""), os("")], &[os("cc"), os("")]),
            (&[os("cc-O2")], &[os("cc"), os("-O2")]),
        ] {
            assert_ne!(key(a), key(b), "{a:?} vs {b:?}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn command_key_tells_unpaired_surrogates_apart() {
        use std::os::windows::ffi::OsStringExt;

        let key = |command: &[OsString]| {
            CompilerCommandKey::new(
                &mut command.iter().map(OsString::as_os_str),
                EnvSnapshot::from_pairs(&[]),
                Box::new([]),
            )
        };
        // A lead and a trail surrogate in two arguments, and the pair they
        // would make as one.
        let lead = OsString::from_wide(&[0xD83D]);
        let trail = OsString::from_wide(&[0xDE00]);
        let pair = OsString::from_wide(&[0xD83D, 0xDE00]);
        assert_ne!(key(&[lead.clone(), trail.clone()]), key(&[pair]));
        assert_eq!(key(&[lead.clone(), trail.clone()]), key(&[lead, trail]));
    }
}
