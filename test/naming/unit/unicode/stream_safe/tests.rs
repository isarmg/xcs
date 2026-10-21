use super::{StreamSafe, classify_nonstarters};
use crate::unicode::lookups::canonical_combining_class;
use crate::unicode::normalize::decompose_compatible;

use core::char;

fn stream_safe(s: &str) -> String {
    StreamSafe::new(s.chars()).collect()
}

#[test]
fn test_simple() {
    let technically_okay = "Da\u{0300}\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}\u{0308}\u{0309}\u{030a}\u{030b}\u{030c}\u{030d}\u{030e}\u{030f}\u{0310}\u{0311}\u{0312}\u{0313}\u{0314}\u{0315}\u{0316}\u{0317}\u{0318}\u{0319}\u{031a}\u{031b}\u{031c}\u{031d}ngerzone";
    assert_eq!(stream_safe(technically_okay), technically_okay);

    let too_much = "Da\u{0300}\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}\u{0308}\u{0309}\u{030a}\u{030b}\u{030c}\u{030d}\u{030e}\u{030f}\u{0310}\u{0311}\u{0312}\u{0313}\u{0314}\u{0315}\u{0316}\u{0317}\u{0318}\u{0319}\u{031a}\u{031b}\u{031c}\u{031d}\u{032e}ngerzone";
    let fixed_it = "Da\u{0300}\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}\u{0308}\u{0309}\u{030a}\u{030b}\u{030c}\u{030d}\u{030e}\u{030f}\u{0310}\u{0311}\u{0312}\u{0313}\u{0314}\u{0315}\u{0316}\u{0317}\u{0318}\u{0319}\u{031a}\u{031b}\u{031c}\u{031d}\u{034f}\u{032e}ngerzone";
    assert_eq!(stream_safe(too_much), fixed_it);

    let woah_nelly = "Da\u{0300}\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}\u{0308}\u{0309}\u{030a}\u{030b}\u{030c}\u{030d}\u{030e}\u{030f}\u{0310}\u{0311}\u{0312}\u{0313}\u{0314}\u{0315}\u{0316}\u{0317}\u{0318}\u{0319}\u{031a}\u{031b}\u{031c}\u{031d}\u{032e}\u{0300}\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}\u{0308}\u{0309}\u{030a}\u{030b}\u{030c}\u{030d}\u{030e}\u{030f}\u{0310}\u{0311}\u{0312}\u{0313}\u{0314}\u{0315}\u{0316}\u{0317}\u{0318}\u{0319}\u{031a}\u{031b}\u{031c}\u{031d}\u{032e}ngerzone";
    let its_cool = "Da\u{0300}\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}\u{0308}\u{0309}\u{030a}\u{030b}\u{030c}\u{030d}\u{030e}\u{030f}\u{0310}\u{0311}\u{0312}\u{0313}\u{0314}\u{0315}\u{0316}\u{0317}\u{0318}\u{0319}\u{031a}\u{031b}\u{031c}\u{031d}\u{034f}\u{032e}\u{0300}\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}\u{0308}\u{0309}\u{030a}\u{030b}\u{030c}\u{030d}\u{030e}\u{030f}\u{0310}\u{0311}\u{0312}\u{0313}\u{0314}\u{0315}\u{0316}\u{0317}\u{0318}\u{0319}\u{031a}\u{031b}\u{031c}\u{034f}\u{031d}\u{032e}ngerzone";
    assert_eq!(stream_safe(woah_nelly), its_cool);
}

#[test]
fn test_all_nonstarters() {
    let s = "\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}\u{0300}";
    let expected = "\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{034F}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}\u{300}";
    assert_eq!(stream_safe(s), expected);
}

#[test]
fn test_classify_nonstarters() {
    // Highest character in the `compat_fully_decomp` table is 2FA1D
    for ch in 0..0x2FA1E {
        let ch = match char::from_u32(ch) {
            Some(c) => c,
            None => continue,
        };
        let c = classify_nonstarters(ch);
        let mut s = Vec::new();
        decompose_compatible(ch, |c| s.push(c));

        assert_eq!(s.len(), c.decomposition_len);

        let num_leading = s
            .iter()
            .take_while(|&c| canonical_combining_class(*c) != 0)
            .count();
        let num_trailing = s
            .iter()
            .rev()
            .take_while(|&c| canonical_combining_class(*c) != 0)
            .count();

        assert_eq!(num_leading, c.leading_nonstarters);
        assert_eq!(num_trailing, c.trailing_nonstarters);
    }
}
