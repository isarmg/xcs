#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_ref_methods() {
        let bytes = &[0u8; 1][..];
        let bytes_slice = &[0u8; 4][..];

        let r: Option<Ref<&[u8], u8>> = Ref::new(bytes);
        assert!(r.is_some());

        let r: Option<(Ref<&[u8], u8>, &[u8])> = Ref::new_from_prefix(bytes);
        assert!(r.is_some());

        let r: Option<(&[u8], Ref<&[u8], u8>)> = Ref::new_from_suffix(bytes);
        assert!(r.is_some());

        let r: Option<Ref<&[u8], u8>> = Ref::new_unaligned(bytes);
        assert!(r.is_some());

        let r: Option<(Ref<&[u8], u8>, &[u8])> = Ref::new_unaligned_from_prefix(bytes);
        assert!(r.is_some());

        let r: Option<(&[u8], Ref<&[u8], u8>)> = Ref::new_unaligned_from_suffix(bytes);
        assert!(r.is_some());

        let r: Option<Ref<&[u8], [u8]>> = Ref::new_slice(bytes_slice);
        assert!(r.is_some());

        let r: Option<Ref<&[u8], [u8]>> = Ref::new_slice_unaligned(bytes_slice);
        assert!(r.is_some());

        let r: Option<(Ref<&[u8], [u8]>, &[u8])> = Ref::new_slice_from_prefix(bytes_slice, 1);
        assert!(r.is_some());

        let r: Option<(&[u8], Ref<&[u8], [u8]>)> = Ref::new_slice_from_suffix(bytes_slice, 1);
        assert!(r.is_some());

        let r: Option<(Ref<&[u8], [u8]>, &[u8])> =
            Ref::new_slice_unaligned_from_prefix(bytes_slice, 1);
        assert!(r.is_some());

        let r: Option<(&[u8], Ref<&[u8], [u8]>)> =
            Ref::new_slice_unaligned_from_suffix(bytes_slice, 1);
        assert!(r.is_some());
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_into_slice() {
        let bytes = &[0u8; 4][..];
        let r: Ref<&[u8], [u8]> = Ref::from_bytes(bytes).unwrap();
        let slice: &[u8] = r.into_slice();
        assert_eq!(slice.len(), 4);
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_into_mut_slice() {
        let mut bytes = [0u8; 4];
        let r: Ref<&mut [u8], [u8]> = Ref::from_bytes(&mut bytes[..]).unwrap();
        let slice: &mut [u8] = r.into_mut_slice();
        assert_eq!(slice.len(), 4);
    }
}
