use core::arch::aarch64 as arch;

#[derive(Clone)]
pub struct State {
    state: u32,
}

impl State {
    #[cfg(not(feature = "std"))]
    pub fn new(state: u32) -> Option<Self> {
        if cfg!(target_feature = "crc") {
            // SAFETY: The conditions above ensure that all
            //         required instructions are supported by the CPU.
            Some(Self { state })
        } else {
            None
        }
    }

    #[cfg(feature = "std")]
    pub fn new(state: u32) -> Option<Self> {
        if std::arch::is_aarch64_feature_detected!("crc") {
            // SAFETY: The condition above ensures that all
            //         required instructions are supported by the CPU.
            Some(Self { state })
        } else {
            None
        }
    }

    pub fn update(&mut self, buf: &[u8]) {
        // SAFETY: The `State::new` constructor ensures that all
        //         required instructions are supported by the CPU.
        self.state = unsafe { calculate(self.state, buf) }
    }

    pub fn finalize(self) -> u32 {
        self.state
    }

    pub fn reset(&mut self) {
        self.state = 0;
    }

    pub fn combine(&mut self, other: u32, amount: u64) {
        self.state = crate::combine::combine(self.state, other, amount);
    }
}

// Conservative crossover: 64 words per stream gives the interleaved loop enough work to
// amortize its two `combine` calls (3 * 64 * 8 bytes = 1.5 KiB).
const MIN_TRIPLE_QUADS: usize = 192;

// target_feature is necessary to allow rustc to inline the crc32* wrappers
#[target_feature(enable = "crc")]
pub unsafe fn calculate(crc: u32, data: &[u8]) -> u32 {
    let (pre_quad, quads, post_quad) = data.align_to::<u64>();

    // Running (finalized) CRC.
    let mut r = crc;

    // Unaligned leading bytes.
    r = crc_bytes(r, pre_quad);

    if quads.len() >= MIN_TRIPLE_QUADS {
        // A single `crc32d` chain is latency-bound: each instruction depends on the previous
        // result. Splitting the aligned region into three equal, contiguous blocks and advancing
        // three independent accumulators in lockstep keeps several `crc32d` in flight, then
        // `combine` reassembles them (appending the zero-seeded later blocks to the first).
        let n3 = quads.len() / 3;
        let block_len = (n3 * 8) as u64;

        let a = &quads[0..n3];
        let b = &quads[n3..2 * n3];
        let c = &quads[2 * n3..3 * n3];

        // Internal (bit-inverted) states. A carries the running CRC; B and C start from zero.
        let mut c0 = !r;
        let mut c1 = !0u32;
        let mut c2 = !0u32;

        // Unroll each stream by 4 while keeping the three chains independent.
        let mut chunks_a = a.chunks_exact(4);
        let mut chunks_b = b.chunks_exact(4);
        let mut chunks_c = c.chunks_exact(4);
        for ((qa, qb), qc) in (&mut chunks_a).zip(&mut chunks_b).zip(&mut chunks_c) {
            for k in 0..4 {
                c0 = arch::__crc32d(c0, qa[k]);
                c1 = arch::__crc32d(c1, qb[k]);
                c2 = arch::__crc32d(c2, qc[k]);
            }
        }
        for ((&qa, &qb), &qc) in chunks_a
            .remainder()
            .iter()
            .zip(chunks_b.remainder())
            .zip(chunks_c.remainder())
        {
            c0 = arch::__crc32d(c0, qa);
            c1 = arch::__crc32d(c1, qb);
            c2 = arch::__crc32d(c2, qc);
        }

        r = crate::combine::combine(!c0, !c1, block_len);
        r = crate::combine::combine(r, !c2, block_len);

        // Words left over after the three equal blocks.
        r = crc_quads(r, &quads[3 * n3..]);
    } else {
        r = crc_quads(r, quads);
    }

    // Unaligned trailing bytes.
    crc_bytes(r, post_quad)
}

#[target_feature(enable = "crc")]
unsafe fn crc_quads(crc: u32, quads: &[u64]) -> u32 {
    let mut c = !crc;

    // unrolling increases performance by a lot
    let mut quad_iter = quads.chunks_exact(8);
    for chunk in &mut quad_iter {
        c = arch::__crc32d(c, chunk[0]);
        c = arch::__crc32d(c, chunk[1]);
        c = arch::__crc32d(c, chunk[2]);
        c = arch::__crc32d(c, chunk[3]);
        c = arch::__crc32d(c, chunk[4]);
        c = arch::__crc32d(c, chunk[5]);
        c = arch::__crc32d(c, chunk[6]);
        c = arch::__crc32d(c, chunk[7]);
    }
    c = quad_iter
        .remainder()
        .iter()
        .fold(c, |acc, &q| arch::__crc32d(acc, q));

    !c
}

#[target_feature(enable = "crc")]
unsafe fn crc_bytes(crc: u32, bytes: &[u8]) -> u32 {
    let c = bytes.iter().fold(!crc, |acc, &b| arch::__crc32b(acc, b));
    !c
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/crc32fast/unit/src/specialized/aarch64/test_147.rs"));
