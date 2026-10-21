#![cfg(all(feature = "block-api", feature = "zeroize"))]
use cryptographic_core::{
    buffer::{EagerBuffer, LazyBuffer, ReadBuffer},
    common_types::array::{Array, typenum::U16},
    zeroize::{Zeroize, ZeroizeOnDrop},
};
#[test]
fn buffer_zeroization_capabilities() {
    let mut eager = EagerBuffer::<U16>::new(&[0x42; 7]);
    eager.zeroize();
    assert_eq!(eager.get_pos(), 0);
    assert!(eager.pad_with_zeros().iter().all(|b| *b == 0));
    let mut lazy = LazyBuffer::<U16>::new(&[0x42; 16]);
    lazy.zeroize();
    assert_eq!(lazy.get_pos(), 0);
    assert!(lazy.pad_with_zeros().iter().all(|b| *b == 0));
    fn drop_wipes<T: ZeroizeOnDrop>() {}
    drop_wipes::<EagerBuffer<U16>>();
    drop_wipes::<LazyBuffer<U16>>();
    drop_wipes::<ReadBuffer<U16>>();
    let mut array = Array::<u8, U16>::from([0x42; 16]);
    array.zeroize();
    assert_eq!(array.as_slice(), &[0; 16]);
}
