// Take a look at the license at the top of the repository in the LICENSE file.

mod docs;
mod headers;
mod signals;
mod utils;

use std::path::Path;
use utils::TestResult;

#[allow(clippy::type_complexity)]
const CHECKS: &[(fn(&str, &Path) -> TestResult, &[&str])] = &[
    (headers::check_license_header, &["src", "tests", "examples"]),
    (signals::check_signals, &["src"]),
    (docs::check_docs, &["src"]),
];

fn handle_tests(res: &mut [TestResult]) {
    utils::read_dirs(
        &["benches", "examples", "src", "../../../test/foundation/sysinfo/tests"],
        &mut |p: &Path, c: &str| {
            let logical_path = p.strip_prefix("../../../test/foundation/sysinfo").unwrap_or(p);
            if let Some(first) = logical_path.iter().next().and_then(|first| first.to_str()) {
                for (pos, (check, filter)) in CHECKS.iter().enumerate() {
                    if filter.contains(&first) {
                        res[pos] += check(c, p);
                    }
                }
            }
        },
    );
}

#[test]
fn code_checks() {
    let mut res = Vec::new();

    for _ in CHECKS {
        res.push(TestResult {
            nb_tests: 0,
            nb_errors: 0,
        });
    }

    handle_tests(&mut res);

    for r in res {
        assert_eq!(r.nb_errors, 0);
        assert_ne!(r.nb_tests, 0);
    }
}
