#[cfg(test)]
mod tests {
    use core::{cell::Cell, mem::ManuallyDrop};

    use crate::{Immutable, KnownLayout, SplitAt, Unalign, Unaligned};

    #[derive(KnownLayout, SplitAt, Immutable, Unaligned)]
    #[repr(C, packed)]
    struct Packed<T: ?Sized> {
        prefix: u8,
        tail: ManuallyDrop<T>,
    }

    #[derive(KnownLayout, SplitAt, Immutable)]
    #[repr(C, packed(2))]
    struct Packed2<T: ?Sized> {
        prefix: u8,
        tail: ManuallyDrop<T>,
    }

    #[derive(KnownLayout, SplitAt, Immutable)]
    #[repr(C)]
    struct Inner<T: ?Sized> {
        prefix: u32,
        tail: T,
    }

    #[test]
    fn test_split_at_packed() {
        use crate::{FromBytes, IntoBytes};

        #[derive(FromBytes, IntoBytes, KnownLayout, SplitAt, Immutable, Unaligned)]
        #[repr(C, packed)]
        struct Packet {
            prefix: u8,
            tail: [u16],
        }

        // The tail starts at an odd address, exercising unaligned reads and
        // writes on targets where `u16` has alignment 2.
        #[repr(align(8))]
        struct Bytes([u8; 9]);

        for i in 0..=4 {
            let mut bytes = Bytes([1; 9]);
            let packet = Packet::ref_from_bytes(&bytes.0).unwrap();
            let (left, right): (&Packet, &[Unalign<u16>]) =
                packet.split_at(i).unwrap().via_immutable();
            assert_eq!(core::mem::size_of_val(left), 1 + 2 * i);
            assert_eq!(right.len(), 4 - i);
            assert!(right.iter().all(|elem| elem.get() == 0x0101));
            assert!(left.split_at(i).is_some());
            assert!(left.split_at(i + 1).is_none());
            assert!(packet.split_at(5).is_none());

            let (_, right) = packet.split_at(i).unwrap().via_into_bytes();
            assert_eq!(right.len(), 4 - i);
            let (_, right) = packet.split_at(i).unwrap().via_no_dynamic_padding();
            assert_eq!(right.len(), 4 - i);

            let packet = Packet::mut_from_bytes(&mut bytes.0).unwrap();
            let (left, right): (&mut Packet, &mut [Unalign<u16>]) =
                packet.split_at_mut(i).unwrap().via_runtime_check().ok().unwrap();
            left.prefix = 2;
            for elem in right {
                *elem = Unalign::new(0x0202);
            }
            assert_eq!(bytes.0[0], 2);
            assert!(bytes.0[1..1 + 2 * i].iter().all(|&byte| byte == 1));
            assert!(bytes.0[1 + 2 * i..].iter().all(|&byte| byte == 2));
        }
    }

    #[test]
    fn test_split_at_nested_packed() {
        // A packing factor greater than 1 still exposes `Unalign` elements,
        // including when the factor is below the original element alignment.
        let mut words = Packed2 { prefix: 0, tail: ManuallyDrop::new([1u32, 2, 3]) };
        let dst: &mut Packed2<[u32]> = &mut words;
        let (left, right): (&mut _, &mut [Unalign<u32>]) =
            dst.split_at_mut(1).unwrap().via_no_dynamic_padding();
        left.prefix = 4;
        right[0] = Unalign::new(5);
        let dst: &Packed2<[u32]> = &words;
        let (left, right) = dst.split_at(0).unwrap().via_immutable();
        assert_eq!(left.prefix, 4);
        assert_eq!(right[0].get(), 1);
        assert_eq!(right[1].get(), 5);
        assert_eq!(right[2].get(), 3);

        let mut packet = Inner {
            prefix: 0,
            tail: Packed2 {
                prefix: 1,
                tail: ManuallyDrop::new(Packed {
                    prefix: 2,
                    tail: ManuallyDrop::new([3u16, 4, 5]),
                }),
            },
        };
        let dst: &Inner<Packed2<Packed<[u16]>>> = &packet;
        let (left, right): (&_, &[Unalign<Unalign<u16>>]) =
            dst.split_at(1).unwrap().via_immutable();
        assert_eq!(left.prefix, 0);
        assert_eq!(right.len(), 2);
        assert_eq!(right[0].get().get(), 4);
        assert_eq!(right[1].get().get(), 5);

        // Split the packed field separately: the unpacked outer struct's
        // padding does not constrain this split.
        let dst: &mut Packed2<Packed<[u16]>> = &mut packet.tail;
        let (left, right): (&mut _, &mut [Unalign<Unalign<u16>>]) =
            dst.split_at_mut(1).unwrap().via_no_dynamic_padding();
        left.prefix = 6;
        right[0] = Unalign::new(Unalign::new(7));
        let dst: &Inner<Packed2<Packed<[u16]>>> = &packet;
        let (_, right) = dst.split_at(0).unwrap().via_immutable();
        assert_eq!(right[0].get().get(), 3);
        assert_eq!(right[1].get().get(), 7);
        assert_eq!(right[2].get().get(), 5);
    }

    #[test]
    fn test_split_at_packed_preserves_inner_padding() {
        for i in 0..=4 {
            let mut packet = Packed {
                prefix: 1,
                tail: ManuallyDrop::new(Inner { prefix: 0, tail: [2u8, 3, 4, 5] }),
            };
            let dst: &Packed<Inner<[u8]>> = &packet;
            let (left, right): (&_, &[Unalign<u8>]) = dst.split_at(i).unwrap().via_immutable();
            assert_eq!(right.len(), 4 - i);
            assert_eq!(right.first().map(Unalign::get), [2, 3, 4, 5].get(i).copied());

            // The inner `u32` prefix makes the tail start at byte 5 of the
            // packed wrapper. Its size may still include inner padding.
            let has_padding = core::mem::size_of_val(left) != 5 + i;
            assert_eq!(dst.split_at(i).unwrap().via_runtime_check().is_err(), has_padding);

            let dst: &mut Packed<Inner<[u8]>> = &mut packet;
            let split = dst.split_at_mut(i).unwrap().via_runtime_check();
            assert_eq!(split.is_err(), has_padding);
            if let Ok((left, right)) = split {
                left.prefix = 6;
                for elem in right {
                    *elem = Unalign::new(7);
                }
                let dst: &Packed<Inner<[u8]>> = &packet;
                let (left, right) = dst.split_at(i).unwrap().via_immutable();
                assert_eq!(left.prefix, 6);
                assert!(right.iter().all(|elem| elem.get() == 7));
            }
        }
    }

    #[test]
    fn test_split_at_packed_interior_mutation_and_zsts() {
        let packet = Packed {
            prefix: 0,
            tail: ManuallyDrop::new([Cell::new(1u8), Cell::new(2), Cell::new(3)]),
        };
        let dst: &Packed<[Cell<u8>]> = &packet;
        let (left, right): (&_, &[Unalign<Cell<u8>>]) =
            dst.split_at(1).unwrap().via_no_dynamic_padding();
        left.tail[0].set(4);
        right[0].try_deref().unwrap().set(5);
        assert_eq!(dst.tail[0].get(), 4);
        assert_eq!(dst.tail[1].get(), 5);
        assert_eq!(dst.tail[2].get(), 3);

        let mut packet = Packed { prefix: 0, tail: ManuallyDrop::new([(); 3]) };
        for i in 0..=3 {
            let dst: &mut Packed<[()]> = &mut packet;
            let (left, right): (&mut _, &mut [Unalign<()>]) =
                dst.split_at_mut(i).unwrap().via_no_dynamic_padding();
            assert_eq!(left.tail.len(), i);
            assert_eq!(right.len(), 3 - i);
            assert_eq!(core::mem::size_of_val(left), 1);
        }
    }




    #[test]
    fn test_split_at_unchecked() {
        use crate::SplitAt;
        let mut arr = [1, 2, 3, 4];
        let slice = &arr[..];
        // SAFETY: 2 <= arr.len() (4)
        let split = unsafe { SplitAt::split_at_unchecked(slice, 2) };
        // SAFETY: SplitAt::split_at_unchecked guarantees that the split is valid.
        let (l, r) = unsafe { split.via_unchecked() };
        assert_eq!(l, &[1, 2]);
        assert_eq!(r, &[3, 4]);

        let slice_mut = &mut arr[..];
        // SAFETY: 2 <= arr.len() (4)
        let split = unsafe { SplitAt::split_at_mut_unchecked(slice_mut, 2) };
        // SAFETY: SplitAt::split_at_mut_unchecked guarantees that the split is valid.
        let (l, r) = unsafe { split.via_unchecked() };
        assert_eq!(l, &mut [1, 2]);
        assert_eq!(r, &mut [3, 4]);
    }

    #[test]
    fn test_split_at_via_methods() {
        use crate::{FromBytes, Immutable, IntoBytes, KnownLayout, SplitAt};
        #[derive(FromBytes, KnownLayout, SplitAt, IntoBytes, Immutable, Debug)]
        #[repr(C)]
        struct Packet {
            length: u8,
            body: [u8],
        }

        let arr = [1, 2, 3, 4];
        let packet = Packet::ref_from_bytes(&arr[..]).unwrap();

        let split1 = packet.split_at(2).unwrap();
        let (l, r) = split1.via_immutable();
        assert_eq!(l.length, 1);
        assert_eq!(r, &[4]);

        let split2 = packet.split_at(2).unwrap();
        let (l, r) = split2.via_into_bytes();
        assert_eq!(l.length, 1);
        assert_eq!(r, &[4]);
    }
    #[test]
    #[allow(deprecated)]
    fn test_split_at_via_unaligned() {
        use crate::{Immutable, KnownLayout, Split, SplitAt, Unaligned};

        fn via_unaligned<'a, T>(split: Split<&'a T>) -> (&'a T, &'a [T::Elem])
        where
            T: ?Sized + SplitAt + Unaligned,
        {
            split.via_unaligned()
        }

        fn via_unaligned_mut<'a, T>(split: Split<&'a mut T>) -> (&'a mut T, &'a mut [T::Elem])
        where
            T: ?Sized + SplitAt + Unaligned,
        {
            split.via_unaligned()
        }

        #[derive(KnownLayout, SplitAt, Immutable, Unaligned)]
        #[repr(C)]
        struct Packet<B: ?Sized> {
            prefix: [u8; 2],
            body: B,
        }

        // Exercise generic callers using the original `T: Unaligned` bound.
        assert!(!Packet::<[[u8; 2]]>::LAYOUT.requires_dynamic_padding());
        let packet = Packet { prefix: [0, 1], body: [[2, 3], [4, 5], [6, 7]] };
        let packet: &Packet<[[u8; 2]]> = &packet;

        let split = packet.split_at(2).unwrap();
        let (l, r) = via_unaligned(split);
        assert_eq!(l.body, [[2, 3], [4, 5]]);
        assert_eq!(r, &[[6, 7]]);

        let mut packet = Packet { prefix: [0, 1], body: [[2, 3], [4, 5], [6, 7]] };
        {
            let packet: &mut Packet<[[u8; 2]]> = &mut packet;
            let split = packet.split_at_mut(2).unwrap();
            let (l, r) = via_unaligned_mut(split);
            l.body[0] = [8, 9];
            r[0] = [10, 11];
        }
        assert_eq!(packet.body, [[8, 9], [4, 5], [10, 11]]);
    }

    #[test]
    fn test_split_at_via_no_dynamic_padding() {
        use core::cell::Cell;

        use crate::SplitAt;

        // `u16` does not implement `Unaligned`, but `[u16]` never requires
        // dynamic trailing padding.
        let words = [1u16, 2, 3, 4];
        let split = SplitAt::split_at(&words[..], 2).unwrap();
        let (left, right) = split.via_no_dynamic_padding();
        assert_eq!(left, [1, 2]);
        assert_eq!(right, [3, 4]);

        let mut words = [1u16, 2, 3, 4];
        let split = SplitAt::split_at_mut(&mut words[..], 2).unwrap();
        let (left, right) = split.via_no_dynamic_padding();
        left[0] = 5;
        right[0] = 6;
        assert_eq!(words, [5, 2, 6, 4]);

        // Exercise the safety-sensitive shared route with interior mutation.
        let cells = [Cell::new(1u16), Cell::new(2), Cell::new(3)];
        let split = SplitAt::split_at(&cells[..], 2).unwrap();
        let (left, right) = split.via_no_dynamic_padding();
        left[0].set(4);
        right[0].set(5);
        assert_eq!([cells[0].get(), cells[1].get(), cells[2].get()], [4, 2, 5]);
    }
}
