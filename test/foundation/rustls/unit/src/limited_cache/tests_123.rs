#[cfg(test)]
mod tests {
    use std::prelude::v1::*;

    type Test = super::LimitedCache<String, usize>;

    #[test]
    fn test_updates_existing_item() {
        let mut t = Test::new(3);
        t.insert("abc".into(), 1);
        t.insert("abc".into(), 2);
        assert_eq!(t.get("abc"), Some(&2));
    }

    #[test]
    fn test_evicts_oldest_item() {
        let mut t = Test::new(3);
        t.insert("abc".into(), 1);
        t.insert("def".into(), 2);
        t.insert("ghi".into(), 3);

        assert_eq!(t.get("abc"), Some(&1));
        assert_eq!(t.get("def"), Some(&2));
        assert_eq!(t.get("ghi"), Some(&3));

        t.insert("jkl".into(), 4);
        assert_eq!(t.get("abc"), None);
        assert_eq!(t.get("jkl"), Some(&4));
    }

    #[test]
    fn test_capacity_one_retains_the_item() {
        let mut t = Test::new(1);
        t.insert("abc".into(), 1);
        assert_eq!(t.get("abc"), Some(&1));
        t.insert("def".into(), 2);
        assert_eq!(t.get("abc"), None);
        assert_eq!(t.get("def"), Some(&2));
    }

    #[test]
    fn test_evicts_second_oldest_item_if_first_removed() {
        let mut t = Test::new(3);
        t.insert("abc".into(), 1);
        t.insert("def".into(), 2);

        assert_eq!(t.remove("abc"), Some(1));

        t.insert("ghi".into(), 3);
        t.insert("jkl".into(), 4);

        assert_eq!(t.get("abc"), None);
        assert_eq!(t.get("def"), Some(&2));
        assert_eq!(t.get("ghi"), Some(&3));
        assert_eq!(t.get("jkl"), Some(&4));
    }

    #[test]
    fn test_evicts_after_second_oldest_item_removed() {
        let mut t = Test::new(3);
        t.insert("abc".into(), 1);
        t.insert("def".into(), 2);

        assert_eq!(t.remove("def"), Some(2));
        assert_eq!(t.get("abc"), Some(&1));

        t.insert("ghi".into(), 3);
        t.insert("jkl".into(), 4);

        assert_eq!(t.get("abc"), Some(&1));
        assert_eq!(t.get("def"), None);
        assert_eq!(t.get("ghi"), Some(&3));
        assert_eq!(t.get("jkl"), Some(&4));

        t.insert("mno".into(), 5);
        assert_eq!(t.get("abc"), None);
        assert_eq!(t.get("mno"), Some(&5));
    }

    #[test]
    fn test_removes_all_items() {
        let mut t = Test::new(3);
        t.insert("abc".into(), 1);
        t.insert("def".into(), 2);

        assert_eq!(t.remove("def"), Some(2));
        assert_eq!(t.remove("abc"), Some(1));

        t.insert("ghi".into(), 3);
        t.insert("jkl".into(), 4);
        t.insert("mno".into(), 5);

        assert_eq!(t.get("abc"), None);
        assert_eq!(t.get("def"), None);
        assert_eq!(t.get("ghi"), Some(&3));
        assert_eq!(t.get("jkl"), Some(&4));
        assert_eq!(t.get("mno"), Some(&5));
    }

    #[test]
    fn test_inserts_many_items() {
        let mut t = Test::new(3);

        for _ in 0..10000 {
            t.insert("abc".into(), 1);
            t.insert("def".into(), 2);
            t.insert("ghi".into(), 3);
        }
    }

    #[test]
    fn test_get_or_insert_default_and_edit_evicts_old_items_to_meet_capacity() {
        let mut t = Test::new(3);

        t.get_or_insert_default_and_edit("abc".into(), |v| *v += 1);
        t.get_or_insert_default_and_edit("def".into(), |v| *v += 2);
        t.get_or_insert_default_and_edit("ghi".into(), |v| *v += 3);
        assert_eq!(t.get("abc"), Some(&1));

        // evicts "abc"
        t.get_or_insert_default_and_edit("jkl".into(), |v| *v += 4);
        assert_eq!(t.get("abc"), None);

        // evicts "def"
        t.get_or_insert_default_and_edit("abc".into(), |v| *v += 5);
        assert_eq!(t.get("def"), None);

        // evicts "ghi"
        t.get_or_insert_default_and_edit("def".into(), |v| *v += 6);
        assert_eq!(t.get("ghi"), None);

        assert_eq!(t.get("abc"), Some(&5));
        assert_eq!(t.get("def"), Some(&6));
        assert_eq!(t.get("ghi"), None);
        assert_eq!(t.get("jkl"), Some(&4));
    }

    #[test]
    fn test_get_or_insert_default_and_edit_edits_existing_item() {
        let mut t = Test::new(3);

        t.get_or_insert_default_and_edit("abc".into(), |v| *v += 1);
        t.get_or_insert_default_and_edit("abc".into(), |v| *v += 2);
        t.get_or_insert_default_and_edit("abc".into(), |v| *v += 3);

        assert_eq!(t.get("abc"), Some(&6));
    }
}
