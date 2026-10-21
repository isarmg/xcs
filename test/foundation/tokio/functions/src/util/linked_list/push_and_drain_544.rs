#[test]
    fn push_and_drain() {
        let a = entry(5);
        let b = entry(7);
        let c = entry(31);

        let mut list = LinkedList::new();
        assert!(list.is_empty());

        list.push_front(a.as_ref());
        assert!(!list.is_empty());
        list.push_front(b.as_ref());
        list.push_front(c.as_ref());

        let items: Vec<i32> = collect_list(&mut list);
        assert_eq!([5, 7, 31].to_vec(), items);

        assert!(list.is_empty());
    }
