use super::{IsNormalized, is_nfc_stream_safe_quick, is_nfd_stream_safe_quick};

#[test]
fn test_stream_safe_nfd() {
    let okay = "Da\u{031b}\u{0316}\u{0317}\u{0318}\u{0319}\u{031c}\u{031d}\u{0300}\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}\u{0308}\u{0309}\u{030a}\u{030b}\u{030c}\u{030d}\u{030e}\u{030f}\u{0310}\u{0311}\u{0312}\u{0313}\u{0314}\u{0315}\u{031a}ngerzone";
    assert_eq!(is_nfd_stream_safe_quick(okay.chars()), IsNormalized::Yes);

    let too_much = "Da\u{031b}\u{0316}\u{0317}\u{0318}\u{0319}\u{031c}\u{031d}\u{031e}\u{0300}\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}\u{0308}\u{0309}\u{030a}\u{030b}\u{030c}\u{030d}\u{030e}\u{030f}\u{0310}\u{0311}\u{0312}\u{0313}\u{0314}\u{0315}\u{031a}ngerzone";
    assert_eq!(is_nfd_stream_safe_quick(too_much.chars()), IsNormalized::No);
}

#[test]
fn test_stream_safe_nfc() {
    let okay = "ok\u{e0}\u{031b}\u{0316}\u{0317}\u{0318}\u{0319}\u{031c}\u{031d}\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}\u{0308}\u{0309}\u{030a}\u{030b}\u{030c}\u{030d}\u{030e}\u{030f}\u{0310}\u{0311}\u{0312}\u{0313}\u{0314}\u{0315}\u{031a}y";
    assert_eq!(is_nfc_stream_safe_quick(okay.chars()), IsNormalized::Maybe);

    let too_much = "not ok\u{e0}\u{031b}\u{0316}\u{0317}\u{0318}\u{0319}\u{031c}\u{031d}\u{031e}\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}\u{0308}\u{0309}\u{030a}\u{030b}\u{030c}\u{030d}\u{030e}\u{030f}\u{0310}\u{0311}\u{0312}\u{0313}\u{0314}\u{0315}\u{031a}y";
    assert_eq!(is_nfc_stream_safe_quick(too_much.chars()), IsNormalized::No);
}
