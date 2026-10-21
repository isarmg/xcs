#[cfg(all(test, feature = "alloc"))] // Tests make use of Vec at the moment
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn collect_list<T: Copy>(mut list: LinkedList<T>) -> Vec<T> {
        let mut result = Vec::new();
        list.drain(|node| {
            result.push(**node);
        });
        result
    }

    fn collect_reverse_list<T: Copy>(mut list: LinkedList<T>) -> Vec<T> {
        let mut result = Vec::new();
        list.reverse_drain(|node| {
            result.push(**node);
        });
        result
    }

    unsafe fn add_nodes(
        list: &mut LinkedList<i32>,
        nodes: &mut [&mut ListNode<i32>],
    ) {
        for node in nodes.iter_mut() {
            list.add_front(node);
        }
    }

    unsafe fn assert_clean<T>(node: &mut ListNode<T>) {
        assert!(node.next.is_none());
        assert!(node.prev.is_none());
    }

    #[test]
    fn insert_and_iterate() {
        unsafe {
            let mut a = ListNode::new(5);
            let mut b = ListNode::new(7);
            let mut c = ListNode::new(31);

            let mut setup = |list: &mut LinkedList<i32>| {
                assert_eq!(true, list.is_empty());
                list.add_front(&mut c);
                assert_eq!(31, **list.peek_first().unwrap());
                assert_eq!(false, list.is_empty());
                list.add_front(&mut b);
                assert_eq!(7, **list.peek_first().unwrap());
                list.add_front(&mut a);
                assert_eq!(5, **list.peek_first().unwrap());
            };

            let mut list = LinkedList::new();
            setup(&mut list);
            let items: Vec<i32> = collect_list(list);
            assert_eq!([5, 7, 31].to_vec(), items);

            let mut list = LinkedList::new();
            setup(&mut list);
            let items: Vec<i32> = collect_reverse_list(list);
            assert_eq!([31, 7, 5].to_vec(), items);
        }
    }

    #[test]
    fn drain_and_collect() {
        unsafe {
            let mut a = ListNode::new(5);
            let mut b = ListNode::new(7);
            let mut c = ListNode::new(31);

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut c, &mut b, &mut a]);

            let taken_items: Vec<i32> = collect_list(list);
            assert_eq!([5, 7, 31].to_vec(), taken_items);
        }
    }

    #[test]
    fn peek_last() {
        unsafe {
            let mut a = ListNode::new(5);
            let mut b = ListNode::new(7);
            let mut c = ListNode::new(31);

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut c, &mut b, &mut a]);

            let last = list.peek_last();
            assert_eq!(31, **last.unwrap());
            list.remove_last();

            let last = list.peek_last();
            assert_eq!(7, **last.unwrap());
            list.remove_last();

            let last = list.peek_last();
            assert_eq!(5, **last.unwrap());
            list.remove_last();

            let last = list.peek_last();
            assert!(last.is_none());
        }
    }

    #[test]
    fn remove_first() {
        unsafe {
            // We iterate forward and backwards through the manipulated lists
            // to make sure pointers in both directions are still ok.
            let mut a = ListNode::new(5);
            let mut b = ListNode::new(7);
            let mut c = ListNode::new(31);

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut c, &mut b, &mut a]);
            let removed = list.remove_first().unwrap();
            assert_clean(removed);
            assert!(!list.is_empty());
            let items: Vec<i32> = collect_list(list);
            assert_eq!([7, 31].to_vec(), items);

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut c, &mut b, &mut a]);
            let removed = list.remove_first().unwrap();
            assert_clean(removed);
            assert!(!list.is_empty());
            let items: Vec<i32> = collect_reverse_list(list);
            assert_eq!([31, 7].to_vec(), items);

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut b, &mut a]);
            let removed = list.remove_first().unwrap();
            assert_clean(removed);
            assert!(!list.is_empty());
            let items: Vec<i32> = collect_list(list);
            assert_eq!([7].to_vec(), items);

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut b, &mut a]);
            let removed = list.remove_first().unwrap();
            assert_clean(removed);
            assert!(!list.is_empty());
            let items: Vec<i32> = collect_reverse_list(list);
            assert_eq!([7].to_vec(), items);

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut a]);
            let removed = list.remove_first().unwrap();
            assert_clean(removed);
            assert!(list.is_empty());
            let items: Vec<i32> = collect_list(list);
            assert!(items.is_empty());

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut a]);
            let removed = list.remove_first().unwrap();
            assert_clean(removed);
            assert!(list.is_empty());
            let items: Vec<i32> = collect_reverse_list(list);
            assert!(items.is_empty());
        }
    }

    #[test]
    fn remove_last() {
        unsafe {
            // We iterate forward and backwards through the manipulated lists
            // to make sure pointers in both directions are still ok.
            let mut a = ListNode::new(5);
            let mut b = ListNode::new(7);
            let mut c = ListNode::new(31);

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut c, &mut b, &mut a]);
            let removed = list.remove_last().unwrap();
            assert_clean(removed);
            assert!(!list.is_empty());
            let items: Vec<i32> = collect_list(list);
            assert_eq!([5, 7].to_vec(), items);

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut c, &mut b, &mut a]);
            let removed = list.remove_last().unwrap();
            assert_clean(removed);
            assert!(!list.is_empty());
            let items: Vec<i32> = collect_reverse_list(list);
            assert_eq!([7, 5].to_vec(), items);

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut b, &mut a]);
            let removed = list.remove_last().unwrap();
            assert_clean(removed);
            assert!(!list.is_empty());
            let items: Vec<i32> = collect_list(list);
            assert_eq!([5].to_vec(), items);

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut b, &mut a]);
            let removed = list.remove_last().unwrap();
            assert_clean(removed);
            assert!(!list.is_empty());
            let items: Vec<i32> = collect_reverse_list(list);
            assert_eq!([5].to_vec(), items);

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut a]);
            let removed = list.remove_last().unwrap();
            assert_clean(removed);
            assert!(list.is_empty());
            let items: Vec<i32> = collect_list(list);
            assert!(items.is_empty());

            let mut list = LinkedList::new();
            add_nodes(&mut list, &mut [&mut a]);
            let removed = list.remove_last().unwrap();
            assert_clean(removed);
            assert!(list.is_empty());
            let items: Vec<i32> = collect_reverse_list(list);
            assert!(items.is_empty());
        }
    }

    #[test]
    fn remove_by_address() {
        unsafe {
            let mut a = ListNode::new(5);
            let mut b = ListNode::new(7);
            let mut c = ListNode::new(31);

            {
                // Remove first
                let mut list = LinkedList::new();
                add_nodes(&mut list, &mut [&mut c, &mut b, &mut a]);
                assert_eq!(true, list.remove(&mut a));
                assert_clean((&mut a).into());
                // a should be no longer there and can't be removed twice
                assert_eq!(false, list.remove(&mut a));
                assert_eq!(Some((&mut b).into()), list.head);
                assert_eq!(Some((&mut c).into()), b.next);
                assert_eq!(Some((&mut b).into()), c.prev);
                let items: Vec<i32> = collect_list(list);
                assert_eq!([7, 31].to_vec(), items);

                let mut list = LinkedList::new();
                add_nodes(&mut list, &mut [&mut c, &mut b, &mut a]);
                assert_eq!(true, list.remove(&mut a));
                assert_clean((&mut a).into());
                // a should be no longer there and can't be removed twice
                assert_eq!(false, list.remove(&mut a));
                assert_eq!(Some((&mut c).into()), b.next);
                assert_eq!(Some((&mut b).into()), c.prev);
                let items: Vec<i32> = collect_reverse_list(list);
                assert_eq!([31, 7].to_vec(), items);
            }

            {
                // Remove middle
                let mut list = LinkedList::new();
                add_nodes(&mut list, &mut [&mut c, &mut b, &mut a]);
                assert_eq!(true, list.remove(&mut b));
                assert_clean((&mut b).into());
                assert_eq!(Some((&mut c).into()), a.next);
                assert_eq!(Some((&mut a).into()), c.prev);
                let items: Vec<i32> = collect_list(list);
                assert_eq!([5, 31].to_vec(), items);

                let mut list = LinkedList::new();
                add_nodes(&mut list, &mut [&mut c, &mut b, &mut a]);
                assert_eq!(true, list.remove(&mut b));
                assert_clean((&mut b).into());
                assert_eq!(Some((&mut c).into()), a.next);
                assert_eq!(Some((&mut a).into()), c.prev);
                let items: Vec<i32> = collect_reverse_list(list);
                assert_eq!([31, 5].to_vec(), items);
            }

            {
                // Remove last
                let mut list = LinkedList::new();
                add_nodes(&mut list, &mut [&mut c, &mut b, &mut a]);
                assert_eq!(true, list.remove(&mut c));
                assert_clean((&mut c).into());
                assert!(b.next.is_none());
                assert_eq!(Some((&mut b).into()), list.tail);
                let items: Vec<i32> = collect_list(list);
                assert_eq!([5, 7].to_vec(), items);

                let mut list = LinkedList::new();
                add_nodes(&mut list, &mut [&mut c, &mut b, &mut a]);
                assert_eq!(true, list.remove(&mut c));
                assert_clean((&mut c).into());
                assert!(b.next.is_none());
                assert_eq!(Some((&mut b).into()), list.tail);
                let items: Vec<i32> = collect_reverse_list(list);
                assert_eq!([7, 5].to_vec(), items);
            }

            {
                // Remove first of two
                let mut list = LinkedList::new();
                add_nodes(&mut list, &mut [&mut b, &mut a]);
                assert_eq!(true, list.remove(&mut a));
                assert_clean((&mut a).into());
                // a should be no longer there and can't be removed twice
                assert_eq!(false, list.remove(&mut a));
                assert_eq!(Some((&mut b).into()), list.head);
                assert_eq!(Some((&mut b).into()), list.tail);
                assert!(b.next.is_none());
                assert!(b.prev.is_none());
                let items: Vec<i32> = collect_list(list);
                assert_eq!([7].to_vec(), items);

                let mut list = LinkedList::new();
                add_nodes(&mut list, &mut [&mut b, &mut a]);
                assert_eq!(true, list.remove(&mut a));
                assert_clean((&mut a).into());
                // a should be no longer there and can't be removed twice
                assert_eq!(false, list.remove(&mut a));
                assert_eq!(Some((&mut b).into()), list.head);
                assert_eq!(Some((&mut b).into()), list.tail);
                assert!(b.next.is_none());
                assert!(b.prev.is_none());
                let items: Vec<i32> = collect_reverse_list(list);
                assert_eq!([7].to_vec(), items);
            }

            {
                // Remove last of two
                let mut list = LinkedList::new();
                add_nodes(&mut list, &mut [&mut b, &mut a]);
                assert_eq!(true, list.remove(&mut b));
                assert_clean((&mut b).into());
                assert_eq!(Some((&mut a).into()), list.head);
                assert_eq!(Some((&mut a).into()), list.tail);
                assert!(a.next.is_none());
                assert!(a.prev.is_none());
                let items: Vec<i32> = collect_list(list);
                assert_eq!([5].to_vec(), items);

                let mut list = LinkedList::new();
                add_nodes(&mut list, &mut [&mut b, &mut a]);
                assert_eq!(true, list.remove(&mut b));
                assert_clean((&mut b).into());
                assert_eq!(Some((&mut a).into()), list.head);
                assert_eq!(Some((&mut a).into()), list.tail);
                assert!(a.next.is_none());
                assert!(a.prev.is_none());
                let items: Vec<i32> = collect_reverse_list(list);
                assert_eq!([5].to_vec(), items);
            }

            {
                // Remove last item
                let mut list = LinkedList::new();
                add_nodes(&mut list, &mut [&mut a]);
                assert_eq!(true, list.remove(&mut a));
                assert_clean((&mut a).into());
                assert!(list.head.is_none());
                assert!(list.tail.is_none());
                let items: Vec<i32> = collect_list(list);
                assert!(items.is_empty());
            }

            {
                // Remove missing
                let mut list = LinkedList::new();
                list.add_front(&mut b);
                list.add_front(&mut a);
                assert_eq!(false, list.remove(&mut c));
            }
        }
    }
}
