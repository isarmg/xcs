#[cfg(all(test))]
#[test]
fn assert_no_stack_overflow() {
    // https://github.com/tokio-rs/tokio/issues/5293

    struct Foo {
        _a: [u8; 2_000_000],
    }

    assert_eq!(
        Layout::new::<MaybeUninit<Block<Foo>>>(),
        Layout::new::<Block<Foo>>()
    );

    let _block = Block::<Foo>::new(0);
}
