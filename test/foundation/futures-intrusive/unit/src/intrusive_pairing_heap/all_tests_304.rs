#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{HeapNode, PairingHeap};
    use core::ptr::NonNull;

    // Recursively check the provided node and all descendants for:
    // - pointer consistency: parent pointers and next/prev
    // - the heap property: `node.data <= child.data` for all children
    unsafe fn validate_heap_node<T: Ord>(
        node: &HeapNode<T>,
        parent: Option<&HeapNode<T>>,
    ) {
        assert_eq!(node.parent, parent.map(NonNull::from));
        if let Some(p) = parent {
            assert!(p.data <= node.data);
        }
        if let Some(prev) = node.prev {
            assert_eq!(prev.as_ref().next, Some(node.into()));
        }
        if let Some(next) = node.next {
            assert_eq!(next.as_ref().prev, Some(node.into()));
        }
        let mut child = node.first_child;
        while let Some(c) = child {
            validate_heap_node(c.as_ref(), Some(node));
            child = c.as_ref().next;
        }
    }

    fn validate_heap<T: Ord>(heap: &PairingHeap<T>) {
        if let Some(root) = heap.root {
            // This is also sufficient to check that `heap.root` is indeed a
            // minimum element of the heap.
            unsafe {
                validate_heap_node(root.as_ref(), None);
            }
        }
    }

    #[test]
    fn insert_and_remove() {
        // This test exhaustively covers every possible schedule of inserting,
        // then removing, each of five different nodes from the heap.
        #[derive(Copy, Clone, Debug)]
        enum Action {
            Insert(u8),
            Remove(u8),
        }

        fn generate_schedules(
            current: &mut Vec<Action>,
            available: &mut Vec<Action>,
            f: fn(&[Action]),
        ) {
            for i in 0..available.len() {
                let action = available.swap_remove(i);
                current.push(action);
                f(current);
                if let Action::Insert(j) = action {
                    available.push(Action::Remove(j));
                }
                generate_schedules(current, available, f);
                if let Action::Insert(_) = action {
                    available.pop();
                }
                current.pop();
                // the opposite of `swap_remove`
                available.push(action);
                let len = available.len();
                available.swap(i, len - 1);
            }
        }
        let max = if cfg!(miri) {
            // Miri is really slow, make things easier.
            3
        } else {
            // 5 runs in a reasonable amount of time but still exercises
            // interesting cases.
            5
        };
        generate_schedules(
            &mut vec![],
            &mut (0..max).map(Action::Insert).collect(),
            |schedule| unsafe {
                let mut nodes = [
                    HeapNode::new(0u8),
                    HeapNode::new(1),
                    HeapNode::new(2),
                    HeapNode::new(3),
                    HeapNode::new(4),
                ];
                let mut heap = PairingHeap::new();
                for action in schedule {
                    match *action {
                        Action::Insert(n) => {
                            heap.insert(&mut nodes[n as usize]);
                            validate_heap(&heap);
                        }
                        Action::Remove(n) => {
                            heap.remove(&mut nodes[n as usize]);
                            assert!(nodes[n as usize].is_root());
                            assert_eq!(nodes[n as usize].first_child, None);
                            validate_heap(&heap);
                        }
                    }
                }
            },
        );
    }

    #[test]
    fn equal_values() {
        // Check that things behave properly in the presence of equal values.
        unsafe {
            let mut nodes = [
                HeapNode::new(0u8),
                HeapNode::new(0),
                HeapNode::new(0),
                HeapNode::new(0),
                HeapNode::new(0),
            ];
            let mut heap = PairingHeap::new();
            for node in &mut nodes {
                heap.insert(node);
                validate_heap(&heap);
            }
            for _ in 0..5 {
                heap.remove(heap.peek_min().unwrap().as_mut());
                validate_heap(&heap);
            }
            assert_eq!(heap.peek_min(), None);
        }
    }
}
