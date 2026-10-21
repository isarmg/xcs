#[cfg(test)]
mod test {
    use super::VarZeroVecOwned;
    #[test]
    fn test_insert_integrity() {
        let mut items: Vec<String> = Vec::new();
        let mut zerovec = VarZeroVecOwned::<str>::new();

        // Insert into an empty vec.
        items.insert(0, "1234567890".into());
        zerovec.insert(0, "1234567890");
        assert_eq!(zerovec, &*items);

        zerovec.insert(1, "foo3");
        items.insert(1, "foo3".into());
        assert_eq!(zerovec, &*items);

        // Insert at the end.
        items.insert(items.len(), "qwertyuiop".into());
        zerovec.insert(zerovec.len(), "qwertyuiop");
        assert_eq!(zerovec, &*items);

        items.insert(0, "asdfghjkl;".into());
        zerovec.insert(0, "asdfghjkl;");
        assert_eq!(zerovec, &*items);

        items.insert(2, "".into());
        zerovec.insert(2, "");
        assert_eq!(zerovec, &*items);
    }

    #[test]
    // ensure that inserting empty items works
    fn test_empty_inserts() {
        let mut items: Vec<String> = Vec::new();
        let mut zerovec = VarZeroVecOwned::<str>::new();

        // Insert into an empty vec.
        items.insert(0, "".into());
        zerovec.insert(0, "");
        assert_eq!(zerovec, &*items);

        items.insert(0, "".into());
        zerovec.insert(0, "");
        assert_eq!(zerovec, &*items);

        items.insert(0, "1234567890".into());
        zerovec.insert(0, "1234567890");
        assert_eq!(zerovec, &*items);

        items.insert(0, "".into());
        zerovec.insert(0, "");
        assert_eq!(zerovec, &*items);
    }

    #[test]
    fn test_small_insert_integrity() {
        // Tests that insert() works even when there
        // is not enough space for the new index in entire_slice.len()
        let mut items: Vec<String> = Vec::new();
        let mut zerovec = VarZeroVecOwned::<str>::new();

        // Insert into an empty vec.
        items.insert(0, "abc".into());
        zerovec.insert(0, "abc");
        assert_eq!(zerovec, &*items);

        zerovec.insert(1, "def");
        items.insert(1, "def".into());
        assert_eq!(zerovec, &*items);
    }

    #[test]
    #[should_panic]
    fn test_insert_past_end() {
        VarZeroVecOwned::<str>::new().insert(1, "");
    }

    #[test]
    fn test_remove_integrity() {
        let mut items: Vec<&str> = vec!["apples", "bananas", "eeples", "", "baneenees", "five", ""];
        let mut zerovec = VarZeroVecOwned::<str>::try_from_elements(&items).unwrap();

        for index in [0, 2, 4, 0, 1, 1, 0] {
            items.remove(index);
            zerovec.remove(index);
            assert_eq!(zerovec, &*items, "index {}, len {}", index, items.len());
        }
    }

    #[test]
    fn test_removing_last_element_clears() {
        let mut zerovec = VarZeroVecOwned::<str>::try_from_elements(&["buy some apples"]).unwrap();
        assert!(!zerovec.as_bytes().is_empty());
        zerovec.remove(0);
        assert!(zerovec.as_bytes().is_empty());
    }

    #[test]
    #[should_panic]
    fn test_remove_past_end() {
        VarZeroVecOwned::<str>::new().remove(0);
    }

    #[test]
    fn test_replace_integrity() {
        let mut items: Vec<&str> = vec!["apples", "bananas", "eeples", "", "baneenees", "five", ""];
        let mut zerovec = VarZeroVecOwned::<str>::try_from_elements(&items).unwrap();

        // Replace with an element of the same size (and the first element)
        items[0] = "blablah";
        zerovec.replace(0, "blablah");
        assert_eq!(zerovec, &*items);

        // Replace with a smaller element
        items[1] = "twily";
        zerovec.replace(1, "twily");
        assert_eq!(zerovec, &*items);

        // Replace an empty element
        items[3] = "aoeuidhtns";
        zerovec.replace(3, "aoeuidhtns");
        assert_eq!(zerovec, &*items);

        // Replace the last element
        items[6] = "0123456789";
        zerovec.replace(6, "0123456789");
        assert_eq!(zerovec, &*items);

        // Replace with an empty element
        items[2] = "";
        zerovec.replace(2, "");
        assert_eq!(zerovec, &*items);
    }

    #[test]
    #[should_panic]
    fn test_replace_past_end() {
        VarZeroVecOwned::<str>::new().replace(0, "");
    }
}
