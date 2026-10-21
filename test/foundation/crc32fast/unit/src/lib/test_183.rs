#[cfg(test)]
mod test {
    use super::Hasher;

    quickcheck::quickcheck! {
        fn combine(bytes_1: Vec<u8>, bytes_2: Vec<u8>) -> bool {
            let mut hash_a = Hasher::new();
            hash_a.update(&bytes_1);
            hash_a.update(&bytes_2);
            let mut hash_b = Hasher::new();
            hash_b.update(&bytes_2);
            let mut hash_c = Hasher::new();
            hash_c.update(&bytes_1);
            hash_c.combine(&hash_b);

            hash_a.finalize() == hash_c.finalize()
        }

        fn combine_from_len(bytes_1: Vec<u8>, bytes_2: Vec<u8>) -> bool {
            let mut hash_a = Hasher::new();
            hash_a.update(&bytes_1);

            let mut hash_b = Hasher::new();
            hash_b.update(&bytes_2);

            let mut hash_ab = Hasher::new();
            hash_ab.update(&bytes_1);
            hash_ab.update(&bytes_2);
            let ab = hash_ab.finalize();

            hash_a.combine(&hash_b);
            hash_a.finalize() == ab
        }
    }
}
