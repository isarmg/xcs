#[cfg(test)]
mod tests {
    use super::*;
    use crate::PointerMetadata as _;

    /// Models the size of nested `repr(C)` structs ending in a slice.
    ///
    /// `leading` lists structs from outermost to innermost. Each tuple gives
    /// the packing factor, minimum alignment, and prefix size before padding
    /// for the final field. `trailing_elem_size` and `trailing_alignment` give
    /// the slice element size and alignment. An empty `leading` describes the
    /// slice itself.
    ///
    /// The returned closure maps a slice length to the complete size, including
    /// padding at every nesting level. It returns `None` only on `usize`
    /// overflow; sizes above `isize::MAX` are permitted by this arithmetic model.
    /// Packing affects field placement, preserving each field's internal
    /// padding, even for descriptors that Rust would reject as types.
    ///
    /// # Panics
    ///
    /// Panics if any alignment or packing factor is not a power of two, a
    /// minimum alignment exceeds its packing factor, or the element size is
    /// not a multiple of the slice alignment.
    fn size_for_metadata_model(
        leading: &[(NonZeroUsize, NonZeroUsize, usize)],
        trailing_elem_size: usize,
        trailing_alignment: NonZeroUsize,
    ) -> impl Fn(usize) -> Option<usize> + '_ {
        // Rust requires power-of-two alignments and sizes divisible by their
        // alignment [1]; alignment modifiers also take powers of two [2].
        //
        // [1] https://doc.rust-lang.org/1.93.0/reference/type-layout.html#size-and-alignment
        // [2] https://doc.rust-lang.org/1.93.0/reference/type-layout.html#the-alignment-modifiers
        assert!(trailing_alignment.get().is_power_of_two());
        assert_eq!(trailing_elem_size % trailing_alignment, 0);
        for &(packed, align, _) in leading {
            assert!(packed.get().is_power_of_two());
            assert!(align.get().is_power_of_two());
            // The descriptor requires its minimum to fit under the packing
            // cap. It also models combinations that Rust forbids, including
            // simultaneous `align` and `packed` modifiers [2].
            assert!(align <= packed);
        }

        move |elems| {
            // A slice shares its array section's layout [3]. Array size is
            // element size times length, with the element's alignment [4].
            //
            // [3] https://doc.rust-lang.org/1.93.0/reference/type-layout.html#slice-layout
            // [4] https://doc.rust-lang.org/1.93.0/reference/type-layout.html#array-layout
            let mut size = trailing_elem_size.checked_mul(elems)?;
            let mut alignment = trailing_alignment;

            // Work outward with each field's complete size, including its
            // trailing padding: an enclosing representation "does not change
            // the layout of the fields themselves" [5]. Thus packing a nested
            // field preserves the padding already included in `size`.
            //
            // [5] https://doc.rust-lang.org/1.93.0/reference/type-layout.html#representations
            for &(packed, align, prefix) in leading.iter().rev() {
                // Packing caps the field's alignment for placement [2].
                let field_align = alignment.min(packed);

                // `repr(C)` places the next field at an aligned offset [6];
                // packing requires the least sufficient inter-field padding
                // [2]. Here `prefix` is the end of the preceding fields.
                //
                // [6] https://doc.rust-lang.org/1.93.0/reference/type-layout.html#reprc-structs
                let offset = prefix.checked_add(util::padding_needed_for(prefix, field_align))?;

                // `repr(C)` uses the greatest field alignment [6], and an
                // explicit alignment can only raise it [2]. Here `align`
                // summarizes the minimum from the prefix and any explicit
                // alignment. Both operands are <= `packed`, so their maximum
                // also respects the packing cap.
                alignment = align.max(field_align);

                // Advance past the complete field, then round to the struct's
                // alignment, following `repr(C)`'s final sizing steps [6].
                size = offset.checked_add(size)?;
                size = size.checked_add(util::padding_needed_for(size, alignment))?;
            }
            // Each enclosing size is at least its field's size. Thus any
            // checked multiplication or addition that overflowed above would
            // also make the final size unrepresentable in `usize`.
            Some(size)
        }
    }

    /// Computes [`size_for_metadata_model`]'s size function using `DstLayout`.
    ///
    /// Layout construction uses [`DstLayout::for_repr_c_struct`], and the
    /// returned closure uses [`crate::PointerMetadata::size_for_metadata`].
    ///
    /// # Panics
    ///
    /// Panics for invalid descriptors as in [`size_for_metadata_model`], or if
    /// `DstLayout` cannot construct the layout. This includes static size
    /// overflow and, in debug builds, alignments or packing factors above
    /// [`DstLayout::CURRENT_MAX_ALIGN`].
    fn size_for_metadata_via_dst_layout(
        leading: &[(NonZeroUsize, NonZeroUsize, usize)],
        trailing_elem_size: usize,
        trailing_alignment: NonZeroUsize,
    ) -> impl Fn(usize) -> Option<usize> + '_ {
        assert_eq!(trailing_elem_size % trailing_alignment, 0);
        let mut layout = DstLayout {
            align: trailing_alignment,
            size_info: SizeInfo::SliceDst(TrailingSliceLayout {
                offset: 0,
                elem_size: trailing_elem_size,
                size_base: 0,
                size_rounding_align_and_phase: RoundingAlignAndPhase::new(trailing_alignment, 0),
            }),
            statically_shallow_unpadded: true,
        };
        for &(packed, align, prefix) in leading.iter().rev() {
            assert!(align <= packed);
            let prefix = DstLayout {
                align: DstLayout::MIN_ALIGN,
                size_info: SizeInfo::Sized { size: prefix },
                statically_shallow_unpadded: true,
            };
            layout = DstLayout::for_repr_c_struct(Some(align), Some(packed), &[prefix, layout]);
        }
        move |elems: usize| elems.size_for_metadata(layout)
    }

    #[test]
    fn test_model_via_dst_layout() {
        let nz = |n| NonZeroUsize::new(n).unwrap();
        let cases: &[&[_]] = &[
            &[],
            &[(nz(8), nz(1), 3)],
            &[(nz(8), nz(8), 3)],
            &[(nz(2), nz(1), 1), (nz(8), nz(8), 3)],
            &[(nz(8), nz(8), 1), (nz(2), nz(1), 3), (nz(8), nz(8), 5)],
            &[(nz(1), nz(1), 1), (nz(2), nz(2), 3), (nz(8), nz(8), 5)],
        ];
        for &leading in cases {
            for &(elem_size, alignment) in &[(0, 1), (0, 8), (1, 1), (2, 2), (3, 1), (8, 8)] {
                let alignment = nz(alignment);
                let expected = size_for_metadata_model(leading, elem_size, alignment);
                let actual = size_for_metadata_via_dst_layout(leading, elem_size, alignment);
                let max_elems = usize::MAX / elem_size.max(1);
                for elems in (0..32).chain([
                    max_elems - 1,
                    max_elems,
                    max_elems.saturating_add(1),
                    usize::MAX,
                ]) {
                    assert_eq!(
                        actual(elems),
                        expected(elems),
                        "leading: {:?}, trailing: {:?}, elems: {}",
                        leading,
                        (elem_size, alignment),
                        elems,
                    );
                }
            }
        }

        for prefix in [usize::MAX - 1, usize::MAX] {
            let leading = [(nz(1), nz(1), prefix)];
            let expected = size_for_metadata_model(&leading, 1, nz(1));
            let actual = size_for_metadata_via_dst_layout(&leading, 1, nz(1));
            for elems in [0, 1, 2] {
                assert_eq!(actual(elems), expected(elems));
            }
        }
    }

    // This exercises arithmetic rather than unsafe operations; keep the
    // smaller comparison test enabled under Miri instead of this large search.
    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_model_via_dst_layout_combinations() {
        use rand::{rngs::SmallRng, Rng as _, SeedableRng as _};

        fn check(
            leading: &[(NonZeroUsize, NonZeroUsize, usize)],
            trailing_elem_size: usize,
            trailing_alignment: NonZeroUsize,
        ) {
            let expected = size_for_metadata_model(leading, trailing_elem_size, trailing_alignment);
            assert!(
                expected(0).is_some(),
                "unrepresentable test layout: {:?}, {:?}",
                leading,
                (trailing_elem_size, trailing_alignment)
            );
            let actual = std::panic::catch_unwind(|| {
                size_for_metadata_via_dst_layout(leading, trailing_elem_size, trailing_alignment)
            })
            .unwrap_or_else(|_| {
                panic!(
                    "DstLayout rejected leading: {:?}, trailing: {:?}",
                    leading,
                    (trailing_elem_size, trailing_alignment)
                )
            });
            let compare = |elems| {
                let expected = expected(elems);
                assert_eq!(
                    actual(elems),
                    expected,
                    "leading: {:?}, trailing: {:?}, elems: {}",
                    leading,
                    (trailing_elem_size, trailing_alignment),
                    elems,
                );
                expected
            };

            let first_sixteen = 0..=16;
            let pointer_width = (0..POINTER_WIDTH_BITS).step_by(4).flat_map(|shift| {
                let elems = 1usize << shift;
                [elems - 1, elems, elems + 1]
            });
            let last_two = (usize::MAX - 1)..=usize::MAX;
            let elemses = first_sixteen.chain(pointer_width).chain(last_two);
            for elems in elemses {
                let _ = compare(elems);
            }

            // The modeled size is nondecreasing. Find the last fitting length,
            // comparing every probe, then check both sides of that boundary.
            // Zero-sized elements can remain representable at usize::MAX.
            let (mut low, mut high) = (0, usize::MAX);
            while low < high {
                let distance = high - low;
                let mid = low + distance / 2 + distance % 2;
                if compare(mid).is_some() {
                    low = mid;
                } else {
                    high = mid - 1;
                }
            }
            assert!(compare(low).is_some());
            if let Some(previous) = low.checked_sub(1) {
                assert!(compare(previous).is_some());
            }
            if let Some(next) = low.checked_add(1) {
                assert_eq!(compare(next), None);
            }
        }

        let nz = |n| NonZeroUsize::new(n).unwrap();
        let check_small_tails = |leading: &[_]| {
            for alignment in [1, 2, 4, 8] {
                for multiple in [0, 1, 2, 3, 7] {
                    check(leading, multiple * alignment, nz(alignment));
                }
            }
        };
        let mut fragments = Vec::new();
        for packed in [1, 2, 4] {
            for align in [1, 2, 4].iter().copied().filter(|&align| align <= packed) {
                for prefix in 0..=4 {
                    fragments.push((nz(packed), nz(align), prefix));
                }
            }
        }

        // Exhaust every combination in this small domain at depths 0, 1, 2.
        check_small_tails(&[]);
        for &outer in &fragments {
            check_small_tails(&[outer]);
            for &inner in &fragments {
                check_small_tails(&[outer, inner]);
            }
        }

        let mut rng = SmallRng::seed_from_u64(0);
        for depth in (0usize..=128).chain([256, 512, 1024]) {
            // A small prefix adds at most 4 * max_align bytes per layer.
            // Reserve half of usize::MAX for a large outermost prefix so
            // construction remains representable even at the largest depth.
            let align_limit =
                DstLayout::CURRENT_MAX_ALIGN.get().min(usize::MAX / (8 * depth.max(1)));
            let alignments: Vec<_> = (0..POINTER_WIDTH_BITS)
                .map(|shift| 1usize << shift)
                .take_while(|&align| align <= align_limit)
                .collect();
            let last = alignments.len() - 1;
            let max_align = alignments[last];
            for case in 0..40 {
                let mut leading = Vec::with_capacity(depth);
                for level in 0..depth {
                    // Mix arbitrary modifiers, increasing and decreasing
                    // alignments, alternating packing, and fully packed trees.
                    let (packed, align) = match case / 8 {
                        0 => {
                            let packed = rng.gen_range(0..=last);
                            (packed, rng.gen_range(0..=packed))
                        }
                        1 => (last, level % alignments.len()),
                        2 => (last, last - level % alignments.len()),
                        3 => {
                            let index = if level % 2 == 0 { 0 } else { last };
                            (index, index)
                        }
                        _ => (0, 0),
                    };
                    let (packed, align) = (alignments[packed], alignments[align]);
                    let prefix = if level == 0 && case % 4 == 3 {
                        usize::MAX / 2 + rng.gen_range(0..=2) - 1
                    } else {
                        match rng.gen_range(0..8) {
                            0 => 0,
                            1 => 1,
                            2 => align - 1,
                            3 => align,
                            4 => align + 1,
                            5 => packed - 1,
                            6 => packed + 1,
                            _ => rng.gen_range(0..=2 * max_align),
                        }
                    };
                    leading.push((nz(packed), nz(align), prefix));
                }
                let alignment = alignments[rng.gen_range(0..=last)];
                let elem_size = match case % 8 {
                    0 => 0,
                    1 => alignment,
                    2 => 2 * alignment,
                    3 => 3 * alignment,
                    4 => 7 * alignment,
                    5 => 15 * alignment,
                    6 => (usize::MAX / 2 / alignment) * alignment,
                    _ => (usize::MAX / alignment) * alignment,
                };
                check(&leading, elem_size, nz(alignment));
            }
        }
    }

    const TEST_SIZE_ROUNDING_ALIGN_AND_PHASE_ALIGN: NonZeroUsize = match NonZeroUsize::new(8) {
        Some(align) => align,
        None => const_unreachable!(),
    };
    const TEST_SIZE_ROUNDING_ALIGN_AND_PHASE: RoundingAlignAndPhase =
        RoundingAlignAndPhase::new(TEST_SIZE_ROUNDING_ALIGN_AND_PHASE_ALIGN, 3);
    const TEST_SIZE_ROUNDING_ALIGN_AND_PHASE_DECODED_ALIGN: usize =
        TEST_SIZE_ROUNDING_ALIGN_AND_PHASE.align().get();
    const TEST_SIZE_ROUNDING_ALIGN_AND_PHASE_DECODED_PHASE: usize =
        TEST_SIZE_ROUNDING_ALIGN_AND_PHASE.components().1;

    #[test]
    #[allow(clippy::arithmetic_side_effects)]
    fn test_size_rounding_align_and_phase_encoding() {
        // These constants ensure that encoding and decoding remain evaluable
        // on every supported compiler, including the MSRV.
        assert_eq!(TEST_SIZE_ROUNDING_ALIGN_AND_PHASE_DECODED_ALIGN, 8);
        assert_eq!(TEST_SIZE_ROUNDING_ALIGN_AND_PHASE_DECODED_PHASE, 3);

        for shift in 0..POINTER_WIDTH_BITS {
            #[allow(clippy::arithmetic_side_effects)]
            let align = NonZeroUsize::new(1usize << shift).unwrap();
            for phase in [0, align.get() / 2, align.get() - 1] {
                let rounding = RoundingAlignAndPhase::new(align, phase);
                assert_eq!(rounding.align(), align);
                assert_eq!(rounding.components().1, phase);
                assert_eq!(rounding.0.get(), align.get() | phase);
            }
        }

        let max_align = DstLayout::THEORETICAL_MAX_ALIGN;
        let max_phase = max_align.get() - 1;
        let max = RoundingAlignAndPhase::new(max_align, max_phase);
        assert_eq!(max.0.get(), usize::MAX);
        assert_eq!(max.align(), max_align);
        assert_eq!(max.components().1, max_phase);

        assert_eq!(mem::size_of::<TrailingSliceLayout>(), 4 * mem::size_of::<usize>());
    }

    #[test]
    #[allow(clippy::arithmetic_side_effects)]
    fn test_trailing_slice_layout_normal_form() {
        // Compare the stored base-and-phase formula with
        // `prefix + round_up(offset + elems * elem_size, align)`, where
        // `prefix < align`. Move whole alignment-sized chunks of `offset`
        // into `size_base` and retain the remainder as the phase. Also verify
        // that `size_offset()` recovers the exact `offset` used by casts to
        // choose between metadata values with the same rounded object size.
        for align in [1, 2, 4, 8, 16] {
            let align = NonZeroUsize::new(align).unwrap();
            for prefix in 0..align.get() {
                for offset in 0..32 {
                    let phase = offset % align;
                    let size_base = (offset - phase) | prefix;
                    let layout = TrailingSliceLayout {
                        offset: 0,
                        elem_size: 3,
                        size_base,
                        size_rounding_align_and_phase: RoundingAlignAndPhase::new(align, phase),
                    };
                    assert_eq!(layout.size_offset(), offset);

                    for elems in 0..32 {
                        let unrounded = offset + elems * layout.elem_size;
                        let old_size =
                            prefix + unrounded + util::padding_needed_for(unrounded, align);
                        assert_eq!(layout.size_for_elems(elems), Some(old_size));
                    }
                }
            }
        }
    }

    #[test]
    #[allow(clippy::arithmetic_side_effects)]
    fn test_normalized_size_bounds() {
        // Use checked evaluation as an independent oracle, including layouts
        // whose empty object is already too large to represent.
        let checked_size = |layout: TrailingSliceLayout, trailing_bytes: usize| {
            let (alignment, phase) = layout.size_rounding_align_and_phase.components();
            let unrounded = phase.checked_add(trailing_bytes)?;
            let rounded = unrounded.checked_add(util::padding_needed_for(unrounded, alignment))?;
            layout.size_base.checked_add(rounded)
        };
        for exponent in 0..POINTER_WIDTH_BITS {
            let alignment = NonZeroUsize::new(1usize << exponent).unwrap();
            for phase in [0, alignment.get() / 2, alignment.get() - 1] {
                for base in [0, 1, alignment.get() - 1, alignment.get(), usize::MAX - 1, usize::MAX]
                {
                    let mut layout = TrailingSliceLayout {
                        offset: 0,
                        elem_size: 1,
                        size_base: base,
                        size_rounding_align_and_phase: RoundingAlignAndPhase::new(alignment, phase),
                    };
                    for available in [0, 1, alignment.get() - 1, alignment.get(), usize::MAX] {
                        match layout.max_trailing_bytes(available) {
                            Some(max_bytes) => {
                                assert!(checked_size(layout, max_bytes).unwrap() <= available);
                                if let Some(next_bytes) = max_bytes.checked_add(1) {
                                    assert!(checked_size(layout, next_bytes)
                                        .map(|size| size > available)
                                        .unwrap_or(true));
                                }
                            }
                            None => {
                                assert!(checked_size(layout, 0)
                                    .map(|size| size > available)
                                    .unwrap_or(true));
                            }
                        }
                    }

                    for elem_size in [0, 1, 3, alignment.get(), usize::MAX] {
                        layout.elem_size = elem_size;
                        let max_elems = layout
                            .max_trailing_bytes(usize::MAX)
                            .and_then(|bytes| bytes.checked_div(elem_size))
                            .unwrap_or(usize::MAX);
                        for elems in [
                            0,
                            1,
                            max_elems.saturating_sub(1),
                            max_elems,
                            max_elems.saturating_add(1),
                            usize::MAX,
                        ] {
                            let expected = elems
                                .checked_mul(elem_size)
                                .and_then(|bytes| checked_size(layout, bytes));
                            // The physical slice offset does not affect
                            // the size formula. Raw layouts may place the
                            // slice beyond the object, and adding its byte
                            // count to the offset may overflow.
                            for offset in [0, usize::MAX] {
                                layout.offset = offset;
                                assert_eq!(layout.size_for_elems(elems), expected);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_padding_for_elems_for_rust_layouts() {
        use super::padding_testutil::{check_layouts, check_padding_for_elems, Aligned};

        for elems in 0..256 {
            check_layouts(elems);
        }
        let size_limit = usize::MAX;
        for remaining in 0..16 {
            check_layouts(size_limit - remaining);
            check_layouts(size_limit / 3 - remaining);
        }
        check_layouts(usize::MAX);

        for (elems, expected) in [(0, 3), (1, 0), (2, 1), (3, 2)] {
            assert_eq!(
                check_padding_for_elems::<Aligned<[u8; 9], [[u8; 3]]>>(elems),
                Some(expected)
            );
        }
        // Every count is valid for zero-sized trailing elements.
        assert_eq!(check_padding_for_elems::<Aligned<[u8; 9], [()]>>(usize::MAX), Some(3));
    }

    #[test]
    fn test_padding_for_elems() {
        for exponent in 0..POINTER_WIDTH_BITS {
            let alignment = NonZeroUsize::new(1usize << exponent).unwrap();
            let mask = alignment.get() - 1;
            for phase in [0, alignment.get() / 2, mask] {
                // Include zero-sized elements, a negative fixed term, fixed
                // padding larger than the alignment, and overflowing sums
                // and products. Raw layouts need not describe Rust types.
                for (offset, elem_size, size_base) in [
                    (0, 0, 0),
                    (0, 1, 1),
                    (9, 3, 8),
                    (3, 3, 0),
                    (9, 3, 12),
                    (usize::MAX, usize::MAX, usize::MAX),
                    (usize::MAX, 0, usize::MAX),
                ] {
                    let layout = TrailingSliceLayout {
                        offset,
                        elem_size,
                        size_base,
                        size_rounding_align_and_phase: RoundingAlignAndPhase::new(alignment, phase),
                    };
                    for elems in [0, 1, 2, 3, alignment.get(), usize::MAX] {
                        // Independently reconstruct the complete rounded
                        // size and slice end, then subtract. Round up by
                        // adding the mask and clearing the low bits.
                        let trailing_bytes = elems.wrapping_mul(elem_size);
                        let unrounded = phase.wrapping_add(trailing_bytes);
                        let rounded = unrounded.wrapping_add(mask) & !mask;
                        let object_size = size_base.wrapping_add(rounded);
                        let trailing_end = offset.wrapping_add(trailing_bytes);
                        assert_eq!(
                            layout.padding_for_elems(elems),
                            object_size.wrapping_sub(trailing_end),
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_dst_layout_for_slice() {
        let layout = DstLayout::for_slice::<u32>();
        match layout.size_info {
            SizeInfo::SliceDst(TrailingSliceLayout { offset, elem_size, .. }) => {
                assert_eq!(offset, 0);
                assert_eq!(elem_size, 4);
            }
            _ => panic!("Expected SliceDst"),
        }
        assert_eq!(layout.align.get(), 4);
    }

    /// Tests of when a sized `DstLayout` is extended with a sized field.
    #[allow(clippy::decimal_literal_representation)]
    #[test]
    fn test_dst_layout_extend_sized_with_sized() {
        // This macro constructs a layout corresponding to a `u8` and extends
        // it with a zero-sized trailing field of the specified alignment. The
        // resulting size and alignment must both be the field's alignment
        // capped by the packing limit. Exercise every supported packing limit
        // as well as the absence of a packing limit.
        macro_rules! test_align_is_size {
            ($n:expr) => {
                let base = DstLayout::for_type::<u8>();
                let trailing_field = DstLayout::for_type::<elain::Align<$n>>();

                let packs = core::iter::once(None)
                    .chain((0..=29).map(|p| NonZeroUsize::new(2usize.pow(p))));

                for pack in packs {
                    let composite = base.extend(trailing_field, pack);
                    let max_align = pack.unwrap_or(DstLayout::CURRENT_MAX_ALIGN);
                    let align = $n.min(max_align.get());
                    assert_eq!(
                        composite,
                        DstLayout {
                            align: NonZeroUsize::new(align).unwrap(),
                            size_info: SizeInfo::Sized { size: align },
                            statically_shallow_unpadded: false,
                        }
                    )
                }
            };
        }

        test_align_is_size!(1);
        test_align_is_size!(2);
        test_align_is_size!(4);
        test_align_is_size!(8);
        test_align_is_size!(16);
        test_align_is_size!(32);
        test_align_is_size!(64);
        test_align_is_size!(128);
        test_align_is_size!(256);
        test_align_is_size!(512);
        test_align_is_size!(1024);
        test_align_is_size!(2048);
        test_align_is_size!(4096);
        test_align_is_size!(8192);
        test_align_is_size!(16384);
        test_align_is_size!(32768);
        test_align_is_size!(65536);
        test_align_is_size!(131072);
        test_align_is_size!(262144);
        test_align_is_size!(524288);
        test_align_is_size!(1048576);
        test_align_is_size!(2097152);
        test_align_is_size!(4194304);
        test_align_is_size!(8388608);
        test_align_is_size!(16777216);
        test_align_is_size!(33554432);
        test_align_is_size!(67108864);
        test_align_is_size!(33554432);
        test_align_is_size!(134217728);
        test_align_is_size!(268435456);
        test_align_is_size!(536870912);
    }

    /// Tests of when a sized `DstLayout` is extended with a DST field.
    #[test]
    fn test_dst_layout_extend_sized_with_dst() {
        // Test that for all combinations of real-world alignments and
        // `repr_packed` values, that the extension of a sized `DstLayout`` with
        // a DST field correctly computes the trailing offset in the composite
        // layout.

        let aligns = (0..29).map(|p| NonZeroUsize::new(2usize.pow(p)).unwrap());
        let packs = core::iter::once(None).chain(aligns.clone().map(Some));

        for field_type_align in aligns {
            for pack in packs.clone() {
                let base = DstLayout::for_type::<u8>();
                let elem_size = 42;
                let trailing_field_offset = 11;
                #[allow(clippy::arithmetic_side_effects)]
                let trailing_phase = trailing_field_offset % field_type_align;
                let trailing_base = util::round_down_to_next_multiple_of_alignment(
                    trailing_field_offset,
                    field_type_align,
                );

                let trailing_field = DstLayout {
                    align: field_type_align,
                    size_info: SizeInfo::SliceDst(TrailingSliceLayout {
                        elem_size,
                        offset: trailing_field_offset,
                        size_base: trailing_base,
                        size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                            field_type_align,
                            trailing_phase,
                        ),
                    }),
                    statically_shallow_unpadded: false,
                };

                let composite = base.extend(trailing_field, pack);

                let max_align = pack.unwrap_or(DstLayout::CURRENT_MAX_ALIGN).get();

                let align = field_type_align.get().min(max_align);
                let field_offset = align;
                let size_base = field_offset + trailing_base;

                assert_eq!(
                    composite,
                    DstLayout {
                        align: NonZeroUsize::new(align).unwrap(),
                        size_info: SizeInfo::SliceDst(TrailingSliceLayout {
                            elem_size,
                            offset: field_offset + trailing_field_offset,
                            size_base,
                            size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                                field_type_align,
                                trailing_phase,
                            ),
                        }),
                        statically_shallow_unpadded: false,
                    }
                )
            }
        }
    }

    /// Tests that calling `pad_to_align` on a sized `DstLayout` adds the
    /// expected amount of trailing padding.
    #[test]
    fn test_dst_layout_pad_to_align_with_sized() {
        // For all valid alignments `align`, construct a one-byte layout aligned
        // to `align`, call `pad_to_align`, and assert that the size of the
        // resulting layout is equal to `align`.
        for align in (0..29).map(|p| NonZeroUsize::new(2usize.pow(p)).unwrap()) {
            let layout = DstLayout {
                align,
                size_info: SizeInfo::Sized { size: 1 },
                statically_shallow_unpadded: true,
            };

            assert_eq!(
                layout.pad_to_align(),
                DstLayout {
                    align,
                    size_info: SizeInfo::Sized { size: align.get() },
                    statically_shallow_unpadded: align.get() == 1
                }
            );
        }

        // Test explicitly-provided combinations of unpadded and padded
        // counterparts.

        macro_rules! test {
            (unpadded { size: $unpadded_size:expr, align: $unpadded_align:expr }
                    => padded { size: $padded_size:expr, align: $padded_align:expr }) => {
                let unpadded = DstLayout {
                    align: NonZeroUsize::new($unpadded_align).unwrap(),
                    size_info: SizeInfo::Sized { size: $unpadded_size },
                    statically_shallow_unpadded: false,
                };
                let padded = unpadded.pad_to_align();

                assert_eq!(
                    padded,
                    DstLayout {
                        align: NonZeroUsize::new($padded_align).unwrap(),
                        size_info: SizeInfo::Sized { size: $padded_size },
                        statically_shallow_unpadded: false,
                    }
                );
            };
        }

        test!(unpadded { size: 0, align: 4 } => padded { size: 0, align: 4 });
        test!(unpadded { size: 1, align: 4 } => padded { size: 4, align: 4 });
        test!(unpadded { size: 2, align: 4 } => padded { size: 4, align: 4 });
        test!(unpadded { size: 3, align: 4 } => padded { size: 4, align: 4 });
        test!(unpadded { size: 4, align: 4 } => padded { size: 4, align: 4 });
        test!(unpadded { size: 5, align: 4 } => padded { size: 8, align: 4 });
        test!(unpadded { size: 6, align: 4 } => padded { size: 8, align: 4 });
        test!(unpadded { size: 7, align: 4 } => padded { size: 8, align: 4 });
        test!(unpadded { size: 8, align: 4 } => padded { size: 8, align: 4 });

        let current_max_align = DstLayout::CURRENT_MAX_ALIGN.get();

        test!(unpadded { size: 1, align: current_max_align }
                => padded { size: current_max_align, align: current_max_align });

        test!(unpadded { size: current_max_align + 1, align: current_max_align }
                => padded { size: current_max_align * 2, align: current_max_align });
    }

    /// Tests that calling `pad_to_align` on a DST `DstLayout` incorporates the
    /// type's trailing-padding operation into its runtime size formula.
    #[test]
    fn test_dst_layout_pad_to_align_with_dst() {
        for align in (0..29).map(|p| NonZeroUsize::new(2usize.pow(p)).unwrap()) {
            for offset in 0..10 {
                for elem_size in 0..10 {
                    let layout = DstLayout {
                        align,
                        size_info: SizeInfo::SliceDst(TrailingSliceLayout {
                            offset,
                            elem_size,
                            size_base: offset,
                            size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                                DstLayout::MIN_ALIGN,
                                0,
                            ),
                        }),
                        statically_shallow_unpadded: false,
                    };
                    assert_eq!(
                        layout.pad_to_align(),
                        DstLayout {
                            align,
                            size_info: SizeInfo::SliceDst(TrailingSliceLayout {
                                offset,
                                elem_size,
                                size_base: offset - offset % align,
                                size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                                    align,
                                    offset % align
                                ),
                            }),
                            statically_shallow_unpadded: false,
                        }
                    );
                }
            }
        }

        // Exercise the branch where the existing inner rounding alignment is
        // at least the enclosing alignment. The inner rounded term is already
        // aligned to two, so padding only advances `size_base` from three to
        // four.
        let inner_rounding_dominates = DstLayout {
            align: NonZeroUsize::new(2).unwrap(),
            size_info: SizeInfo::SliceDst(TrailingSliceLayout {
                offset: 3,
                elem_size: 1,
                size_base: 3,
                size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                    NonZeroUsize::new(4).unwrap(),
                    1,
                ),
            }),
            statically_shallow_unpadded: false,
        };
        assert_eq!(
            inner_rounding_dominates.pad_to_align(),
            DstLayout {
                align: NonZeroUsize::new(2).unwrap(),
                size_info: SizeInfo::SliceDst(TrailingSliceLayout {
                    offset: 3,
                    elem_size: 1,
                    size_base: 4,
                    size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                        NonZeroUsize::new(4).unwrap(),
                        1
                    ),
                }),
                statically_shallow_unpadded: false,
            }
        );

        // Exercise the branch where the enclosing alignment is larger. For
        // a trailing-slice byte count `trailing_size`, rounding
        // `6 + round_up(1 + trailing_size, 4)` to eight gives the equivalent
        // formula `8 + round_up(1 + trailing_size, 8)`.
        let outer_rounding_dominates = DstLayout {
            align: NonZeroUsize::new(8).unwrap(),
            size_info: SizeInfo::SliceDst(TrailingSliceLayout {
                offset: 7,
                elem_size: 1,
                size_base: 6,
                size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                    NonZeroUsize::new(4).unwrap(),
                    1,
                ),
            }),
            statically_shallow_unpadded: false,
        };
        assert_eq!(
            outer_rounding_dominates.pad_to_align(),
            DstLayout {
                align: NonZeroUsize::new(8).unwrap(),
                size_info: SizeInfo::SliceDst(TrailingSliceLayout {
                    offset: 7,
                    elem_size: 1,
                    size_base: 8,
                    size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                        NonZeroUsize::new(8).unwrap(),
                        1
                    ),
                }),
                statically_shallow_unpadded: false,
            }
        );
    }

    #[test]
    fn test_trailing_slice_layout_size_sequence_equivalence() {
        // These are the two normalized descriptions produced by a
        // `repr(C, packed(2))` wrapper around slice DSTs whose element types
        // both have size four but alignments four and two respectively. Their
        // formulas differ structurally, but both simplify to `2 + 4 * elems`.
        let align4 = TrailingSliceLayout {
            offset: 2,
            elem_size: 4,
            size_base: 2,
            size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                NonZeroUsize::new(4).unwrap(),
                0,
            ),
        };
        let align2 = TrailingSliceLayout {
            offset: 2,
            elem_size: 4,
            size_base: 2,
            size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                NonZeroUsize::new(2).unwrap(),
                0,
            ),
        };
        assert!(align4.has_same_size_sequence(align2));
        for elems in 0..8 {
            assert_eq!(align4.size_for_elems(elems), Some(2 + 4 * elems));
            assert_eq!(align4.size_for_elems(elems), align2.size_for_elems(elems));
        }

        let dynamically_padded = TrailingSliceLayout { elem_size: 1, ..align4 };
        assert!(!dynamically_padded.has_same_size_sequence(align2));
    }

    // This test takes a long time when running under Miri, so we skip it in
    // that case. This is acceptable because this is a logic test that doesn't
    // attempt to expose UB.
    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_validate_cast_and_convert_metadata() {
        #[allow(non_local_definitions)]
        impl From<usize> for SizeInfo {
            fn from(size: usize) -> SizeInfo {
                SizeInfo::Sized { size }
            }
        }

        #[allow(non_local_definitions)]
        impl From<(usize, usize)> for SizeInfo {
            fn from((offset, elem_size): (usize, usize)) -> SizeInfo {
                SizeInfo::SliceDst(TrailingSliceLayout {
                    offset,
                    elem_size,
                    size_base: offset,
                    size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                        DstLayout::MIN_ALIGN,
                        0,
                    ),
                })
            }
        }

        fn layout<S: Into<SizeInfo>>(s: S, align: usize) -> DstLayout {
            let align = NonZeroUsize::new(align).unwrap();
            let size_info = match s.into() {
                SizeInfo::SliceDst(mut trailing) => {
                    let phase = trailing.size_base % align;
                    trailing.size_base =
                        util::round_down_to_next_multiple_of_alignment(trailing.size_base, align);
                    trailing.size_rounding_align_and_phase =
                        RoundingAlignAndPhase::new(align, phase);
                    SizeInfo::SliceDst(trailing)
                }
                size_info => size_info,
            };
            DstLayout { size_info, align, statically_shallow_unpadded: false }
        }

        /// This macro accepts arguments in the form of:
        ///
        ///           layout(_, _).validate(_, _, _), Ok(Some((_, _)))
        ///                  |  |           |  |  |            |  |
        ///    size ---------+  |           |  |  |            |  |
        ///    align -----------+           |  |  |            |  |
        ///    addr ------------------------+  |  |            |  |
        ///    bytes_len ----------------------+  |            |  |
        ///    cast_type -------------------------+            |  |
        ///    elems ------------------------------------------+  |
        ///    split_at ------------------------------------------+
        ///
        /// `.validate` is shorthand for `.validate_cast_and_convert_metadata`
        /// for brevity.
        ///
        /// Each argument can either be an iterator or a wildcard. Each
        /// wildcarded variable is implicitly replaced by an iterator over a
        /// representative sample of values for that variable. Each `test!`
        /// invocation iterates over every combination of values provided by
        /// each variable's iterator (ie, the cartesian product) and validates
        /// that the results are expected.
        ///
        /// The final argument uses the same syntax, but it has a different
        /// meaning:
        /// - If it is `Ok(pat)`, then the pattern `pat` is supplied to
        ///   a matching assert to validate the computed result for each
        ///   combination of input values.
        /// - If it is `Err(Some(msg) | None)`, then `test!` validates that the
        ///   call to `validate_cast_and_convert_metadata` panics with the given
        ///   panic message or, if the current Rust toolchain version is too
        ///   early to support panicking in `const fn`s, panics with *some*
        ///   message. In the latter case, the `const_panic!` macro is used,
        ///   which emits code which causes a non-panicking error at const eval
        ///   time, but which does panic when invoked at runtime. Thus, it is
        ///   merely difficult to predict the *value* of this panic. We deem
        ///   that testing against the real panic strings on stable and nightly
        ///   toolchains is enough to ensure correctness.
        ///
        /// Note that the meta-variables that match these variables have the
        /// `tt` type, and some valid expressions are not valid `tt`s (such as
        /// `a..b`). In this case, wrap the expression in parentheses, and it
        /// will become valid `tt`.
        macro_rules! test {
                (
                    layout($size:tt, $align:tt)
                    .validate($addr:tt, $bytes_len:tt, $cast_type:tt), $expect:pat $(,)?
                ) => {
                    iterator_toolkit::iproduct!(
                        test!(@generate_size $size),
                        test!(@generate_align $align),
                        test!(@generate_usize $addr),
                        test!(@generate_usize $bytes_len),
                        test!(@generate_cast_type $cast_type)
                    ).for_each(|(size_info, align, addr, bytes_len, cast_type)| {
                        // Temporarily disable the panic hook installed by the test
                        // harness. If we don't do this, all panic messages will be
                        // kept in an internal log. On its own, this isn't a
                        // problem, but if a non-caught panic ever happens (ie, in
                        // code later in this test not in this macro), all of the
                        // previously-buffered messages will be dumped, hiding the
                        // real culprit.
                        let previous_hook = std::panic::take_hook();
                        // I don't understand why, but this seems to be required in
                        // addition to the previous line.
                        std::panic::set_hook(Box::new(|_| {}));
                        let actual = std::panic::catch_unwind(|| {
                            layout(size_info, align).validate_cast_and_convert_metadata(addr, bytes_len, cast_type)
                        }).map_err(|d| {
                            let msg = d.downcast::<&'static str>().ok().map(|s| *s.as_ref());
                            assert!(msg.is_some() || cfg!(no_zerocopy_panic_in_const_and_vec_try_reserve_1_57_0), "non-string panic messages are not permitted when usage of panic in const fn is enabled");
                            msg
                        });
                        std::panic::set_hook(previous_hook);

                        assert!(
                            matches!(actual, $expect),
                            "layout({:?}, {}).validate_cast_and_convert_metadata({}, {}, {:?})" ,size_info, align, addr, bytes_len, cast_type
                        );
                    });
                };
                (@generate_usize _) => { 0..8 };
                // Generate sizes for both Sized and !Sized types.
                (@generate_size _) => {
                    test!(@generate_size (_)).chain(test!(@generate_size (_, _)))
                };
                // Generate sizes for both Sized and !Sized types by chaining
                // specified iterators for each.
                (@generate_size ($sized_sizes:tt | $unsized_sizes:tt)) => {
                    test!(@generate_size ($sized_sizes)).chain(test!(@generate_size $unsized_sizes))
                };
                // Generate sizes for Sized types.
                (@generate_size (_)) => { test!(@generate_size (0..8)) };
                (@generate_size ($sizes:expr)) => { $sizes.into_iter().map(Into::<SizeInfo>::into) };
                // Generate sizes for !Sized types.
                (@generate_size ($min_sizes:tt, $elem_sizes:tt)) => {
                    iterator_toolkit::iproduct!(
                        test!(@generate_min_size $min_sizes),
                        test!(@generate_elem_size $elem_sizes)
                    ).map(Into::<SizeInfo>::into)
                };
                (@generate_fixed_size _) => { (0..8).into_iter().map(Into::<SizeInfo>::into) };
                (@generate_min_size _) => { 0..8 };
                (@generate_elem_size _) => { 1..8 };
                (@generate_align _) => { [1, 2, 4, 8, 16] };
                (@generate_opt_usize _) => { [None].into_iter().chain((0..8).map(Some).into_iter()) };
                (@generate_cast_type _) => { [CastType::Prefix, CastType::Suffix] };
                (@generate_cast_type $variant:ident) => { [CastType::$variant] };
                // Some expressions need to be wrapped in parentheses in order to be
                // valid `tt`s (required by the top match pattern). See the comment
                // below for more details. This arm removes these parentheses to
                // avoid generating an `unused_parens` warning.
                (@$_:ident ($vals:expr)) => { $vals };
                (@$_:ident $vals:expr) => { $vals };
            }

        const EVENS: [usize; 8] = [0, 2, 4, 6, 8, 10, 12, 14];
        const ODDS: [usize; 8] = [1, 3, 5, 7, 9, 11, 13, 15];

        // base_size is too big for the memory region.
        test!(
            layout(((1..8) | ((1..8), (1..8))), _).validate([0], [0], _),
            Ok(Err(MetadataCastError::Size))
        );
        test!(
            layout(((2..8) | ((2..8), (2..8))), _).validate([0], [1], Prefix),
            Ok(Err(MetadataCastError::Size))
        );
        test!(
            layout(((2..8) | ((2..8), (2..8))), _).validate([0x1000_0000 - 1], [1], Suffix),
            Ok(Err(MetadataCastError::Size))
        );

        // addr is unaligned for prefix cast
        test!(layout(_, [2]).validate(ODDS, _, Prefix), Ok(Err(MetadataCastError::Alignment)));
        test!(layout(_, [2]).validate(ODDS, _, Prefix), Ok(Err(MetadataCastError::Alignment)));

        // addr is aligned, but end of buffer is unaligned for suffix cast
        test!(layout(_, [2]).validate(EVENS, ODDS, Suffix), Ok(Err(MetadataCastError::Alignment)));
        test!(layout(_, [2]).validate(EVENS, ODDS, Suffix), Ok(Err(MetadataCastError::Alignment)));

        // Unfortunately, these constants cannot easily be used in the
        // implementation of `validate_cast_and_convert_metadata`, since
        // `panic!` consumes a string literal, not an expression.
        //
        // It's important that these messages be in a separate module. If they
        // were at the function's top level, we'd pass them to `test!` as, e.g.,
        // `Err(TRAILING)`, which would run into a subtle Rust footgun - the
        // `TRAILING` identifier would be treated as a pattern to match rather
        // than a value to check for equality.
        mod msgs {
            pub(super) const TRAILING: &str =
                "attempted to cast to slice type with zero-sized element";
            pub(super) const OVERFLOW: &str = "`addr` + `bytes_len` > usize::MAX";
        }

        // casts with ZST trailing element types are unsupported
        test!(layout((_, [0]), _).validate(_, _, _), Err(Some(msgs::TRAILING) | None),);

        // addr + bytes_len must not overflow usize
        test!(layout(_, _).validate([usize::MAX], (1..100), _), Err(Some(msgs::OVERFLOW) | None));
        test!(layout(_, _).validate((1..100), [usize::MAX], _), Err(Some(msgs::OVERFLOW) | None));
        test!(
            layout(_, _).validate(
                [usize::MAX / 2 + 1, usize::MAX],
                [usize::MAX / 2 + 1, usize::MAX],
                _
            ),
            Err(Some(msgs::OVERFLOW) | None)
        );

        // Validates that `validate_cast_and_convert_metadata` satisfies its own
        // documented safety postconditions, and also a few other properties
        // that aren't documented but we want to guarantee anyway.
        fn validate_behavior(
            (layout, addr, bytes_len, cast_type): (DstLayout, usize, usize, CastType),
        ) {
            if let Ok((elems, split_at)) =
                layout.validate_cast_and_convert_metadata(addr, bytes_len, cast_type)
            {
                let (size_info, align) = (layout.size_info, layout.align);
                let debug_str = format!(
                    "layout({:?}, {}).validate_cast_and_convert_metadata({}, {}, {:?}) => ({}, {})",
                    size_info, align, addr, bytes_len, cast_type, elems, split_at
                );

                // If this is a sized type (no trailing slice), then `elems` is
                // meaningless, but in practice we set it to 0. Callers are not
                // allowed to rely on this, but a lot of math is nicer if
                // they're able to, and some callers might accidentally do that.
                let sized = matches!(layout.size_info, SizeInfo::Sized { .. });
                assert!(!(sized && elems != 0), "{}", debug_str);

                let resulting_size = match layout.size_info {
                    SizeInfo::Sized { size } => size,
                    SizeInfo::SliceDst(TrailingSliceLayout {
                        elem_size,
                        size_base,
                        size_rounding_align_and_phase,
                        ..
                    }) => {
                        let (size_align, size_phase) = size_rounding_align_and_phase.components();
                        let padded_size = |elems| {
                            let without_padding = size_phase + elems * elem_size;
                            size_base
                                + without_padding
                                + util::padding_needed_for(without_padding, size_align)
                        };

                        let resulting_size = padded_size(elems);
                        // Test that `validate_cast_and_convert_metadata`
                        // computed the largest possible value that fits in the
                        // given range.
                        assert!(padded_size(elems + 1) > bytes_len, "{}", debug_str);
                        resulting_size
                    }
                };

                // Test safety postconditions guaranteed by
                // `validate_cast_and_convert_metadata`.
                assert!(resulting_size <= bytes_len, "{}", debug_str);
                match cast_type {
                    CastType::Prefix => {
                        assert_eq!(addr % align, 0, "{}", debug_str);
                        assert_eq!(resulting_size, split_at, "{}", debug_str);
                    }
                    CastType::Suffix => {
                        assert_eq!(split_at, bytes_len - resulting_size, "{}", debug_str);
                        assert_eq!((addr + split_at) % align, 0, "{}", debug_str);
                    }
                }
            } else {
                let min_size = match layout.size_info {
                    SizeInfo::Sized { size } => size,
                    SizeInfo::SliceDst(TrailingSliceLayout {
                        size_base,
                        size_rounding_align_and_phase,
                        ..
                    }) => {
                        let (size_align, size_phase) = size_rounding_align_and_phase.components();
                        size_base + size_phase + util::padding_needed_for(size_phase, size_align)
                    }
                };

                // If a cast is invalid, it is either because...
                // 1. there are insufficient bytes at the given region for type:
                let insufficient_bytes = bytes_len < min_size;
                // 2. performing the cast would misalign type:
                let base = match cast_type {
                    CastType::Prefix => 0,
                    CastType::Suffix => bytes_len,
                };
                let misaligned = (base + addr) % layout.align != 0;

                assert!(insufficient_bytes || misaligned);
            }
        }

        let sizes = 0..8;
        let elem_sizes = 1..8;
        let size_infos = sizes
            .clone()
            .map(Into::<SizeInfo>::into)
            .chain(iterator_toolkit::iproduct!(sizes, elem_sizes).map(Into::<SizeInfo>::into));
        let layouts = iterator_toolkit::iproduct!(size_infos, [1, 2, 4, 8, 16, 32])
                .filter(|(size_info, align)| !matches!(size_info, SizeInfo::Sized { size } if size % align != 0))
                .map(|(size_info, align)| layout(size_info, align));
        iterator_toolkit::iproduct!(layouts, 0..8, 0..8, [CastType::Prefix, CastType::Suffix])
            .for_each(validate_behavior);

        // Exercise a normalized formula which cannot be represented as
        // `round_up(offset + elems * elem_size, align)`: the physical trailing
        // slice begins at offset 7, while an inner aligned DST contributes the
        // size formula `2 + round_up(5 + elems, 4)` to a packed outer type.
        let nested_packed = DstLayout {
            align: NonZeroUsize::new(2).unwrap(),
            size_info: SizeInfo::SliceDst(TrailingSliceLayout {
                offset: 7,
                elem_size: 1,
                size_base: 6,
                size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                    NonZeroUsize::new(4).unwrap(),
                    1,
                ),
            }),
            statically_shallow_unpadded: false,
        };
        iterator_toolkit::iproduct!([nested_packed], 0..8, 0..24, [CastType::Prefix, CastType::Suffix])
            .for_each(validate_behavior);

        assert!(matches!(
            nested_packed.validate_cast_and_convert_metadata(0, 8, CastType::Prefix),
            Err(MetadataCastError::Size)
        ));
        assert!(matches!(
            nested_packed.validate_cast_and_convert_metadata(0, 10, CastType::Prefix),
            Ok((3, 10))
        ));
    }

    #[test]
    #[cfg(__ZEROCOPY_INTERNAL_USE_ONLY_NIGHTLY_FEATURES_IN_TESTS)]
    fn test_validate_rust_layout() {
        use core::{
            convert::TryInto as _,
            ptr::{self, NonNull},
        };

        use crate::util::testutil::*;

        // This test synthesizes pointers with various metadata and uses Rust's
        // built-in APIs to confirm that Rust makes decisions about type layout
        // which are consistent with what we believe is guaranteed by the
        // language. If this test fails, it doesn't just mean our code is wrong
        // - it means we're misunderstanding the language's guarantees.

        #[derive(Debug)]
        struct MacroArgs {
            offset: usize,
            align: NonZeroUsize,
            elem_size: Option<usize>,
        }

        /// # Safety
        ///
        /// `test` promises to only call `addr_of_slice_field` on a `NonNull<T>`
        /// which points to a valid `T`.
        ///
        /// `with_elems` must produce a pointer which points to a valid `T`.
        fn test<T: ?Sized, W: Fn(usize) -> NonNull<T>>(
            args: MacroArgs,
            with_elems: W,
            addr_of_slice_field: Option<fn(NonNull<T>) -> NonNull<u8>>,
        ) {
            let dst = args.elem_size.is_some();
            let layout = {
                let size_info = match args.elem_size {
                    Some(elem_size) => SizeInfo::SliceDst(TrailingSliceLayout {
                        offset: args.offset,
                        elem_size,
                        size_base: util::round_down_to_next_multiple_of_alignment(
                            args.offset,
                            args.align,
                        ),
                        size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                            args.align,
                            args.offset % args.align,
                        ),
                    }),
                    None => SizeInfo::Sized {
                        // Rust only supports types whose sizes are a multiple
                        // of their alignment. If the macro created a type like
                        // this:
                        //
                        //   #[repr(C, align(2))]
                        //   struct Foo([u8; 1]);
                        //
                        // ...then Rust will automatically round the type's size
                        // up to 2.
                        size: args.offset + util::padding_needed_for(args.offset, args.align),
                    },
                };
                DstLayout { size_info, align: args.align, statically_shallow_unpadded: false }
            };

            for elems in 0..128 {
                let ptr = with_elems(elems);

                if let Some(addr_of_slice_field) = addr_of_slice_field {
                    let slc_field_ptr = addr_of_slice_field(ptr).as_ptr();
                    // SAFETY: Both `slc_field_ptr` and `ptr` are pointers to
                    // the same valid Rust object.
                    // Work around https://github.com/rust-lang/rust-clippy/issues/12280
                    let offset: usize =
                        unsafe { slc_field_ptr.byte_offset_from(ptr.as_ptr()).try_into().unwrap() };
                    assert_eq!(offset, args.offset);
                }

                // SAFETY: `ptr` points to a valid `T`.
                #[allow(clippy::multiple_unsafe_ops_per_block)]
                let (size, align) = unsafe {
                    (mem::size_of_val_raw(ptr.as_ptr()), mem::align_of_val_raw(ptr.as_ptr()))
                };

                // Avoid expensive allocation when running under Miri.
                let assert_msg = if !cfg!(miri) {
                    format!("\n{:?}\nsize:{}, align:{}", args, size, align)
                } else {
                    String::new()
                };

                let without_padding =
                    args.offset + args.elem_size.map(|elem_size| elems * elem_size).unwrap_or(0);
                assert!(size >= without_padding, "{}", assert_msg);
                assert_eq!(align, args.align.get(), "{}", assert_msg);

                // This encodes the most important part of the test: our
                // understanding of how Rust determines the layout of repr(C)
                // types. Sized repr(C) types are trivial, but DST types have
                // some subtlety. Note that:
                // - For sized types, `without_padding` is just the size of the
                //   type that we constructed for `Foo`. Since we may have
                //   requested a larger alignment, `Foo` may actually be larger
                //   than this, hence `padding_needed_for`.
                // - For unsized types, `without_padding` is dynamically
                //   computed from the offset, the element size, and element
                //   count. We expect that the size of the object should be
                //   `offset + elem_size * elems` rounded up to the next
                //   alignment.
                let expected_size =
                    without_padding + util::padding_needed_for(without_padding, args.align);
                assert_eq!(expected_size, size, "{}", assert_msg);

                // For zero-sized element types,
                // `validate_cast_and_convert_metadata` just panics, so we skip
                // testing those types.
                if args.elem_size.map(|elem_size| elem_size > 0).unwrap_or(true) {
                    let addr = ptr.addr().get();
                    let (got_elems, got_split_at) = layout
                        .validate_cast_and_convert_metadata(addr, size, CastType::Prefix)
                        .unwrap();
                    // Avoid expensive allocation when running under Miri.
                    let assert_msg = if !cfg!(miri) {
                        format!(
                            "{}\nvalidate_cast_and_convert_metadata({}, {})",
                            assert_msg, addr, size,
                        )
                    } else {
                        String::new()
                    };
                    assert_eq!(got_split_at, size, "{}", assert_msg);
                    if dst {
                        assert!(got_elems >= elems, "{}", assert_msg);
                        if got_elems != elems {
                            // If `validate_cast_and_convert_metadata`
                            // returned more elements than `elems`, that
                            // means that `elems` is not the maximum number
                            // of elements that can fit in `size` - in other
                            // words, there is enough padding at the end of
                            // the value to fit at least one more element.
                            // If we use this metadata to synthesize a
                            // pointer, despite having a different element
                            // count, we still expect it to have the same
                            // size.
                            let got_ptr = with_elems(got_elems);
                            // SAFETY: `got_ptr` is a pointer to a valid `T`.
                            let size_of_got_ptr = unsafe { mem::size_of_val_raw(got_ptr.as_ptr()) };
                            assert_eq!(size_of_got_ptr, size, "{}", assert_msg);
                        }
                    } else {
                        // For sized casts, the returned element value is
                        // technically meaningless, and we don't guarantee any
                        // particular value. In practice, it's always zero.
                        assert_eq!(got_elems, 0, "{}", assert_msg)
                    }
                }
            }
        }

        macro_rules! validate_against_rust {
                ($offset:literal, $align:literal $(, $elem_size:literal)?) => {{
                    #[repr(C, align($align))]
                    struct Foo([u8; $offset]$(, [[u8; $elem_size]])?);

                    let args = MacroArgs {
                        offset: $offset,
                        align: $align.try_into().unwrap(),
                        elem_size: {
                            #[allow(unused)]
                            let ret = None::<usize>;
                            $(let ret = Some($elem_size);)?
                            ret
                        }
                    };

                    #[repr(C, align($align))]
                    struct FooAlign;
                    // Create an aligned buffer to use in order to synthesize
                    // pointers to `Foo`. We don't ever load values from these
                    // pointers - we just do arithmetic on them - so having a "real"
                    // block of memory as opposed to a validly-aligned-but-dangling
                    // pointer is only necessary to make Miri happy since we run it
                    // with "strict provenance" checking enabled.
                    let aligned_buf = Align::<_, FooAlign>::new([0u8; 1024]);
                    let with_elems = |elems| {
                        let slc = NonNull::slice_from_raw_parts(NonNull::from(&aligned_buf.t), elems);
                        #[allow(clippy::as_conversions)]
                        NonNull::new(slc.as_ptr() as *mut Foo).unwrap()
                    };
                    let addr_of_slice_field = {
                        #[allow(unused)]
                        let f = None::<fn(NonNull<Foo>) -> NonNull<u8>>;
                        $(
                            // SAFETY: `test` promises to only call `f` with a `ptr`
                            // to a valid `Foo`.
                            let f: Option<fn(NonNull<Foo>) -> NonNull<u8>> = Some(|ptr: NonNull<Foo>| unsafe {
                                NonNull::new(ptr::addr_of_mut!((*ptr.as_ptr()).1)).unwrap().cast::<u8>()
                            });
                            let _ = $elem_size;
                        )?
                        f
                    };

                    test::<Foo, _>(args, with_elems, addr_of_slice_field);
                }};
            }

        // Every permutation of:
        // - offset in [0, 4]
        // - align in [1, 16]
        // - elem_size in [0, 4] (plus no elem_size)
        validate_against_rust!(0, 1);
        validate_against_rust!(0, 1, 0);
        validate_against_rust!(0, 1, 1);
        validate_against_rust!(0, 1, 2);
        validate_against_rust!(0, 1, 3);
        validate_against_rust!(0, 1, 4);
        validate_against_rust!(0, 2);
        validate_against_rust!(0, 2, 0);
        validate_against_rust!(0, 2, 1);
        validate_against_rust!(0, 2, 2);
        validate_against_rust!(0, 2, 3);
        validate_against_rust!(0, 2, 4);
        validate_against_rust!(0, 4);
        validate_against_rust!(0, 4, 0);
        validate_against_rust!(0, 4, 1);
        validate_against_rust!(0, 4, 2);
        validate_against_rust!(0, 4, 3);
        validate_against_rust!(0, 4, 4);
        validate_against_rust!(0, 8);
        validate_against_rust!(0, 8, 0);
        validate_against_rust!(0, 8, 1);
        validate_against_rust!(0, 8, 2);
        validate_against_rust!(0, 8, 3);
        validate_against_rust!(0, 8, 4);
        validate_against_rust!(0, 16);
        validate_against_rust!(0, 16, 0);
        validate_against_rust!(0, 16, 1);
        validate_against_rust!(0, 16, 2);
        validate_against_rust!(0, 16, 3);
        validate_against_rust!(0, 16, 4);
        validate_against_rust!(1, 1);
        validate_against_rust!(1, 1, 0);
        validate_against_rust!(1, 1, 1);
        validate_against_rust!(1, 1, 2);
        validate_against_rust!(1, 1, 3);
        validate_against_rust!(1, 1, 4);
        validate_against_rust!(1, 2);
        validate_against_rust!(1, 2, 0);
        validate_against_rust!(1, 2, 1);
        validate_against_rust!(1, 2, 2);
        validate_against_rust!(1, 2, 3);
        validate_against_rust!(1, 2, 4);
        validate_against_rust!(1, 4);
        validate_against_rust!(1, 4, 0);
        validate_against_rust!(1, 4, 1);
        validate_against_rust!(1, 4, 2);
        validate_against_rust!(1, 4, 3);
        validate_against_rust!(1, 4, 4);
        validate_against_rust!(1, 8);
        validate_against_rust!(1, 8, 0);
        validate_against_rust!(1, 8, 1);
        validate_against_rust!(1, 8, 2);
        validate_against_rust!(1, 8, 3);
        validate_against_rust!(1, 8, 4);
        validate_against_rust!(1, 16);
        validate_against_rust!(1, 16, 0);
        validate_against_rust!(1, 16, 1);
        validate_against_rust!(1, 16, 2);
        validate_against_rust!(1, 16, 3);
        validate_against_rust!(1, 16, 4);
        validate_against_rust!(2, 1);
        validate_against_rust!(2, 1, 0);
        validate_against_rust!(2, 1, 1);
        validate_against_rust!(2, 1, 2);
        validate_against_rust!(2, 1, 3);
        validate_against_rust!(2, 1, 4);
        validate_against_rust!(2, 2);
        validate_against_rust!(2, 2, 0);
        validate_against_rust!(2, 2, 1);
        validate_against_rust!(2, 2, 2);
        validate_against_rust!(2, 2, 3);
        validate_against_rust!(2, 2, 4);
        validate_against_rust!(2, 4);
        validate_against_rust!(2, 4, 0);
        validate_against_rust!(2, 4, 1);
        validate_against_rust!(2, 4, 2);
        validate_against_rust!(2, 4, 3);
        validate_against_rust!(2, 4, 4);
        validate_against_rust!(2, 8);
        validate_against_rust!(2, 8, 0);
        validate_against_rust!(2, 8, 1);
        validate_against_rust!(2, 8, 2);
        validate_against_rust!(2, 8, 3);
        validate_against_rust!(2, 8, 4);
        validate_against_rust!(2, 16);
        validate_against_rust!(2, 16, 0);
        validate_against_rust!(2, 16, 1);
        validate_against_rust!(2, 16, 2);
        validate_against_rust!(2, 16, 3);
        validate_against_rust!(2, 16, 4);
        validate_against_rust!(3, 1);
        validate_against_rust!(3, 1, 0);
        validate_against_rust!(3, 1, 1);
        validate_against_rust!(3, 1, 2);
        validate_against_rust!(3, 1, 3);
        validate_against_rust!(3, 1, 4);
        validate_against_rust!(3, 2);
        validate_against_rust!(3, 2, 0);
        validate_against_rust!(3, 2, 1);
        validate_against_rust!(3, 2, 2);
        validate_against_rust!(3, 2, 3);
        validate_against_rust!(3, 2, 4);
        validate_against_rust!(3, 4);
        validate_against_rust!(3, 4, 0);
        validate_against_rust!(3, 4, 1);
        validate_against_rust!(3, 4, 2);
        validate_against_rust!(3, 4, 3);
        validate_against_rust!(3, 4, 4);
        validate_against_rust!(3, 8);
        validate_against_rust!(3, 8, 0);
        validate_against_rust!(3, 8, 1);
        validate_against_rust!(3, 8, 2);
        validate_against_rust!(3, 8, 3);
        validate_against_rust!(3, 8, 4);
        validate_against_rust!(3, 16);
        validate_against_rust!(3, 16, 0);
        validate_against_rust!(3, 16, 1);
        validate_against_rust!(3, 16, 2);
        validate_against_rust!(3, 16, 3);
        validate_against_rust!(3, 16, 4);
        validate_against_rust!(4, 1);
        validate_against_rust!(4, 1, 0);
        validate_against_rust!(4, 1, 1);
        validate_against_rust!(4, 1, 2);
        validate_against_rust!(4, 1, 3);
        validate_against_rust!(4, 1, 4);
        validate_against_rust!(4, 2);
        validate_against_rust!(4, 2, 0);
        validate_against_rust!(4, 2, 1);
        validate_against_rust!(4, 2, 2);
        validate_against_rust!(4, 2, 3);
        validate_against_rust!(4, 2, 4);
        validate_against_rust!(4, 4);
        validate_against_rust!(4, 4, 0);
        validate_against_rust!(4, 4, 1);
        validate_against_rust!(4, 4, 2);
        validate_against_rust!(4, 4, 3);
        validate_against_rust!(4, 4, 4);
        validate_against_rust!(4, 8);
        validate_against_rust!(4, 8, 0);
        validate_against_rust!(4, 8, 1);
        validate_against_rust!(4, 8, 2);
        validate_against_rust!(4, 8, 3);
        validate_against_rust!(4, 8, 4);
        validate_against_rust!(4, 16);
        validate_against_rust!(4, 16, 0);
        validate_against_rust!(4, 16, 1);
        validate_against_rust!(4, 16, 2);
        validate_against_rust!(4, 16, 3);
        validate_against_rust!(4, 16, 4);
    }
}
