//! Isolated entropy consumer boundary and error checks.
use core::mem::MaybeUninit;
#[test]
fn boundaries() {
    assert!(system_entropy::fill(&mut []).is_ok());
    for n in [1, 7, 256, 4096, 65537] {
        let mut b = vec![0xA5; n + 2];
        let result = system_entropy::fill(&mut b[1..n + 1]);
        #[cfg(not(system_entropy_backend = "unsupported"))]
        result.unwrap();
        #[cfg(system_entropy_backend = "unsupported")]
        assert!(result.is_err());
        assert_eq!(b[0], 0xA5);
        assert_eq!(b[n + 1], 0xA5);
        let mut u = vec![MaybeUninit::uninit(); n];
        let result = system_entropy::fill_uninit(&mut u);
        #[cfg(not(system_entropy_backend = "unsupported"))]
        assert_eq!(result.unwrap().len(), n);
        #[cfg(system_entropy_backend = "unsupported")]
        assert!(result.is_err());
    }
}
