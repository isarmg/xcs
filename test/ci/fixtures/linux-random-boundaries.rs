//! Isolated entropy consumer boundary and error checks.
use core::mem::MaybeUninit;
#[test]
fn boundaries() {
    assert!(getrandom::FILL(&mut []).is_ok());
    for n in [1, 7, 256, 4096, 65537] {
        let mut b = vec![0xA5; n + 2];
        let result = getrandom::FILL(&mut b[1..n + 1]);
        #[cfg(not(getrandom_backend = "unsupported"))]
        result.unwrap();
        #[cfg(getrandom_backend = "unsupported")]
        assert!(result.is_err());
        assert_eq!(b[0], 0xA5);
        assert_eq!(b[n + 1], 0xA5);
        let mut u = vec![MaybeUninit::uninit(); n];
        let result = getrandom::UNINIT(&mut u);
        #[cfg(not(getrandom_backend = "unsupported"))]
        assert_eq!(result.unwrap().len(), n);
        #[cfg(getrandom_backend = "unsupported")]
        assert!(result.is_err());
    }
}
