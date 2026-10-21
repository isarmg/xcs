#[cfg(test)]
mod test {
    use super::*;
    use alloc::collections::BTreeMap;

    #[test]
    fn stress_test() {
        let mut zm2d = ZeroMap2d::<u16, str, str>::new();

        assert_eq!(
            format!("{zm2d:?}"),
            "ZeroMap2d { keys0: ZeroVec([]), joiner: ZeroVec([]), keys1: [], values: [] }"
        );
        assert_eq!(zm2d.get0(&0), None);

        let result = zm2d.try_append(&3, "ccc", "CCC");
        assert!(result.is_none());

        assert_eq!(format!("{zm2d:?}"), "ZeroMap2d { keys0: ZeroVec([3]), joiner: ZeroVec([1]), keys1: [\"ccc\"], values: [\"CCC\"] }");
        assert_eq!(zm2d.get0(&0), None);
        assert_eq!(zm2d.get0(&3).unwrap().get1(""), None);
        assert_eq!(zm2d.get_2d(&3, "ccc"), Some("CCC"));
        assert_eq!(zm2d.get0(&99), None);

        let result = zm2d.try_append(&3, "eee", "EEE");
        assert!(result.is_none());

        assert_eq!(format!("{zm2d:?}"), "ZeroMap2d { keys0: ZeroVec([3]), joiner: ZeroVec([2]), keys1: [\"ccc\", \"eee\"], values: [\"CCC\", \"EEE\"] }");
        assert_eq!(zm2d.get0(&0), None);
        assert_eq!(zm2d.get0(&3).unwrap().get1(""), None);
        assert_eq!(zm2d.get_2d(&3, "ccc"), Some("CCC"));
        assert_eq!(zm2d.get_2d(&3, "eee"), Some("EEE"));
        assert_eq!(zm2d.get0(&3).unwrap().get1("five"), None);
        assert_eq!(zm2d.get0(&99), None);

        // Out of order
        let result = zm2d.try_append(&3, "ddd", "DD0");
        assert!(result.is_some());

        // Append a few more elements
        let result = zm2d.try_append(&5, "ddd", "DD1");
        assert!(result.is_none());
        let result = zm2d.try_append(&7, "ddd", "DD2");
        assert!(result.is_none());
        let result = zm2d.try_append(&7, "eee", "EEE");
        assert!(result.is_none());
        let result = zm2d.try_append(&7, "www", "WWW");
        assert!(result.is_none());
        let result = zm2d.try_append(&9, "yyy", "YYY");
        assert!(result.is_none());

        assert_eq!(format!("{zm2d:?}"), "ZeroMap2d { keys0: ZeroVec([3, 5, 7, 9]), joiner: ZeroVec([2, 3, 6, 7]), keys1: [\"ccc\", \"eee\", \"ddd\", \"ddd\", \"eee\", \"www\", \"yyy\"], values: [\"CCC\", \"EEE\", \"DD1\", \"DD2\", \"EEE\", \"WWW\", \"YYY\"] }");
        assert_eq!(zm2d.get0(&0), None);
        assert_eq!(zm2d.get0(&3).unwrap().get1(""), None);
        assert_eq!(zm2d.get_2d(&3, "ccc"), Some("CCC"));
        assert_eq!(zm2d.get_2d(&3, "eee"), Some("EEE"));
        assert_eq!(zm2d.get0(&3).unwrap().get1("zzz"), None);
        assert_eq!(zm2d.get0(&4), None);
        assert_eq!(zm2d.get0(&5).unwrap().get1("aaa"), None);
        assert_eq!(zm2d.get_2d(&5, "ddd"), Some("DD1"));
        assert_eq!(zm2d.get0(&5).unwrap().get1("zzz"), None);
        assert_eq!(zm2d.get0(&6), None);
        assert_eq!(zm2d.get0(&7).unwrap().get1("aaa"), None);
        assert_eq!(zm2d.get_2d(&7, "ddd"), Some("DD2"));
        assert_eq!(zm2d.get_2d(&7, "eee"), Some("EEE"));
        assert_eq!(zm2d.get_2d(&7, "www"), Some("WWW"));
        assert_eq!(zm2d.get0(&7).unwrap().get1("yyy"), None);
        assert_eq!(zm2d.get0(&7).unwrap().get1("zzz"), None);
        assert_eq!(zm2d.get0(&8), None);
        assert_eq!(zm2d.get0(&9).unwrap().get1("aaa"), None);
        assert_eq!(zm2d.get0(&9).unwrap().get1("www"), None);
        assert_eq!(zm2d.get_2d(&9, "yyy"), Some("YYY"));
        assert_eq!(zm2d.get0(&9).unwrap().get1("zzz"), None);
        assert_eq!(zm2d.get0(&10), None);
        assert_eq!(zm2d.get0(&99), None);

        // Insert some elements
        zm2d.insert(&3, "mmm", "MM0");
        zm2d.insert(&6, "ddd", "DD3");
        zm2d.insert(&6, "mmm", "MM1");
        zm2d.insert(&6, "nnn", "NNN");

        assert_eq!(format!("{zm2d:?}"), "ZeroMap2d { keys0: ZeroVec([3, 5, 6, 7, 9]), joiner: ZeroVec([3, 4, 7, 10, 11]), keys1: [\"ccc\", \"eee\", \"mmm\", \"ddd\", \"ddd\", \"mmm\", \"nnn\", \"ddd\", \"eee\", \"www\", \"yyy\"], values: [\"CCC\", \"EEE\", \"MM0\", \"DD1\", \"DD3\", \"MM1\", \"NNN\", \"DD2\", \"EEE\", \"WWW\", \"YYY\"] }");
        assert_eq!(zm2d.get0(&0), None);
        assert_eq!(zm2d.get0(&3).unwrap().get1(""), None);
        assert_eq!(zm2d.get_2d(&3, "ccc"), Some("CCC"));
        assert_eq!(zm2d.get_2d(&3, "eee"), Some("EEE"));
        assert_eq!(zm2d.get_2d(&3, "mmm"), Some("MM0"));
        assert_eq!(zm2d.get0(&3).unwrap().get1("zzz"), None);
        assert_eq!(zm2d.get0(&4), None);
        assert_eq!(zm2d.get0(&5).unwrap().get1("aaa"), None);
        assert_eq!(zm2d.get_2d(&5, "ddd"), Some("DD1"));
        assert_eq!(zm2d.get0(&5).unwrap().get1("zzz"), None);
        assert_eq!(zm2d.get0(&6).unwrap().get1("aaa"), None);
        assert_eq!(zm2d.get_2d(&6, "ddd"), Some("DD3"));
        assert_eq!(zm2d.get_2d(&6, "mmm"), Some("MM1"));
        assert_eq!(zm2d.get_2d(&6, "nnn"), Some("NNN"));
        assert_eq!(zm2d.get0(&6).unwrap().get1("zzz"), None);
        assert_eq!(zm2d.get0(&7).unwrap().get1("aaa"), None);
        assert_eq!(zm2d.get_2d(&7, "ddd"), Some("DD2"));
        assert_eq!(zm2d.get_2d(&7, "eee"), Some("EEE"));
        assert_eq!(zm2d.get_2d(&7, "www"), Some("WWW"));
        assert_eq!(zm2d.get0(&7).unwrap().get1("yyy"), None);
        assert_eq!(zm2d.get0(&7).unwrap().get1("zzz"), None);
        assert_eq!(zm2d.get0(&8), None);
        assert_eq!(zm2d.get0(&9).unwrap().get1("aaa"), None);
        assert_eq!(zm2d.get0(&9).unwrap().get1("www"), None);
        assert_eq!(zm2d.get_2d(&9, "yyy"), Some("YYY"));
        assert_eq!(zm2d.get0(&9).unwrap().get1("zzz"), None);
        assert_eq!(zm2d.get0(&10), None);
        assert_eq!(zm2d.get0(&99), None);

        // Remove some elements
        let result = zm2d.remove(&3, "ccc"); // first element
        assert_eq!(result.as_deref(), Some("CCC"));
        let result = zm2d.remove(&3, "mmm"); // middle element
        assert_eq!(result.as_deref(), Some("MM0"));
        let result = zm2d.remove(&5, "ddd"); // singleton K0
        assert_eq!(result.as_deref(), Some("DD1"));
        let result = zm2d.remove(&9, "yyy"); // last element
        assert_eq!(result.as_deref(), Some("YYY"));

        assert_eq!(format!("{zm2d:?}"), "ZeroMap2d { keys0: ZeroVec([3, 6, 7]), joiner: ZeroVec([1, 4, 7]), keys1: [\"eee\", \"ddd\", \"mmm\", \"nnn\", \"ddd\", \"eee\", \"www\"], values: [\"EEE\", \"DD3\", \"MM1\", \"NNN\", \"DD2\", \"EEE\", \"WWW\"] }");
    }

    #[test]
    fn zeromap2d_metazone() {
        let source_data = [
            (*b"aedxb", 0, Some(*b"gulf")),
            (*b"afkbl", 0, Some(*b"afgh")),
            (*b"ushnl", 0, None),
            (*b"ushnl", 7272660, Some(*b"haal")),
            (*b"ushnl", 0, None),
            (*b"ushnl", 7272660, Some(*b"haal")),
        ];

        let btreemap: BTreeMap<([u8; 5], i32), Option<[u8; 4]>> = source_data
            .iter()
            .copied()
            .map(|(a, b, c)| ((a, b), c))
            .collect();

        let zeromap2d: ZeroMap2d<[u8; 5], i32, Option<[u8; 4]>> =
            source_data.iter().copied().collect();

        let mut btreemap_iter = btreemap.iter();

        for cursor in zeromap2d.iter0() {
            for (key1, value) in cursor.iter1() {
                // This code runs for every (key0, key1) pair in order
                let expected = btreemap_iter.next().unwrap();
                assert_eq!(
                    (expected.0 .0, expected.0 .1, expected.1),
                    (*cursor.key0(), key1.as_unsigned_int() as i32, &value.get())
                );
            }
        }
        assert!(btreemap_iter.next().is_none());
    }
}
