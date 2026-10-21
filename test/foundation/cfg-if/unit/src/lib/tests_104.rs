#[cfg(test)]
mod tests {
    cfg_if! {
        if #[cfg(test)] {
            use core::option::Option as Option2;
            fn works1() -> Option2<u32> { Some(1) }
        } else {
            fn works1() -> Option<u32> { None }
        }
    }

    cfg_if! {
        if #[cfg(foo)] {
            fn works2() -> bool { false }
        } else if #[cfg(test)] {
            fn works2() -> bool { true }
        } else {
            fn works2() -> bool { false }
        }
    }

    cfg_if! {
        if #[cfg(foo)] {
            fn works3() -> bool { false }
        } else {
            fn works3() -> bool { true }
        }
    }

    cfg_if! {
        if #[cfg(test)] {
            use core::option::Option as Option3;
            fn works4() -> Option3<u32> { Some(1) }
        }
    }

    cfg_if! {
        if #[cfg(foo)] {
            fn works5() -> bool { false }
        } else if #[cfg(test)] {
            fn works5() -> bool { true }
        }
    }

    // In issue #90 there was a bug that caused only the first item within a
    // block to be annotated with the produced `#[cfg(...)]`. In this example,
    // it meant that the first `type _B` wasn't being omitted as it should have
    // been, which meant we had two `type _B`s, which caused an error. See also
    // the "Subtle" comment above.
    cfg_if!(
        if #[cfg(target_os = "no-such-operating-system-good-sir!")] {
            type _A = usize;
            type _B = usize;
        } else {
            type _A = i32;
            type _B = i32;
        }
    );

    #[cfg(not(msrv_test))]
    cfg_if! {
        if #[cfg(false)] {
            fn works6() -> bool { false }
        } else if #[cfg(true)] {
            fn works6() -> bool { true }
        } else if #[cfg(false)] {
            fn works6() -> bool { false }
        }
    }

    #[test]
    fn it_works() {
        assert!(works1().is_some());
        assert!(works2());
        assert!(works3());
        assert!(works4().is_some());
        assert!(works5());
        #[cfg(not(msrv_test))]
        assert!(works6());
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_usage_within_a_function() {
        cfg_if! {
            if #[cfg(debug_assertions)] {
                // we want to put more than one thing here to make sure that they
                // all get configured properly.
                assert!(cfg!(debug_assertions));
                assert_eq!(4, 2 + 2);
            } else {
                assert!(works1().is_some());
                assert_eq!(10, 5 + 5);
            }
        }
    }

    #[allow(dead_code)]
    trait Trait {
        fn blah(&self);
    }

    #[allow(dead_code)]
    struct Struct;

    impl Trait for Struct {
        cfg_if! {
            if #[cfg(feature = "blah")] {
                fn blah(&self) { unimplemented!(); }
            } else {
                fn blah(&self) { unimplemented!(); }
            }
        }
    }
}
