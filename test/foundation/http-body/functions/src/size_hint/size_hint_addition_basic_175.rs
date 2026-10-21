/// Asserts that some "real data" gets passed through without issue
#[test]
fn size_hint_addition_basic() {
    let exact_l = SizeHint::with_exact(20);
    let exact_r = SizeHint::with_exact(5);

    assert_eq!(Some(25), (exact_l + exact_r).exact());

    let inexact_l = SizeHint {
        lower: 25,
        upper: None,
    };
    let inexact_r = SizeHint {
        lower: 10,
        upper: Some(50),
    };

    let inexact = inexact_l + inexact_r;

    assert_eq!(inexact.lower(), 35);
    assert_eq!(inexact.upper(), None);

    let exact_inexact = exact_l + inexact_r;

    assert_eq!(exact_inexact.lower(), 30);
    assert_eq!(exact_inexact.upper(), Some(70));

    // same as previous but reversed operation order
    let inexact_exact = inexact_r + exact_l;

    assert_eq!(inexact_exact.lower(), 30);
    assert_eq!(inexact_exact.upper(), Some(70));
}
