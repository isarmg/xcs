#[cfg(any(test))]
mod padding_testutil {
    use crate::KnownLayout;

    #[derive(KnownLayout)]
    #[repr(C, align(4))]
    pub(super) struct Aligned<Prefix, Tail: ?Sized> {
        pub(super) prefix: Prefix,
        pub(super) tail: Tail,
    }

    /// Compare direct padding with the complete size minus the slice end.
    /// Counts whose complete object size exceeds `usize::MAX` are omitted.
    pub(super) fn check_padding_for_elems<Target>(elems: usize) -> Option<usize>
    where
        Target: ?Sized + KnownLayout<PointerMetadata = usize>,
    {
        let layout = crate::trailing_slice_layout::<Target>();
        let trailing_size = elems.checked_mul(layout.elem_size)?;

        // Evaluate the complete rounded size independently of both helpers:
        // `size_for_elems` delegates to the padding method being tested.
        let (alignment, phase) = layout.size_rounding_align_and_phase.components();
        #[allow(clippy::arithmetic_side_effects)]
        let mask = alignment.get() - 1;
        let unrounded = phase.checked_add(trailing_size)?;
        let rounded = unrounded.checked_add(mask)? & !mask;
        let object_size = layout.size_base.checked_add(rounded)?;
        let trailing_end = layout.offset.checked_add(trailing_size).unwrap();
        let expected = object_size.checked_sub(trailing_end).unwrap();

        let padding = layout.padding_for_elems(elems);
        assert_eq!(padding, expected);
        Some(padding)
    }

    pub(super) fn check_layouts(elems: usize) {
        // Slices have no trailing padding. The aligned fixtures cover
        // zero and nonzero phases, a three-byte element whose padding
        // cycles with the element count, and zero-sized elements.
        let _ = check_padding_for_elems::<[u8]>(elems);
        let _ = check_padding_for_elems::<[()]>(elems);
        let _ = check_padding_for_elems::<Aligned<[u8; 8], [u8]>>(elems);
        let _ = check_padding_for_elems::<Aligned<[u8; 9], [u8]>>(elems);
        let _ = check_padding_for_elems::<Aligned<[u8; 9], [[u8; 3]]>>(elems);
        let _ = check_padding_for_elems::<Aligned<[u8; 9], [()]>>(elems);
    }
}
