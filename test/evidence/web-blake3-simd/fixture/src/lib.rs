// Bounded streaming fixture. Every input byte is derived from its global offset,
// independent of update boundaries; never allocates a whole-file buffer.
pub fn digest(len: usize, update_size: usize) -> blake3::Hash {
    assert!((1..=65536).contains(&update_size));
    let mut buffer = [0u8; 65536];
    let mut hasher = blake3::Hasher::new();
    let mut offset = 0;
    while offset < len {
        let count = update_size.min(len - offset);
        for (i, byte) in buffer[..count].iter_mut().enumerate() {
            let pos = offset + i;
            *byte = ((pos.wrapping_mul(131) ^ (pos >> 8) ^ (pos >> 17)) & 255) as u8;
        }
        hasher.update(&buffer[..count]);
        offset += count;
    }
    hasher.finalize()
}

#[unsafe(no_mangle)]
pub extern "C" fn digest_word(len: u32, update_size: u32, word: u32) -> u32 {
    let result = digest(len as usize, update_size as usize);
    let start = word as usize * 4;
    u32::from_le_bytes(result.as_bytes()[start..start + 4].try_into().unwrap())
}
