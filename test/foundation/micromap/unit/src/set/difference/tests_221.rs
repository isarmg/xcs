#[cfg(test)]
    mod tests {
        use super::Set;

        #[test]
        fn difference_ref_lifetime() {
            let sentence_1 = String::from("I love the surf and the sand.");
            let sentence_1_words: Set<&str, 10> = sentence_1.split(" ").collect();
            let sentence_2 = String::from("I hate the hate and the sand.");
            let sentence_2_words: Set<&str, 10> = sentence_2.split(" ").collect();
            let diff: Vec<_> = sentence_1_words.difference_ref(&sentence_2_words).collect();
            assert_eq!(diff, vec!["love", "surf"]);
        }

        #[test]
        fn difference_ref_disjoint() {
            let set_a = Set::from(["apple", "banana", "cherry"]);
            let set_b = Set::from(["date", "elderberry", "fig", "grape"]);
            let diff: Vec<_> = set_a.difference_ref(&set_b).collect();
            assert_eq!(diff, vec!["apple", "banana", "cherry"]);
        }

        #[test]
        fn difference_ref_with_overlap() {
            let set_a = Set::from(["apple", "banana", "cherry"]);
            let set_b = Set::from(["banana", "cherry", "date"]);
            let diff: Vec<_> = set_a.difference_ref(&set_b).collect();
            assert_eq!(diff, vec!["apple"]);
        }

        #[test]
        fn difference_ref_complete_overlap() {
            let set_a = Set::from(["apple", "banana", "cherry"]);
            let set_b = Set::from(["apple", "banana", "cherry"]);
            let diff: Vec<_> = set_a.difference_ref(&set_b).collect();
            assert!(diff.is_empty());
        }

        #[test]
        fn difference_ref_empty_set() {
            let set_a = Set::from(["apple", "banana", "cherry"]);
            let set_b = Set::from([]);
            let diff = set_a.difference_ref(&set_b);
            assert_eq!(diff.size_hint(), (3, Some(3)));
            let diff: Vec<_> = diff.collect();
            assert_eq!(diff, vec!["apple", "banana", "cherry"]);
        }

        #[test]
        fn difference_ref_fold() {
            let set_a = Set::from(["apple", "banana", "cherry"]);
            let set_b = Set::from(["banana", "cherry", "date"]);
            let diff = set_a.difference_ref(&set_b);
            let result = diff.fold(String::new(), |mut acc, item| {
                acc.push_str(item);
                acc.push(',');
                acc
            });
            assert_eq!(result, "apple,");
        }

        #[test]
        fn difference_ref_clone() {
            let ss = ["apple", "banana", "cherry", "date"];
            let sss: [String; 4] = ss.map(|s| s.to_string());
            let set_a = Set::from([&sss[0], &sss[1], &sss[2]]);
            let set_b = Set::from([&sss[1], &sss[2], &sss[3]]);
            let diff = set_a.difference_ref(&set_b);
            let cloned_diff = diff.clone();
            let original: Vec<_> = diff.collect();
            let cloned: Vec<_> = cloned_diff.collect();
            assert_eq!(original, cloned);
        }

        #[test]
        fn difference_ref_fmt_debug() {
            let ss = ["apple", "banana", "cherry", "date"];
            let sss: [String; 4] = ss.map(|s| s.to_string());
            let set_a = Set::from([&sss[0], &sss[1], &sss[2]]);
            let set_b = Set::from([&sss[1], &sss[2], &sss[3]]);
            let diff = set_a.difference_ref(&set_b);
            let debug_output = format!("{:?}", diff);
            assert_eq!(debug_output, "[\"apple\"]");
        }
    }
