#[test]
#[cfg(feature = "std")]
fn print_size_of() {
    use core::mem::size_of;
    dbg!(size_of::<Style>());
    dbg!(size_of::<StyleDisplay>());
}
