fn main() {
    for len in [
        0, 1, 63, 64, 65, 1023, 1024, 1025, 2048, 4095, 4096, 4097, 16384, 65535, 65536, 65537,
        1048576, 2097169,
    ] {
        for chunk in [63, 1024, 4096, 65536] {
            println!(
                "{len} {chunk} {}",
                web_blake3_simd_check::digest(len, chunk).to_hex()
            );
        }
    }
}
