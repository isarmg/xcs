mod alloc {}
mod core {}
mod either {}
mod std {}

#[test]
fn iproduct_hygiene() {
    let _ = iterator_toolkit::iproduct!();
    let _ = iterator_toolkit::iproduct!(0..6);
    let _ = iterator_toolkit::iproduct!(0..6, 0..9);
    let _ = iterator_toolkit::iproduct!(0..6, 0..9, 0..12);
}

#[test]
fn izip_hygiene() {
    let _ = iterator_toolkit::izip!(0..6);
    let _ = iterator_toolkit::izip!(0..6, 0..9);
    let _ = iterator_toolkit::izip!(0..6, 0..9, 0..12);
}

#[test]
fn chain_hygiene() {
    let _: ::std::iter::Empty<i32> = iterator_toolkit::chain!();
    let _ = iterator_toolkit::chain!(0..6);
    let _ = iterator_toolkit::chain!(0..6, 0..9);
    let _ = iterator_toolkit::chain!(0..6, 0..9, 0..12);
}
