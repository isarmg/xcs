#![no_std]
use random_generator_core::{Error, OsRng, RngCore};

pub fn try_entropy(bytes: &mut [u8]) -> Result<(), Error> {
    OsRng.try_fill_bytes(bytes)
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;

    struct SeedProbe;
    impl random_generator_core::SeedableRng for SeedProbe {
        type Seed = [u8; 32];
        fn from_seed(_: Self::Seed) -> Self { panic!("seed constructor must not run after entropy failure") }
    }

    #[test]
    fn fail_closed() {
        let mut bytes = [0xa5; 32];
        let error = try_entropy(&mut bytes).expect_err("unsupported entropy must not succeed");
        assert_eq!(error.code(), None);
        assert_eq!(error.raw_os_error(), None);
        assert_eq!(std::format!("{}", error), std::format!("{}", system_entropy::Error::UNSUPPORTED));
        assert_eq!(bytes, [0xa5; 32]);
        assert!(std::panic::catch_unwind(|| OsRng.next_u64()).is_err());
        assert!(std::panic::catch_unwind(|| OsRng.fill_bytes(&mut [0; 32])).is_err());
        assert!(OsRng.try_fill_bytes(&mut []).is_ok());
        use random_generator_core::SeedableRng;
        let panic = std::panic::catch_unwind(SeedProbe::from_entropy).err().expect("entropy seed must fail");
        let message = panic.downcast_ref::<std::string::String>().expect("formatted entropy failure");
        assert!(message.starts_with("from_entropy failed:"));
        #[cfg(target_os = "linux")]
        {
            use ring::rand::{SecureRandom, SystemRandom};
            assert!(SystemRandom::new().fill(&mut bytes).is_err());
        }
    }
}
