#[cfg(test)]
mod tests {
    use super::Set;

    // NOTE: This sample is a BUG in the standard library function.
    #[test]
    fn difference_lifetime() {
        // use std::collections::hash_set::HashSet as Set;
        let sentence_1 = String::from("I love the surf and the sand.");
        let sentence_1_words: Set<&str, 10> = sentence_1.split(" ").collect();
        let first_only = {
            let sentence_2 = String::from("I hate the hate and the sand.");
            let sentence_2_words: Set<&str, 10> = sentence_2.split(" ").collect();
            let first_only: Vec<_> = sentence_1_words.difference_ref(&sentence_2_words).collect();
            let second_only: Vec<_> = sentence_2_words.difference_ref(&sentence_1_words).collect();
            println!("First  Sentence: {}", sentence_1);
            println!("Second Sentence: {}", sentence_2);
            println!("{:?}", first_only);
            println!("{:?}", second_only);
            first_only
        };
        assert_eq!(
            Set::<_, 10>::from_iter(first_only.iter().copied()),
            Set::<_, 2>::from(["love", "surf"])
        );
    }

    #[test]
    fn difference_disjoint() {
        let set_a: Set<u32, 5> = Set::from([0, 1, 3, 5, 7]);
        let set_b: Set<u32, 4> = Set::from([2, 4, 6, 8]);
        let set_diff = set_a.difference(&set_b).copied().collect::<Set<u32, 5>>();
        assert_eq!(set_a, set_diff);
    }

    #[test]
    fn difference_with_overlap() {
        let set_a = Set::from([1, 3, 5, 7]);
        let set_b = Set::from([3, 5, 6, 8, 9]);
        let set_diff = set_a.difference(&set_b).copied().collect::<Set<u32, 4>>();
        let expected = Set::from([1, 7]);
        assert_eq!(expected, set_diff);
    }

    #[test]
    fn difference_complete_overlap() {
        let set_a = Set::from([1, 3, 5, 7]);
        let set_b = Set::from([1, 3, 5, 7]);
        let set_diff = set_a.difference(&set_b).copied().collect::<Set<u32, 4>>();
        let expected = Set::from([]);
        assert_eq!(expected, set_diff);
    }

    #[test]
    fn difference_empty_set() {
        let set_a = Set::from([1, 3, 5, 7]);
        let set_b = Set::from([]);
        let set_diff = set_a.difference(&set_b).copied().collect::<Set<u32, 4>>();
        assert_eq!(set_a, set_diff);
    }

    #[test]
    fn difference_with_empty_first_set() {
        let set_a = Set::from([]);
        let set_b = Set::from([2, 4, 6, 8]);
        let set_diff = set_a.difference(&set_b).copied().collect::<Set<u32, 4>>();
        let expected = Set::from([]);
        assert_eq!(expected, set_diff);
    }

    #[test]
    fn difference_partial_overlap() {
        let set_a = Set::from([1, 2, 3, 4, 5, 6]);
        let set_b = Set::from([4, 5, 6, 7, 8, 9]);
        let set_diff = set_a.difference(&set_b).copied().collect::<Set<_, 6>>();
        let expected = Set::from([1, 2, 3]);
        assert_eq!(expected, set_diff);
    }

    #[test]
    fn difference_size_hint() {
        let set_a = Set::from([1, 1, 2, 3]); // cap is 4, but len() is 3
        let set_b = Set::from([4, 5, 6, 6, 6, 7, 8, 9]); // cap is 8, but len() is 6
        let set_c = Set::from([]);
        let set_d = Set::from([3, 4]);
        assert_eq!(set_a.difference(&set_b).size_hint(), (0, Some(3)));
        assert_eq!(set_a.difference(&set_c).size_hint(), (3, Some(3)));
        assert_eq!(set_a.difference(&set_d).size_hint(), (1, Some(3)));
        assert_eq!(set_b.difference(&set_a).size_hint(), (3, Some(6)));
        assert_eq!(set_b.difference(&set_d).size_hint(), (4, Some(6)));
        assert_eq!(set_c.difference(&set_b).size_hint(), (0, Some(0)));
        assert_eq!(set_d.difference(&set_a).size_hint(), (0, Some(2)));
        assert_eq!(set_d.difference(&set_b).size_hint(), (0, Some(2)));
        assert_eq!(set_d.difference(&set_c).size_hint(), (2, Some(2)));
    }

    #[test]
    fn difference_fmt_debug() {
        let set_a = Set::from([1, 2, 3]);
        let set_b = Set::from([3, 4, 5, 6]);
        let diff = set_a.difference(&set_b);
        let debug_output = format!("{:?}", diff);
        assert_eq!(debug_output, "[1, 2]");
    }

    #[test]
    fn difference_fmt_debug_empty() {
        let set_a = Set::from([1, 2, 3]);
        let set_b = Set::from([1, 2, 3]);
        let diff = set_a.difference(&set_b);
        let debug_output = format!("{:?}", diff);
        assert_eq!(debug_output, "[]");
    }

    #[test]
    fn difference_fmt_debug_with_empty_other_set() {
        let set_a = Set::from([1, 2, 3]);
        let set_b = Set::from([]);
        let diff = set_a.difference(&set_b);
        let debug_output = format!("{:?}", diff);
        assert_eq!(debug_output, "[1, 2, 3]");
    }
}
