// Internal
#[cfg(feature = "suggestions")]
mod similarity;

use crate::builder::Command;

/// Find strings from an iterable of `possible_values` similar to a given value `v`
/// Returns a Vec of all possible values that exceed a similarity threshold
/// sorted by ascending similarity, most similar comes last
#[cfg(feature = "suggestions")]
pub(crate) fn did_you_mean<T, I>(v: &str, possible_values: I) -> Vec<String>
where
    T: AsRef<str>,
    I: IntoIterator<Item = T>,
{
    use std::cmp::Ordering;

    let mut candidates: Vec<(f64, String)> = Vec::new();
    for pv in possible_values {
        // GH #4660: using `jaro` because `jaro_winkler` implementation in `strsim-rs` is wrong
        // causing strings with common prefix >=10 to be considered perfectly similar
        let confidence = similarity::jaro(v, pv.as_ref());

        if confidence > 0.7 {
            let new_elem = (confidence, pv.as_ref().to_owned());
            let pos = candidates
                .binary_search_by(|probe| {
                    if probe.0 > confidence {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                })
                .unwrap_or_else(|e| e);
            candidates.insert(pos, new_elem);
        }
    }

    candidates.into_iter().map(|(_, pv)| pv).collect()
}

#[cfg(not(feature = "suggestions"))]
pub(crate) fn did_you_mean<T, I>(_: &str, _: I) -> Vec<String>
where
    T: AsRef<str>,
    I: IntoIterator<Item = T>,
{
    Vec::new()
}

/// Returns a suffix that can be empty, or is the standard 'did you mean' phrase
pub(crate) fn did_you_mean_flag<'a, 'help, I, T>(
    arg: &str,
    remaining_args: &[&std::ffi::OsStr],
    longs: I,
    subcommands: impl IntoIterator<Item = &'a mut Command>,
) -> Option<(String, Option<String>)>
where
    'help: 'a,
    T: AsRef<str>,
    I: IntoIterator<Item = T>,
{
    use crate::mkeymap::KeyType;

    match did_you_mean(arg, longs).pop() {
        Some(candidate) => Some((candidate, None)),
        None => subcommands
            .into_iter()
            .filter_map(|subcommand| {
                subcommand._build_self(false);

                let longs = subcommand.get_keymap().keys().filter_map(|a| {
                    if let KeyType::Long(v) = a {
                        Some(v.to_string_lossy().into_owned())
                    } else {
                        None
                    }
                });

                let subcommand_name = subcommand.get_name();

                let candidate = some!(did_you_mean(arg, longs).pop());
                let score = some!(remaining_args.iter().position(|x| subcommand_name == *x));
                Some((score, (candidate, Some(subcommand_name.to_owned()))))
            })
            .min_by_key(|(x, _)| *x)
            .map(|(_, suggestion)| suggestion),
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/command-line/builder/unit/src/parser/features/suggestions/all_test_88.rs"
));
