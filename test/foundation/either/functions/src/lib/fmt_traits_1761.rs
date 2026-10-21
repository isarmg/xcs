#[test]
fn fmt_traits() {
    use std::format;

    let value: Either<u32, i32> = Left(0xC0DE);
    // fmt::Binary
    assert_eq!("1100000011011110", format!("{value:b}"));
    // fmt::Display
    assert_eq!("49374", format!("{value}"));
    // fmt::LowerExp
    assert_eq!("4.9374e4", format!("{value:e}"));
    // fmt::LowerHex
    assert_eq!("c0de", format!("{value:x}"));
    // fmt::Octal
    assert_eq!("140336", format!("{value:o}"));
    // fmt::UpperExp
    assert_eq!("4.9374E4", format!("{value:E}"));
    // fmt::UpperHex
    assert_eq!("C0DE", format!("{value:X}"));

    // fmt::Pointer
    let pointer: Either<*const u32, &mut i32> = Left(std::ptr::null());
    assert_eq!("0x0", format!("{pointer:p}"));
}
