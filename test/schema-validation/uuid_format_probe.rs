// Compiled by check-uuid-format.py against the production predicate and a pinned
// pre-retirement uuid-simd implementation recovered from committed git history.
#[path = "production.rs"]
mod production;
use std::{hint::black_box, time::Instant};
#[inline(never)]
fn old(bytes: &[u8]) -> bool {
    uuid_simd::parse_hyphenated(bytes, uuid_simd::Out::from_mut(&mut [0; 16])).is_ok()
}
#[inline(never)]
fn new(bytes: &[u8]) -> bool {
    production::is_valid(bytes)
}
fn reference(bytes: &[u8]) -> bool {
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                *b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn main() {
    let mut count = 0;
    let mut check = |bytes: &[u8]| {
        assert_eq!(old(bytes), reference(bytes), "old contract: {bytes:?}");
        assert_eq!(new(bytes), old(bytes), "differential: {bytes:?}");
        count += 1;
    };
    let seeds = [
        *b"12345678-9aBc-DeF0-1234-56789abcdef0",
        *b"00000000-0000-0000-0000-000000000000",
        *b"FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF",
    ];
    for seed in seeds {
        check(&seed);
        for pos in 0..36 {
            for byte in 0..=255 {
                let mut bytes = seed;
                bytes[pos] = byte;
                check(&bytes);
            }
        }
        for len in 0..=128 {
            let mut bytes = vec![b'a'; len];
            let n = len.min(36);
            bytes[..n].copy_from_slice(&seed[..n]);
            check(&bytes);
        }
    }
    for text in [
        "１２３４５６７８-9abc-def0-1234-56789abcdef0",
        "12345678–9abc-def0-1234-56789abcdef0",
        "12345678-9abc-def0-1234-56789abcdeé",
        "urn:uuid:12345678-9abc-def0-1234-56789abcdef0",
        "{12345678-9abc-def0-1234-56789abcdef0}",
    ] {
        check(text.as_bytes());
    }
    let mut random = 0x129b728af57caa31u64;
    let mut next = || {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        random
    };
    for _ in 0..200_000 {
        let len = next() as usize % 81;
        let mut bytes = vec![0; len];
        for b in &mut bytes {
            *b = next() as u8;
        }
        check(&bytes);
        let mut bytes = seeds[0];
        for _ in 0..(next() % 5) {
            bytes[next() as usize % 36] = next() as u8;
        }
        check(&bytes);
        let mut valid = seeds[0];
        for (i, b) in valid.iter_mut().enumerate() {
            if ![8, 13, 18, 23].contains(&i) {
                *b = b"0123456789abcdefABCDEF"[next() as usize % 22];
            }
        }
        check(&valid);
    }
    println!("UUID exact syntax differential: {count} cases PASS");
    if std::env::args().any(|arg| arg == "--bench") {
        let mut cases = Vec::new();
        for seed in seeds {
            cases.push(seed.to_vec());
        }
        let mut early = seeds[0];
        early[0] = b'!';
        cases.push(early.to_vec());
        let mut late = seeds[0];
        late[35] = b'!';
        cases.push(late.to_vec());
        let mut separator = seeds[0];
        separator[18] = b'_';
        cases.push(separator.to_vec());
        cases.push(vec![b'a'; 35]);
        let mut datasets: Vec<Vec<Vec<u8>>> =
            cases.iter().map(|bytes| vec![bytes.clone()]).collect();
        datasets.push(cases);
        for (case, dataset) in datasets.iter().enumerate() {
            let mut times = [Vec::new(), Vec::new()];
            for round in 0..11 {
                for index in [round % 2, 1 - round % 2] {
                    let f = if index == 0 { old } else { new };
                    let start = Instant::now();
                    let mut sum = 0;
                    for i in 0..2_000_000 {
                        sum += f(black_box(&dataset[i % dataset.len()])) as usize;
                    }
                    black_box(sum);
                    times[index].push(start.elapsed().as_nanos() as f64 / 2_000_000.0);
                }
            }
            for t in &mut times {
                t.sort_by(f64::total_cmp);
            }
            println!("case {case}: old median {:.3} ns, new median {:.3} ns, ratio {:.3}; old/new minima {:.3}/{:.3}, p90 {:.3}/{:.3}", times[0][5],times[1][5],times[1][5]/times[0][5],times[0][0],times[1][0],times[0][9],times[1][9]);
        }
    }
}
