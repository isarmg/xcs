#[cfg(test)]
mod tests {
    use super::*;
    use crate::ule::AsULE;
    use rand::{distr::StandardUniform, Rng, SeedableRng};
    use rand_pcg::Lcg64Xsh32;

    #[test]
    fn test_zhms_u64k_u64v() {
        const N: usize = 65530;
        let seed = u64::from_le_bytes(*b"testseed");
        let rng = Lcg64Xsh32::seed_from_u64(seed);
        let kv: Vec<(u64, u64)> = rng.sample_iter(&StandardUniform).take(N).collect();
        let hashmap: ZeroHashMap<u64, u64> =
            ZeroHashMap::from_iter(kv.iter().map(|e| (&e.0, &e.1)));
        for (k, v) in kv {
            assert_eq!(
                hashmap.get(&k).copied().map(<u64 as AsULE>::from_unaligned),
                Some(v),
            );
        }
    }

    #[test]
    #[should_panic(expected = "Hashmap too large for u32")]
    #[cfg(target_pointer_width = "64")]
    fn test_hashmap_too_large() {
        struct HugeIter;
        impl Iterator for HugeIter {
            type Item = u64;
            fn next(&mut self) -> Option<Self::Item> {
                None
            }
        }
        impl ExactSizeIterator for HugeIter {
            fn len(&self) -> usize {
                // Return something that is > u32::MAX on 64-bit systems
                (u32::MAX as usize) + 1
            }
        }
        let _ = compute_displacements(HugeIter);
    }
}
