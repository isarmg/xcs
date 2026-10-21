#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        Array,
        sizes::{U2, U5},
    };

    #[test]
    fn flatten() {
        let flat: Array<u8, _> = Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let unflat2: Array<Array<u8, _>, _> = Array([
            Array([1, 2]),
            Array([3, 4]),
            Array([5, 6]),
            Array([7, 8]),
            Array([9, 10]),
        ]);
        let unflat5: Array<Array<u8, _>, _> =
            Array([Array([1, 2, 3, 4, 5]), Array([6, 7, 8, 9, 10])]);

        // Flatten
        let actual = unflat2.flatten();
        assert_eq!(flat, actual);

        let actual = unflat5.flatten();
        assert_eq!(flat, actual);

        // Unflatten
        let actual: Array<Array<u8, U2>, U5> = flat.unflatten();
        assert_eq!(unflat2, actual);

        let actual: Array<Array<u8, U5>, U2> = flat.unflatten();
        assert_eq!(unflat5, actual);

        // Unflatten on references
        let actual: Array<&Array<u8, U2>, U5> = (&flat).unflatten();
        for (i, part) in actual.iter().enumerate() {
            assert_eq!(&unflat2[i], *part);
        }

        let actual: Array<&Array<u8, U5>, U2> = (&flat).unflatten();
        for (i, part) in actual.iter().enumerate() {
            assert_eq!(&unflat5[i], *part);
        }
    }
}
