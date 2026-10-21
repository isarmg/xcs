#[cfg(test)]
mod tests {
    use super::*;

    // compile time test
    #[test]
    #[expect(clippy::type_complexity)]
    fn either_macro() {
        let a: f64 = 0.0;
        let _: Either!(f64) = a;
        let a: Either<&str, f64> = either!(12,
            12 => "12",
            _ => 0.0f64,
        );
        let _: Either!(&str, f64) = a;
        let a: EitherOf3<&str, f64, i32> = either!(12,
            12 => "12",
            13 => 0.0f64,
            _ => 12i32,
        );
        let _: Either!(&str, f64, i32) = a;
        let a: EitherOf4<&str, f64, char, i32> = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            _ => 12i32,
        );
        let _: Either!(&str, f64, char, i32) = a;
        let a: EitherOf5<&str, f64, char, f32, i32> = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            15 => 0.0f32,
            _ => 12i32,
        );
        let _: Either!(&str, f64, char, f32, i32) = a;
        let a: EitherOf6<&str, f64, char, f32, u8, i32> = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            _ => 12i32,
        );
        let _: Either!(&str, f64, char, f32, u8, i32) = a;
        let a: EitherOf7<&str, f64, char, f32, u8, i8, i32> = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            17 => 2i8,
            _ => 12i32,
        );
        let _: Either!(&str, f64, char, f32, u8, i8, i32) = a;
        let a: EitherOf8<&str, f64, char, f32, u8, i8, u16, i32> = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            17 => 2i8,
            18 => 42u16,
            _ => 12i32,
        );
        let _: Either!(&str, f64, char, f32, u8, i8, u16, i32) = a;
        let a: EitherOf9<&str, f64, char, f32, u8, i8, u16, i16, i32> = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            17 => 2i8,
            18 => 42u16,
            19 => 64i16,
            _ => 12i32,
        );
        let _: Either!(&str, f64, char, f32, u8, i8, u16, i16, i32) = a;
        let a: EitherOf10<&str, f64, char, f32, u8, i8, u16, i16, u32, i32> = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            17 => 2i8,
            18 => 42u16,
            19 => 64i16,
            20 => 84u32,
            _ => 12i32,
        );
        let _: Either!(&str, f64, char, f32, u8, i8, u16, i16, u32, i32) = a;
        let a: EitherOf11<
            &str,
            f64,
            char,
            f32,
            u8,
            i8,
            u16,
            i16,
            u32,
            i32,
            u64,
        > = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            17 => 2i8,
            18 => 42u16,
            19 => 64i16,
            20 => 84u32,
            21 => 12i32,
            _ => 13u64,
        );
        let _: Either!(&str, f64, char, f32, u8, i8, u16, i16, u32, i32, u64,) =
            a;
        let a: EitherOf12<
            &str,
            f64,
            char,
            f32,
            u8,
            i8,
            u16,
            i16,
            u32,
            i32,
            u64,
            i64,
        > = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            17 => 2i8,
            18 => 42u16,
            19 => 64i16,
            20 => 84u32,
            21 => 12i32,
            22 => 13u64,
            _ => 14i64,
        );
        let _: Either!(
            &str, f64, char, f32, u8, i8, u16, i16, u32, i32, u64, i64
        ) = a;
        let a: EitherOf13<
            &str,
            f64,
            char,
            f32,
            u8,
            i8,
            u16,
            i16,
            u32,
            i32,
            u64,
            i64,
            u128,
        > = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            17 => 2i8,
            18 => 42u16,
            19 => 64i16,
            20 => 84u32,
            21 => 12i32,
            22 => 13u64,
            23 => 14i64,
            _ => 15u128,
        );
        let _: Either!(
            &str, f64, char, f32, u8, i8, u16, i16, u32, i32, u64, i64, u128,
        ) = a;
        let a: EitherOf14<
            &str,
            f64,
            char,
            f32,
            u8,
            i8,
            u16,
            i16,
            u32,
            i32,
            u64,
            i64,
            u128,
            i128,
        > = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            17 => 2i8,
            18 => 42u16,
            19 => 64i16,
            20 => 84u32,
            21 => 12i32,
            22 => 13u64,
            23 => 14i64,
            24 => 15u128,
            _ => 16i128,
        );
        let _: Either!(
            &str, f64, char, f32, u8, i8, u16, i16, u32, i32, u64, i64, u128,
            i128,
        ) = a;
        let a: EitherOf15<
            &str,
            f64,
            char,
            f32,
            u8,
            i8,
            u16,
            i16,
            u32,
            i32,
            u64,
            i64,
            u128,
            i128,
            [u8; 1],
        > = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            17 => 2i8,
            18 => 42u16,
            19 => 64i16,
            20 => 84u32,
            21 => 12i32,
            22 => 13u64,
            23 => 14i64,
            24 => 15u128,
            25 => 16i128,
            _ => [1u8],
        );
        let _: Either!(
            &str, f64, char, f32, u8, i8, u16, i16, u32, i32, u64, i64, u128,
            i128, [u8; 1],
        ) = a;
        let a: EitherOf16<
            &str,
            f64,
            char,
            f32,
            u8,
            i8,
            u16,
            i16,
            u32,
            i32,
            u64,
            i64,
            u128,
            i128,
            [u8; 1],
            [i8; 1],
        > = either!(12,
            12 => "12",
            13 => 0.0f64,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            17 => 2i8,
            18 => 42u16,
            19 => 64i16,
            20 => 84u32,
            21 => 12i32,
            22 => 13u64,
            23 => 14i64,
            24 => 15u128,
            25 => 16i128,
            26 => [1u8],
            _ => [1i8],
        );
        let _: Either!(
            &str, f64, char, f32, u8, i8, u16, i16, u32, i32, u64, i64, u128,
            i128, [u8; 1], [i8; 1],
        ) = a;
    }

    #[test]
    #[should_panic]
    fn unwrap_wrong_either() {
        Either::<i32, &str>::Left(0).unwrap_right();
    }

    #[test]
    fn either_or() {
        let right = false.either_or(|_| 'a', |_| 12);
        assert!(matches!(right, Either::Right(12)));

        let left = true.either_or(|_| 'a', |_| 12);
        assert!(matches!(left, Either::Left('a')));

        let left = Some(12).either_or(|a| a, |_| 'a');
        assert!(matches!(left, Either::Left(12)));
        let right = None.either_or(|a: i32| a, |_| 'a');
        assert!(matches!(right, Either::Right('a')));

        let result: Result<_, ()> = Ok(1.2f32);
        let left = result.either_or(|a| a * 2f32, |b| b);
        assert!(matches!(left, Either::Left(2.4f32)));

        let result: Result<i32, _> = Err("12");
        let right = result.either_or(|a| a, |b| b.chars().next());
        assert!(matches!(right, Either::Right(Some('1'))));

        let either = Either::<i32, char>::Left(12);
        let left = either.either_or(|a| a, |b| b);
        assert!(matches!(left, Either::Left(12)));

        let either = Either::<i32, char>::Right('a');
        let right = either.either_or(|a| a, |b| b);
        assert!(matches!(right, Either::Right('a')));
    }
}
