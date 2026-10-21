#[test]
#[should_panic]
fn chunks_panic_on_cap_zero() {
    let _ = list().chunks(0);
}
