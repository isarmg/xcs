#[cfg(test)]
mod tests {
    use super::*;

    // We use a thread with an explicit stack size to test that our destructor
    // for Ast can handle arbitrarily sized expressions in constant stack
    // space. In case we run on a platform without threads (WASM?), we limit
    // this test to Windows/Unix.
    #[test]
    #[cfg(any(unix, windows))]
    fn no_stack_overflow_on_drop() {
        use std::thread;

        let run = || {
            let span = || Span::splat(Position::new(0, 0, 0));
            let mut ast = Ast::empty(span());
            for i in 0..200 {
                ast = Ast::group(Group {
                    span: span(),
                    kind: GroupKind::CaptureIndex(i),
                    ast: Box::new(ast),
                });
            }
            assert!(!ast.is_empty());
        };

        // We run our test on a thread with a small stack size so we can
        // force the issue more easily.
        //
        // NOTE(2023-03-21): It turns out that some platforms (like FreeBSD)
        // will just barf with very small stack sizes. So we bump this up a bit
        // to give more room to breath. When I did this, I confirmed that if
        // I remove the custom `Drop` impl for `Ast`, then this test does
        // indeed still fail with a stack overflow. (At the time of writing, I
        // had to bump it all the way up to 32K before the test would pass even
        // without the custom `Drop` impl. So 16K seems like a safe number
        // here.)
        //
        // See: https://github.com/rust-lang/regex/issues/967
        thread::Builder::new()
            .stack_size(16 << 10)
            .spawn(run)
            .unwrap()
            .join()
            .unwrap();
    }

    // This tests that our `Ast` has a reasonable size. This isn't a hard rule
    // and it can be increased if given a good enough reason. But this test
    // exists because the size of `Ast` was at one point over 200 bytes on a
    // 64-bit target. Wow.
    #[test]
    fn ast_size() {
        let max = 2 * core::mem::size_of::<usize>();
        let size = core::mem::size_of::<Ast>();
        assert!(
            size <= max,
            "Ast size of {size} bytes is bigger than suggested max {max}",
        );
    }
}
