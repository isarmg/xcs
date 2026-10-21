#[cfg(test)]
mod tests {
    use super::*;

    extern crate std;
    use std::print;
    use std::println;

    fn print_byte_to_stdout(byte: u8) {
        let c = char::from(byte);
        if c.is_ascii_alphanumeric() {
            print!("'{c}'");
        } else {
            print!("0x{byte:X}");
        }
    }

    fn random_alphanums(seed: u64, len: usize) -> Vec<u8> {
        use rand::seq::SliceRandom;
        use rand::SeedableRng;
        let mut bytes: Vec<u8> =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".into();
        let mut rng = rand_pcg::Lcg64Xsh32::seed_from_u64(seed);
        bytes.partial_shuffle(&mut rng, len).0.into()
    }

    #[test]
    fn test_random_distributions() {
        let mut p_distr = vec![0; 256];
        let mut q_distr = vec![0; 256];
        for len in 0..50 {
            for seed in 0..50 {
                let bytes = random_alphanums(seed, len);
                let (p, qq) = find(bytes.as_slice()).unwrap();
                p_distr[p as usize] += 1;
                for q in qq {
                    q_distr[q as usize] += 1;
                }
            }
        }
        println!("p_distr: {p_distr:?}");
        println!("q_distr: {q_distr:?}");

        let fast_p = p_distr[0..=P_FAST_MAX as usize].iter().sum::<usize>();
        let slow_p = p_distr[(P_FAST_MAX + 1) as usize..].iter().sum::<usize>();
        let fast_q = q_distr[0..=Q_FAST_MAX as usize].iter().sum::<usize>();
        let slow_q = q_distr[(Q_FAST_MAX + 1) as usize..].iter().sum::<usize>();

        assert_eq!(2500, fast_p);
        assert_eq!(0, slow_p);
        assert_eq!(61243, fast_q);
        assert_eq!(7, slow_q);

        let bytes = random_alphanums(0, 16);

        #[allow(non_snake_case)]
        let N = u8::try_from(bytes.len()).unwrap();

        let (p, qq) = find(bytes.as_slice()).unwrap();

        println!("Results:");
        for byte in bytes.iter() {
            print_byte_to_stdout(*byte);
            let l1 = f1(*byte, p, N) as usize;
            let q = qq[l1];
            let l2 = f2(*byte, q, N) as usize;
            println!(" => l1 {l1} => q {q} => l2 {l2}");
        }
    }
}
