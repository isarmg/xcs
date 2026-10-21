/// Asserts that SizeHint addition is perfect with a basic proof
#[test]
fn size_hint_addition_proof() {
    /// Converts a SizeHint to a tuple for equality checks and matching
    fn to_parts(s: SizeHint) -> (u64, Option<u64>) {
        (s.lower(), s.upper())
    }

    // assuming addition itself is perfect, there are 3 distinct states:
    // (_, Some(_)) + (_, Some(_)) => (_ + _, Some(_ + _))
    // (_, Some(_)) + (_, None) => (_ + _, None)
    // (_, None) + (_, None) => (_ + _, None)
    //
    // we can assert this in the typesystem! (and name them for our tests)
    match (to_parts(SizeHint::new()), to_parts(SizeHint::new())) {
        ((_, Some(_)), (_, Some(_))) => {} // 1
        ((_, None), (_, None)) => {}       // 2

        // note that these cases are identical if we can prove lhs + rhs is equivalent to rhs + lhs
        // see below, we do prove that!
        ((_, Some(_)), (_, None)) => {} // 3
        ((_, None), (_, Some(_))) => {}
    }
    //
    // Additionally, we assert a with_exact remains intact if we add two with_exact's together
    //
    // Additionally, we assert that all operations are equivalent if we do a + b vs b + a

    // asserts a + b == b + a == eq
    macro_rules! reciprocal_add_eq {
        ($a:expr, $b:expr, $eq:expr) => {
            assert_eq!(to_parts($a.clone() + $b.clone()), $eq);
            assert_eq!(to_parts($b.clone() + $a.clone()), $eq);
        };
    }

    // note that we use increasing powers of two every time we fetch a number, this ensures all
    // numbers will add uniquely

    let exact_1 = SizeHint::with_exact(1);
    let exact_2 = SizeHint::with_exact(2);

    // with_exact
    reciprocal_add_eq!(exact_1, exact_2, to_parts(SizeHint::with_exact(1 + 2)));

    let some_lhs = SizeHint {
        lower: 4,
        upper: Some(8),
    };

    let some_rhs = SizeHint {
        lower: 16,
        upper: Some(32),
    };

    // case 1
    reciprocal_add_eq!(some_lhs, some_rhs, (4 + 16, Some(8 + 32)));

    let none_lhs = SizeHint {
        lower: 64,
        upper: None,
    };

    let none_rhs = SizeHint {
        lower: 128,
        upper: None,
    };

    // case 2
    reciprocal_add_eq!(none_lhs, none_rhs, (64 + 128, None));

    // case 3
    reciprocal_add_eq!(some_lhs, none_rhs, (4 + 128, None));
}
