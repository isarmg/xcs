#[cfg(any(test, fuzzing))]

pub(crate) mod tests {
    use super::*;

    use std::pin::Pin;

    #[derive(Debug)]
    #[repr(C)]
    struct Entry {
        pointers: Pointers<Entry>,
        val: i32,
    }

    unsafe impl<'a> Link for &'a Entry {
        type Handle = Pin<&'a Entry>;
        type Target = Entry;

        fn as_raw(handle: &Pin<&'_ Entry>) -> NonNull<Entry> {
            NonNull::from(handle.get_ref())
        }

        unsafe fn from_raw(ptr: NonNull<Entry>) -> Pin<&'a Entry> {
            Pin::new_unchecked(&*ptr.as_ptr())
        }

        unsafe fn pointers(target: NonNull<Entry>) -> NonNull<Pointers<Entry>> {
            target.cast()
        }
    }

    fn entry(val: i32) -> Pin<Box<Entry>> {
        Box::pin(Entry {
            pointers: Pointers::new(),
            val,
        })
    }

    fn ptr(r: &Pin<Box<Entry>>) -> NonNull<Entry> {
        r.as_ref().get_ref().into()
    }

    fn collect_list(list: &mut LinkedList<&'_ Entry>) -> Vec<i32> {
        let mut ret = vec![];

        while let Some(entry) = list.pop_back() {
            ret.push(entry.val);
        }

        ret
    }

    fn push_all<'a>(list: &mut LinkedList<&'a Entry>, entries: &[Pin<&'a Entry>]) {
        for entry in entries.iter() {
            list.push_front(*entry);
        }
    }

    #[cfg(test)]
    macro_rules! assert_clean {
        ($e:ident) => {{
            assert!($e.pointers.get_next().is_none());
            assert!($e.pointers.get_prev().is_none());
        }};
    }

    #[cfg(test)]
    macro_rules! assert_ptr_eq {
        ($a:expr, $b:expr) => {{
            // Deal with mapping a Pin<&mut T> -> Option<NonNull<T>>
            assert_eq!(Some($a.as_ref().get_ref().into()), $b)
        }};
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/util/linked_list/const_new_539.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/util/linked_list/push_and_drain_544.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/util/linked_list/push_pop_push_pop_564.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tokio/functions/src/util/linked_list/remove_by_address_586.rs"));

    /// This is a fuzz test. You run it by entering `cargo fuzz run fuzz_linked_list` in CLI in `/tokio/` module.
    #[cfg(fuzzing)]
    pub fn fuzz_linked_list(ops: &[u8]) {
        enum Op {
            Push,
            Pop,
            Remove(usize),
        }
        use std::collections::VecDeque;

        let ops = ops
            .iter()
            .map(|i| match i % 3u8 {
                0 => Op::Push,
                1 => Op::Pop,
                2 => Op::Remove((i / 3u8) as usize),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();

        let mut ll = LinkedList::<&Entry>::new();
        let mut reference = VecDeque::new();

        let entries: Vec<_> = (0..ops.len()).map(|i| entry(i as i32)).collect();

        for (i, op) in ops.iter().enumerate() {
            match op {
                Op::Push => {
                    reference.push_front(i as i32);
                    assert_eq!(entries[i].val, i as i32);

                    ll.push_front(entries[i].as_ref());
                }
                Op::Pop => {
                    if reference.is_empty() {
                        assert!(ll.is_empty());
                        continue;
                    }

                    let v = reference.pop_back();
                    assert_eq!(v, ll.pop_back().map(|v| v.val));
                }
                Op::Remove(n) => {
                    if reference.is_empty() {
                        assert!(ll.is_empty());
                        continue;
                    }

                    let idx = n % reference.len();
                    let expect = reference.remove(idx).unwrap();

                    unsafe {
                        let entry = ll.remove(ptr(&entries[expect as usize])).unwrap();
                        assert_eq!(expect, entry.val);
                    }
                }
            }
        }
    }
}
