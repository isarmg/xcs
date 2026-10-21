#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::version_and_date_from_rustc_version;
    use super::version_and_date_from_rustc_verbose_version;

    macro_rules! check_parse {
        (@ $f:expr, $s:expr => $v:expr, $d:expr) => ({
            if let (Some(v), d) = $f(&$s) {
                let e_d: Option<&str> = $d.into();
                assert_eq!((v, d), ($v.to_string(), e_d.map(|s| s.into())));
            } else {
                panic!("{:?} didn't parse for version testing.", $s);
            }
        });
        ($f:expr, $s:expr => $v:expr, $d:expr) => ({
            let warn = "warning: invalid logging spec 'warning', ignoring it";
            let warn2 = "warning: sorry, something went wrong :(sad)";
            check_parse!(@ $f, $s => $v, $d);
            check_parse!(@ $f, &format!("{}\n{}", warn, $s) => $v, $d);
            check_parse!(@ $f, &format!("{}\n{}", warn2, $s) => $v, $d);
            check_parse!(@ $f, &format!("{}\n{}\n{}", warn, warn2, $s) => $v, $d);
            check_parse!(@ $f, &format!("{}\n{}\n{}", warn2, warn, $s) => $v, $d);
        })
    }

    macro_rules! check_terse_parse {
        ($($s:expr => $v:expr, $d:expr,)+) => {$(
            check_parse!(version_and_date_from_rustc_version, $s => $v, $d);
        )+}
    }

    macro_rules! check_verbose_parse {
        ($($s:expr => $v:expr, $d:expr,)+) => {$(
            check_parse!(version_and_date_from_rustc_verbose_version, $s => $v, $d);
        )+}
    }

    #[test]
    fn test_version_parse() {
        check_terse_parse! {
            "rustc 1.18.0" => "1.18.0", None,
            "rustc 1.8.0" => "1.8.0", None,
            "rustc 1.20.0-nightly" => "1.20.0-nightly", None,
            "rustc 1.20" => "1.20", None,
            "rustc 1.3" => "1.3", None,
            "rustc 1" => "1", None,
            "rustc 1.5.1-beta" => "1.5.1-beta", None,
            "rustc 1.20.0 (2017-07-09)" => "1.20.0", Some("2017-07-09"),
            "rustc 1.20.0-dev (2017-07-09)" => "1.20.0-dev", Some("2017-07-09"),
            "rustc 1.20.0-nightly (d84693b93 2017-07-09)" => "1.20.0-nightly", Some("2017-07-09"),
            "rustc 1.20.0 (d84693b93 2017-07-09)" => "1.20.0", Some("2017-07-09"),
            "rustc 1.30.0-nightly (3bc2ca7e4 2018-09-20)" => "1.30.0-nightly", Some("2018-09-20"),
        };
    }

    #[test]
    fn test_verbose_version_parse() {
        check_verbose_parse! {
            "rustc 1.0.0 (a59de37e9 2015-05-13) (built 2015-05-14)\n\
                binary: rustc\n\
                commit-hash: a59de37e99060162a2674e3ff45409ac73595c0e\n\
                commit-date: 2015-05-13\n\
                build-date: 2015-05-14\n\
                host: x86_64-unknown-linux-gnu\n\
                release: 1.0.0" => "1.0.0", Some("2015-05-13"),

            "rustc 1.0.0 (a59de37e9 2015-05-13) (built 2015-05-14)\n\
                commit-hash: a59de37e99060162a2674e3ff45409ac73595c0e\n\
                commit-date: 2015-05-13\n\
                build-date: 2015-05-14\n\
                host: x86_64-unknown-linux-gnu\n\
                release: 1.0.0" => "1.0.0", Some("2015-05-13"),

            "rustc 1.50.0 (cb75ad5db 2021-02-10)\n\
                binary: rustc\n\
                commit-hash: cb75ad5db02783e8b0222fee363c5f63f7e2cf5b\n\
                commit-date: 2021-02-10\n\
                host: x86_64-unknown-linux-gnu\n\
                release: 1.50.0" => "1.50.0", Some("2021-02-10"),

            "rustc 1.52.0-nightly (234781afe 2021-03-07)\n\
                binary: rustc\n\
                commit-hash: 234781afe33d3f339b002f85f948046d8476cfc9\n\
                commit-date: 2021-03-07\n\
                host: x86_64-unknown-linux-gnu\n\
                release: 1.52.0-nightly\n\
                LLVM version: 12.0.0" => "1.52.0-nightly", Some("2021-03-07"),

            "rustc 1.41.1\n\
                binary: rustc\n\
                commit-hash: unknown\n\
                commit-date: unknown\n\
                host: x86_64-unknown-linux-gnu\n\
                release: 1.41.1\n\
                LLVM version: 7.0" => "1.41.1", None,

            "rustc 1.49.0\n\
                binary: rustc\n\
                commit-hash: unknown\n\
                commit-date: unknown\n\
                host: x86_64-unknown-linux-gnu\n\
                release: 1.49.0" => "1.49.0", None,

            "rustc 1.50.0 (Fedora 1.50.0-1.fc33)\n\
                binary: rustc\n\
                commit-hash: unknown\n\
                commit-date: unknown\n\
                host: x86_64-unknown-linux-gnu\n\
                release: 1.50.0" => "1.50.0", None,
        };
    }

    fn read_static(verbose: bool, channel: &str, minor: usize) -> String {
        use std::fs::File;
        use std::path::Path;
        use std::io::{BufReader, Read};

        let subdir = if verbose { "verbose" } else { "terse" };
        let path = Path::new(STATIC_PATH)
            .join(channel)
            .join(subdir)
            .join(format!("rustc-1.{}.0", minor));

        let file = File::open(path).unwrap();
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents).unwrap();
        contents
    }

    static STATIC_PATH: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/static");

    static DATES: [&'static str; 51] = [
        "2015-05-13", "2015-06-19", "2015-08-03", "2015-09-15", "2015-10-27",
        "2015-12-04", "2016-01-19", "2016-02-29", "2016-04-11", "2016-05-18",
        "2016-07-03", "2016-08-15", "2016-09-23", "2016-11-07", "2016-12-16",
        "2017-01-19", "2017-03-10", "2017-04-24", "2017-06-06", "2017-07-17",
        "2017-08-27", "2017-10-09", "2017-11-20", "2018-01-01", "2018-02-12",
        "2018-03-25", "2018-05-07", "2018-06-19", "2018-07-30", "2018-09-11",
        "2018-10-24", "2018-12-04", "2019-01-16", "2019-02-28", "2019-04-10",
        "2019-05-20", "2019-07-03", "2019-08-13", "2019-09-23", "2019-11-04",
        "2019-12-16", "2020-01-27", "2020-03-09", "2020-04-20", "2020-06-01",
        "2020-07-13", "2020-08-24", "2020-10-07", "2020-11-16", "2020-12-29",
        "2021-02-10",
    ];

    #[test]
    fn test_stable_compatibility() {
        if env::var_os("FORCE_STATIC").is_none() && fs::metadata(STATIC_PATH).is_err() {
            // We exclude `/static` when we package `version_check`, so don't
            // run if static files aren't present unless we know they should be.
            return;
        }

        // Ensure we can parse all output from all Linux stable releases.
        for v in 0..DATES.len() {
            let (version, date) = (&format!("1.{}.0", v), Some(DATES[v]));
            check_terse_parse!(read_static(false, "stable", v) => version, date,);
            check_verbose_parse!(read_static(true, "stable", v) => version, date,);
        }
    }

    #[test]
    fn test_parse_current() {
        let (version, channel) = (::Version::read(), ::Channel::read());
        assert!(version.is_some());
        assert!(channel.is_some());

        if let Ok(known_channel) = env::var("KNOWN_CHANNEL") {
            assert_eq!(channel, ::Channel::parse(&known_channel));
        }
    }
}
