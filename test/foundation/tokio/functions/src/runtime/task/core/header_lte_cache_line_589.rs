#[test]

fn header_lte_cache_line() {
    assert!(std::mem::size_of::<Header>() <= 8 * std::mem::size_of::<*const ()>());
}
