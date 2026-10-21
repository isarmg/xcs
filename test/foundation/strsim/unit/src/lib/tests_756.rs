#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_delta {
        ($x:expr, $y:expr) => {
            assert_delta!($x, $y, 1e-5);
        };
        ($x:expr, $y:expr, $d:expr) => {
            if ($x - $y).abs() > $d {
                panic!(
                    "assertion failed: actual: `{}`, expected: `{}`: \
                    actual not within < {} of expected",
                    $x, $y, $d
                );
            }
        };
    }

    #[test]
    fn bigrams_iterator() {
        let mut bi = bigrams("abcde");

        assert_eq!(Some(('a', 'b')), bi.next());
        assert_eq!(Some(('b', 'c')), bi.next());
        assert_eq!(Some(('c', 'd')), bi.next());
        assert_eq!(Some(('d', 'e')), bi.next());
        assert_eq!(None, bi.next());
    }

    fn assert_hamming_dist(dist: usize, str1: &str, str2: &str) {
        assert_eq!(Ok(dist), hamming(str1, str2));
    }

    #[test]
    fn hamming_empty() {
        assert_hamming_dist(0, "", "")
    }

    #[test]
    fn hamming_same() {
        assert_hamming_dist(0, "hamming", "hamming")
    }

    #[test]
    fn hamming_numbers() {
        assert_eq!(Ok(1), generic_hamming(&[1, 2, 4], &[1, 2, 3]));
    }

    #[test]
    fn hamming_diff() {
        assert_hamming_dist(3, "hamming", "hammers")
    }

    #[test]
    fn hamming_diff_multibyte() {
        assert_hamming_dist(2, "hamming", "h香mmüng");
    }

    #[test]
    fn hamming_unequal_length() {
        assert_eq!(
            Err(StrSimError::DifferentLengthArgs),
            generic_hamming("ham".chars(), "hamming".chars())
        );
    }

    #[test]
    fn hamming_names() {
        assert_hamming_dist(14, "Friedrich Nietzs", "Jean-Paul Sartre")
    }

    #[test]
    fn jaro_both_empty() {
        assert_eq!(1.0, jaro("", ""));
    }

    #[test]
    fn jaro_first_empty() {
        assert_eq!(0.0, jaro("", "jaro"));
    }

    #[test]
    fn jaro_second_empty() {
        assert_eq!(0.0, jaro("distance", ""));
    }

    #[test]
    fn jaro_same() {
        assert_eq!(1.0, jaro("jaro", "jaro"));
    }

    #[test]
    fn jaro_multibyte() {
        assert_delta!(0.818, jaro("testabctest", "testöঙ香test"), 0.001);
        assert_delta!(0.818, jaro("testöঙ香test", "testabctest"), 0.001);
    }

    #[test]
    fn jaro_diff_short() {
        assert_delta!(0.767, jaro("dixon", "dicksonx"), 0.001);
    }

    #[test]
    fn jaro_diff_one_character() {
        assert_eq!(0.0, jaro("a", "b"));
    }

    #[test]
    fn jaro_same_one_character() {
        assert_eq!(1.0, jaro("a", "a"));
    }

    #[test]
    fn generic_jaro_diff() {
        assert_eq!(0.0, generic_jaro(&[1, 2], &[3, 4]));
    }

    #[test]
    fn jaro_diff_one_and_two() {
        assert_delta!(0.83, jaro("a", "ab"), 0.01);
    }

    #[test]
    fn jaro_diff_two_and_one() {
        assert_delta!(0.83, jaro("ab", "a"), 0.01);
    }

    #[test]
    fn jaro_diff_no_transposition() {
        assert_delta!(0.822, jaro("dwayne", "duane"), 0.001);
    }

    #[test]
    fn jaro_diff_with_transposition() {
        assert_delta!(0.944, jaro("martha", "marhta"), 0.001);
        assert_delta!(0.6, jaro("a jke", "jane a k"), 0.001);
    }

    #[test]
    fn jaro_names() {
        assert_delta!(
            0.392,
            jaro("Friedrich Nietzsche", "Jean-Paul Sartre"),
            0.001
        );
    }

    #[test]
    fn jaro_winkler_both_empty() {
        assert_eq!(1.0, jaro_winkler("", ""));
    }

    #[test]
    fn jaro_winkler_first_empty() {
        assert_eq!(0.0, jaro_winkler("", "jaro-winkler"));
    }

    #[test]
    fn jaro_winkler_second_empty() {
        assert_eq!(0.0, jaro_winkler("distance", ""));
    }

    #[test]
    fn jaro_winkler_same() {
        assert_eq!(1.0, jaro_winkler("Jaro-Winkler", "Jaro-Winkler"));
    }

    #[test]
    fn jaro_winkler_multibyte() {
        assert_delta!(0.89, jaro_winkler("testabctest", "testöঙ香test"), 0.001);
        assert_delta!(0.89, jaro_winkler("testöঙ香test", "testabctest"), 0.001);
    }

    #[test]
    fn jaro_winkler_diff_short() {
        assert_delta!(0.813, jaro_winkler("dixon", "dicksonx"), 0.001);
        assert_delta!(0.813, jaro_winkler("dicksonx", "dixon"), 0.001);
    }

    #[test]
    fn jaro_winkler_diff_one_character() {
        assert_eq!(0.0, jaro_winkler("a", "b"));
    }

    #[test]
    fn jaro_winkler_same_one_character() {
        assert_eq!(1.0, jaro_winkler("a", "a"));
    }

    #[test]
    fn jaro_winkler_diff_no_transposition() {
        assert_delta!(0.84, jaro_winkler("dwayne", "duane"), 0.001);
    }

    #[test]
    fn jaro_winkler_diff_with_transposition() {
        assert_delta!(0.961, jaro_winkler("martha", "marhta"), 0.001);
        assert_delta!(0.6, jaro_winkler("a jke", "jane a k"), 0.001);
    }

    #[test]
    fn jaro_winkler_names() {
        assert_delta!(
            0.452,
            jaro_winkler("Friedrich Nietzsche", "Fran-Paul Sartre"),
            0.001
        );
    }

    #[test]
    fn jaro_winkler_long_prefix() {
        assert_delta!(0.866, jaro_winkler("cheeseburger", "cheese fries"), 0.001);
    }

    #[test]
    fn jaro_winkler_more_names() {
        assert_delta!(0.868, jaro_winkler("Thorkel", "Thorgier"), 0.001);
    }

    #[test]
    fn jaro_winkler_length_of_one() {
        assert_delta!(0.738, jaro_winkler("Dinsdale", "D"), 0.001);
    }

    #[test]
    fn jaro_winkler_very_long_prefix() {
        assert_delta!(
            0.98519,
            jaro_winkler("thequickbrownfoxjumpedoverx", "thequickbrownfoxjumpedovery")
        );
    }

    #[test]
    fn levenshtein_empty() {
        assert_eq!(0, levenshtein("", ""));
    }

    #[test]
    fn levenshtein_same() {
        assert_eq!(0, levenshtein("levenshtein", "levenshtein"));
    }

    #[test]
    fn levenshtein_diff_short() {
        assert_eq!(3, levenshtein("kitten", "sitting"));
    }

    #[test]
    fn levenshtein_diff_with_space() {
        assert_eq!(5, levenshtein("hello, world", "bye, world"));
    }

    #[test]
    fn levenshtein_diff_multibyte() {
        assert_eq!(3, levenshtein("öঙ香", "abc"));
        assert_eq!(3, levenshtein("abc", "öঙ香"));
    }

    #[test]
    fn levenshtein_diff_longer() {
        let a = "The quick brown fox jumped over the angry dog.";
        let b = "Lorem ipsum dolor sit amet, dicta latine an eam.";
        assert_eq!(37, levenshtein(a, b));
    }

    #[test]
    fn levenshtein_first_empty() {
        assert_eq!(7, levenshtein("", "sitting"));
    }

    #[test]
    fn levenshtein_second_empty() {
        assert_eq!(6, levenshtein("kitten", ""));
    }

    #[test]
    fn normalized_levenshtein_diff_short() {
        assert_delta!(0.57142, normalized_levenshtein("kitten", "sitting"));
    }

    #[test]
    fn normalized_levenshtein_for_empty_strings() {
        assert_delta!(1.0, normalized_levenshtein("", ""));
    }

    #[test]
    fn normalized_levenshtein_first_empty() {
        assert_delta!(0.0, normalized_levenshtein("", "second"));
    }

    #[test]
    fn normalized_levenshtein_second_empty() {
        assert_delta!(0.0, normalized_levenshtein("first", ""));
    }

    #[test]
    fn normalized_levenshtein_identical_strings() {
        assert_delta!(1.0, normalized_levenshtein("identical", "identical"));
    }

    #[test]
    fn osa_distance_empty() {
        assert_eq!(0, osa_distance("", ""));
    }

    #[test]
    fn osa_distance_same() {
        assert_eq!(0, osa_distance("damerau", "damerau"));
    }

    #[test]
    fn osa_distance_first_empty() {
        assert_eq!(7, osa_distance("", "damerau"));
    }

    #[test]
    fn osa_distance_second_empty() {
        assert_eq!(7, osa_distance("damerau", ""));
    }

    #[test]
    fn osa_distance_diff() {
        assert_eq!(3, osa_distance("ca", "abc"));
    }

    #[test]
    fn osa_distance_diff_short() {
        assert_eq!(3, osa_distance("damerau", "aderua"));
    }

    #[test]
    fn osa_distance_diff_reversed() {
        assert_eq!(3, osa_distance("aderua", "damerau"));
    }

    #[test]
    fn osa_distance_diff_multibyte() {
        assert_eq!(3, osa_distance("öঙ香", "abc"));
        assert_eq!(3, osa_distance("abc", "öঙ香"));
    }

    #[test]
    fn osa_distance_diff_unequal_length() {
        assert_eq!(6, osa_distance("damerau", "aderuaxyz"));
    }

    #[test]
    fn osa_distance_diff_unequal_length_reversed() {
        assert_eq!(6, osa_distance("aderuaxyz", "damerau"));
    }

    #[test]
    fn osa_distance_diff_comedians() {
        assert_eq!(5, osa_distance("Stewart", "Colbert"));
    }

    #[test]
    fn osa_distance_many_transpositions() {
        assert_eq!(4, osa_distance("abcdefghijkl", "bacedfgihjlk"));
    }

    #[test]
    fn osa_distance_diff_longer() {
        let a = "The quick brown fox jumped over the angry dog.";
        let b = "Lehem ipsum dolor sit amet, dicta latine an eam.";
        assert_eq!(36, osa_distance(a, b));
    }

    #[test]
    fn osa_distance_beginning_transposition() {
        assert_eq!(1, osa_distance("foobar", "ofobar"));
    }

    #[test]
    fn osa_distance_end_transposition() {
        assert_eq!(1, osa_distance("specter", "spectre"));
    }

    #[test]
    fn osa_distance_restricted_edit() {
        assert_eq!(4, osa_distance("a cat", "an abct"));
    }

    #[test]
    fn damerau_levenshtein_empty() {
        assert_eq!(0, damerau_levenshtein("", ""));
    }

    #[test]
    fn damerau_levenshtein_same() {
        assert_eq!(0, damerau_levenshtein("damerau", "damerau"));
    }

    #[test]
    fn damerau_levenshtein_first_empty() {
        assert_eq!(7, damerau_levenshtein("", "damerau"));
    }

    #[test]
    fn damerau_levenshtein_second_empty() {
        assert_eq!(7, damerau_levenshtein("damerau", ""));
    }

    #[test]
    fn damerau_levenshtein_diff() {
        assert_eq!(2, damerau_levenshtein("ca", "abc"));
    }

    #[test]
    fn damerau_levenshtein_diff_short() {
        assert_eq!(3, damerau_levenshtein("damerau", "aderua"));
    }

    #[test]
    fn damerau_levenshtein_diff_reversed() {
        assert_eq!(3, damerau_levenshtein("aderua", "damerau"));
    }

    #[test]
    fn damerau_levenshtein_diff_multibyte() {
        assert_eq!(3, damerau_levenshtein("öঙ香", "abc"));
        assert_eq!(3, damerau_levenshtein("abc", "öঙ香"));
    }

    #[test]
    fn damerau_levenshtein_diff_unequal_length() {
        assert_eq!(6, damerau_levenshtein("damerau", "aderuaxyz"));
    }

    #[test]
    fn damerau_levenshtein_diff_unequal_length_reversed() {
        assert_eq!(6, damerau_levenshtein("aderuaxyz", "damerau"));
    }

    #[test]
    fn damerau_levenshtein_diff_comedians() {
        assert_eq!(5, damerau_levenshtein("Stewart", "Colbert"));
    }

    #[test]
    fn damerau_levenshtein_many_transpositions() {
        assert_eq!(4, damerau_levenshtein("abcdefghijkl", "bacedfgihjlk"));
    }

    #[test]
    fn damerau_levenshtein_diff_longer() {
        let a = "The quick brown fox jumped over the angry dog.";
        let b = "Lehem ipsum dolor sit amet, dicta latine an eam.";
        assert_eq!(36, damerau_levenshtein(a, b));
    }

    #[test]
    fn damerau_levenshtein_beginning_transposition() {
        assert_eq!(1, damerau_levenshtein("foobar", "ofobar"));
    }

    #[test]
    fn damerau_levenshtein_end_transposition() {
        assert_eq!(1, damerau_levenshtein("specter", "spectre"));
    }

    #[test]
    fn damerau_levenshtein_unrestricted_edit() {
        assert_eq!(3, damerau_levenshtein("a cat", "an abct"));
    }

    #[test]
    fn normalized_damerau_levenshtein_diff_short() {
        assert_delta!(
            0.27272,
            normalized_damerau_levenshtein("levenshtein", "löwenbräu")
        );
    }

    #[test]
    fn normalized_damerau_levenshtein_for_empty_strings() {
        assert_delta!(1.0, normalized_damerau_levenshtein("", ""));
    }

    #[test]
    fn normalized_damerau_levenshtein_first_empty() {
        assert_delta!(0.0, normalized_damerau_levenshtein("", "flower"));
    }

    #[test]
    fn normalized_damerau_levenshtein_second_empty() {
        assert_delta!(0.0, normalized_damerau_levenshtein("tree", ""));
    }

    #[test]
    fn normalized_damerau_levenshtein_identical_strings() {
        assert_delta!(
            1.0,
            normalized_damerau_levenshtein("sunglasses", "sunglasses")
        );
    }

    #[test]
    fn sorensen_dice_all() {
        // test cases taken from
        // https://github.com/aceakash/string-similarity/blob/f83ba3cd7bae874c20c429774e911ae8cff8bced/src/spec/index.spec.js#L11

        assert_delta!(1.0, sorensen_dice("a", "a"));
        assert_delta!(0.0, sorensen_dice("a", "b"));
        assert_delta!(1.0, sorensen_dice("", ""));
        assert_delta!(0.0, sorensen_dice("a", ""));
        assert_delta!(0.0, sorensen_dice("", "a"));
        assert_delta!(1.0, sorensen_dice("apple event", "apple    event"));
        assert_delta!(0.90909, sorensen_dice("iphone", "iphone x"));
        assert_delta!(0.0, sorensen_dice("french", "quebec"));
        assert_delta!(1.0, sorensen_dice("france", "france"));
        assert_delta!(0.2, sorensen_dice("fRaNce", "france"));
        assert_delta!(0.8, sorensen_dice("healed", "sealed"));
        assert_delta!(
            0.78788,
            sorensen_dice("web applications", "applications of the web")
        );
        assert_delta!(
            0.92,
            sorensen_dice(
                "this will have a typo somewhere",
                "this will huve a typo somewhere"
            )
        );
        assert_delta!(
            0.60606,
            sorensen_dice(
                "Olive-green table for sale, in extremely good condition.",
                "For sale: table in very good  condition, olive green in colour."
            )
        );
        assert_delta!(
            0.25581,
            sorensen_dice(
                "Olive-green table for sale, in extremely good condition.",
                "For sale: green Subaru Impreza, 210,000 miles"
            )
        );
        assert_delta!(
            0.14118,
            sorensen_dice(
                "Olive-green table for sale, in extremely good condition.",
                "Wanted: mountain bike with at least 21 gears."
            )
        );
        assert_delta!(
            0.77419,
            sorensen_dice("this has one extra word", "this has one word")
        );
    }
}
