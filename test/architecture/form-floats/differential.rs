#[derive(Default)]
struct Stats {
    total: u64,
    diff: u64,
    other: u64,
}
fn normalized(s: &str) -> String {
    s.replace("e+", "e")
}
fn check32(bits: u32, s: &mut Stats) {
    let f = f32::from_bits(bits);
    let mut r = ryu::Buffer::new();
    let mut z = zmij::Buffer::new();
    let a = r.format(f);
    let b = z.format(f);
    s.total += 1;
    let oldwire = form_codec::to_string(&[("x", a)]).unwrap();
    let newwire = form_codec::to_string(&[("x", b)]).unwrap();
    assert_eq!(form_codec::to_string(&[("x", f)]).unwrap(), newwire);
    assert_eq!(
        a == b,
        oldwire == newwire,
        "formatter/wire differential diverged"
    );
    assert_eq!(normalized(a), normalized(b), "unexpected digit difference");
    if f.is_finite() {
        let parsed: Vec<(String, f32)> = form_codec::from_str(&newwire).unwrap();
        assert_eq!(
            f.to_bits(),
            parsed[0].1.to_bits(),
            "numeric round-trip failure"
        );
    }
    if a != b {
        s.diff += 1;
        let other = normalized(a) != normalized(b);
        if other {
            s.other += 1;
        }
        if s.diff <= 5 || (other && s.other <= 10) {
            println!(
                "f32 {bits:08x}: ryu={a} zmij={b} oldwire={} newwire={}",
                form_codec::to_string(&[("x", a)]).unwrap(),
                form_codec::to_string(&[("x", b)]).unwrap()
            );
        }
    }
}
fn check64(bits: u64, s: &mut Stats) {
    let f = f64::from_bits(bits);
    let mut r = ryu::Buffer::new();
    let mut z = zmij::Buffer::new();
    let a = r.format(f);
    let b = z.format(f);
    s.total += 1;
    let oldwire = form_codec::to_string(&[("x", a)]).unwrap();
    let newwire = form_codec::to_string(&[("x", b)]).unwrap();
    assert_eq!(form_codec::to_string(&[("x", f)]).unwrap(), newwire);
    assert_eq!(
        a == b,
        oldwire == newwire,
        "formatter/wire differential diverged"
    );
    assert_eq!(normalized(a), normalized(b), "unexpected digit difference");
    if f.is_finite() {
        let parsed: Vec<(String, f64)> = form_codec::from_str(&newwire).unwrap();
        assert_eq!(
            f.to_bits(),
            parsed[0].1.to_bits(),
            "numeric round-trip failure"
        );
    }
    if a != b {
        s.diff += 1;
        let other = normalized(a) != normalized(b);
        if other {
            s.other += 1;
        }
        if s.diff <= 5 || (other && s.other <= 10) {
            println!(
                "f64 {bits:016x}: ryu={a} zmij={b} oldwire={} newwire={}",
                form_codec::to_string(&[("x", a)]).unwrap(),
                form_codec::to_string(&[("x", b)]).unwrap()
            );
        }
    }
}
fn main() {
    let mut a = Stats::default();
    let mut b = Stats::default();
    for x in [
        0.,
        -0.,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        1e-7,
        1e-6,
        1e-5,
        1e14,
        1e15,
        1e16,
        1e17,
    ] {
        check32(x.to_bits(), &mut a);
    }
    for x in [
        0.,
        -0.,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        1e-7,
        1e-6,
        1e-5,
        1e14,
        1e15,
        1e16,
        1e17,
    ] {
        check64(x.to_bits(), &mut b);
    }
    for sign in [0, 1u32 << 31] {
        for exp in 0..=255u32 {
            for m in [
                0, 1, 2, 3, 0x3ffffe, 0x3fffff, 0x400000, 0x400001, 0x7ffffd, 0x7ffffe, 0x7fffff,
            ] {
                check32(sign | (exp << 23) | m, &mut a);
            }
        }
    }
    for sign in [0, 1u64 << 63] {
        for exp in 0..=2047u64 {
            for m in [
                0,
                1,
                2,
                3,
                0x7fffffffffffe,
                0x7ffffffffffff,
                0x8000000000000,
                0x8000000000001,
                0xffffffffffffd,
                0xffffffffffffe,
                0xfffffffffffff,
            ] {
                check64(sign | (exp << 52) | m, &mut b);
            }
        }
    }
    for exp in -45..=38 {
        let x = 10f32.powi(exp).to_bits();
        for d in -16i64..=16 {
            let p = (x as i64 + d).clamp(0, u32::MAX as i64) as u32;
            check32(p, &mut a);
            check32(p | (1 << 31), &mut a);
        }
    }
    for exp in -323..=308 {
        let x = 10f64.powi(exp).to_bits();
        for d in -16i128..=16 {
            let p = (x as i128 + d).clamp(0, u64::MAX as i128) as u64;
            check64(p, &mut b);
            check64(p | (1 << 63), &mut b);
        }
    }
    println!(
        "boundaries: f32 total={} diff={} not_just_plus={}; f64 total={} diff={} not_just_plus={}",
        a.total, a.diff, a.other, b.total, b.diff, b.other
    );
    let mut rng = 0x19a24309a92f1c79u64;
    for _ in 0..5_000_000 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        check64(rng, &mut b);
        check32((rng >> 32) as u32, &mut a);
    }
    println!(
        "with_random: f32 total={} diff={} not_just_plus={}; f64 total={} diff={} not_just_plus={}",
        a.total, a.diff, a.other, b.total, b.diff, b.other
    );
    let mut n32 = Stats::default();
    let mut n64 = Stats::default();
    for payload in 0..=0x7fffff {
        for sign in [0, 1 << 31] {
            check32(sign | 0x7f800000 | payload, &mut n32);
        }
    }
    for payload in [0, 1, 2, 0x7ffffffffffff, 0x8000000000000, 0xfffffffffffff] {
        for sign in [0, 1 << 63] {
            check64(sign | 0x7ff0000000000000 | payload, &mut n64);
        }
    }
    println!(
        "nonfinite: f32 total={} diff={}; f64 representative total={} diff={}",
        n32.total, n32.diff, n64.total, n64.diff
    );
}
