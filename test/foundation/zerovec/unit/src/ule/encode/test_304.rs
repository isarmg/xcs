#[cfg(test)]
mod test {
    use super::*;

    const STRING_ARRAY: [&str; 2] = ["hello", "world"];

    const STRING_SLICE: &[&str] = &STRING_ARRAY;

    const U8_ARRAY: [u8; 8] = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];

    const U8_2D_ARRAY: [&[u8]; 2] = [&U8_ARRAY, &U8_ARRAY];

    const U8_2D_SLICE: &[&[u8]] = &[&U8_ARRAY, &U8_ARRAY];

    const U8_3D_ARRAY: [&[&[u8]]; 2] = [U8_2D_SLICE, U8_2D_SLICE];

    const U8_3D_SLICE: &[&[&[u8]]] = &[U8_2D_SLICE, U8_2D_SLICE];

    const U32_ARRAY: [u32; 4] = [0x00010203, 0x04050607, 0x08090A0B, 0x0C0D0E0F];

    const U32_2D_ARRAY: [&[u32]; 2] = [&U32_ARRAY, &U32_ARRAY];

    const U32_2D_SLICE: &[&[u32]] = &[&U32_ARRAY, &U32_ARRAY];

    const U32_3D_ARRAY: [&[&[u32]]; 2] = [U32_2D_SLICE, U32_2D_SLICE];

    const U32_3D_SLICE: &[&[&[u32]]] = &[U32_2D_SLICE, U32_2D_SLICE];

    #[test]
    fn test_vzv_from() {
        type VZV<'a, T> = VarZeroVec<'a, T>;
        type ZS<T> = ZeroSlice<T>;
        type VZS<T> = VarZeroSlice<T>;

        let u8_zerovec: ZeroVec<u8> = ZeroVec::from_slice_or_alloc(&U8_ARRAY);
        let u8_2d_zerovec: [ZeroVec<u8>; 2] = [u8_zerovec.clone(), u8_zerovec.clone()];
        let u8_2d_vec: Vec<Vec<u8>> = vec![U8_ARRAY.into(), U8_ARRAY.into()];
        let u8_3d_vec: Vec<Vec<Vec<u8>>> = vec![u8_2d_vec.clone(), u8_2d_vec.clone()];

        let u32_zerovec: ZeroVec<u32> = ZeroVec::from_slice_or_alloc(&U32_ARRAY);
        let u32_2d_zerovec: [ZeroVec<u32>; 2] = [u32_zerovec.clone(), u32_zerovec.clone()];
        let u32_2d_vec: Vec<Vec<u32>> = vec![U32_ARRAY.into(), U32_ARRAY.into()];
        let u32_3d_vec: Vec<Vec<Vec<u32>>> = vec![u32_2d_vec.clone(), u32_2d_vec.clone()];

        let a: VZV<str> = VarZeroVec::from(&STRING_ARRAY);
        let b: VZV<str> = VarZeroVec::from(STRING_SLICE);
        let c: VZV<str> = VarZeroVec::from(&Vec::from(STRING_SLICE));
        assert_eq!(a, STRING_SLICE);
        assert_eq!(a, b);
        assert_eq!(a, c);

        let a: VZV<[u8]> = VarZeroVec::from(&U8_2D_ARRAY);
        let b: VZV<[u8]> = VarZeroVec::from(U8_2D_SLICE);
        let c: VZV<[u8]> = VarZeroVec::from(&u8_2d_vec);
        assert_eq!(a, U8_2D_SLICE);
        assert_eq!(a, b);
        assert_eq!(a, c);
        let u8_3d_vzv_brackets = &[a.clone(), a.clone()];

        let a: VZV<ZS<u8>> = VarZeroVec::from(&U8_2D_ARRAY);
        let b: VZV<ZS<u8>> = VarZeroVec::from(U8_2D_SLICE);
        let c: VZV<ZS<u8>> = VarZeroVec::from(&u8_2d_vec);
        let d: VZV<ZS<u8>> = VarZeroVec::from(&u8_2d_zerovec);
        assert_eq!(a, U8_2D_SLICE);
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a, d);
        let u8_3d_vzv_zeroslice = &[a.clone(), a.clone()];

        let a: VZV<VZS<[u8]>> = VarZeroVec::from(&U8_3D_ARRAY);
        let b: VZV<VZS<[u8]>> = VarZeroVec::from(U8_3D_SLICE);
        let c: VZV<VZS<[u8]>> = VarZeroVec::from(&u8_3d_vec);
        let d: VZV<VZS<[u8]>> = VarZeroVec::from(u8_3d_vzv_brackets);
        assert_eq!(
            a.iter()
                .map(|x| x.iter().map(|y| y.to_vec()).collect::<Vec<Vec<u8>>>())
                .collect::<Vec<Vec<Vec<u8>>>>(),
            u8_3d_vec
        );
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a, d);

        let a: VZV<VZS<ZS<u8>>> = VarZeroVec::from(&U8_3D_ARRAY);
        let b: VZV<VZS<ZS<u8>>> = VarZeroVec::from(U8_3D_SLICE);
        let c: VZV<VZS<ZS<u8>>> = VarZeroVec::from(&u8_3d_vec);
        let d: VZV<VZS<ZS<u8>>> = VarZeroVec::from(u8_3d_vzv_zeroslice);
        assert_eq!(
            a.iter()
                .map(|x| x
                    .iter()
                    .map(|y| y.iter().collect::<Vec<u8>>())
                    .collect::<Vec<Vec<u8>>>())
                .collect::<Vec<Vec<Vec<u8>>>>(),
            u8_3d_vec
        );
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a, d);

        let a: VZV<ZS<u32>> = VarZeroVec::from(&U32_2D_ARRAY);
        let b: VZV<ZS<u32>> = VarZeroVec::from(U32_2D_SLICE);
        let c: VZV<ZS<u32>> = VarZeroVec::from(&u32_2d_vec);
        let d: VZV<ZS<u32>> = VarZeroVec::from(&u32_2d_zerovec);
        assert_eq!(a, u32_2d_zerovec);
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a, d);
        let u32_3d_vzv = &[a.clone(), a.clone()];

        let a: VZV<VZS<ZS<u32>>> = VarZeroVec::from(&U32_3D_ARRAY);
        let b: VZV<VZS<ZS<u32>>> = VarZeroVec::from(U32_3D_SLICE);
        let c: VZV<VZS<ZS<u32>>> = VarZeroVec::from(&u32_3d_vec);
        let d: VZV<VZS<ZS<u32>>> = VarZeroVec::from(u32_3d_vzv);
        assert_eq!(
            a.iter()
                .map(|x| x
                    .iter()
                    .map(|y| y.iter().collect::<Vec<u32>>())
                    .collect::<Vec<Vec<u32>>>())
                .collect::<Vec<Vec<Vec<u32>>>>(),
            u32_3d_vec
        );
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a, d);
    }
}
