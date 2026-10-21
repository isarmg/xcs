#[cfg(test)]
mod test {
    use crate::*;
    use zerovec::*;

    #[test]
    fn test_zerovec() {
        let mut vec = ZeroVec::<TinyAsciiStr<7>>::new();

        vec.with_mut(|v| v.push("foobar".parse().unwrap()));
        vec.with_mut(|v| v.push("baz".parse().unwrap()));
        vec.with_mut(|v| v.push("quux".parse().unwrap()));

        let bytes = vec.as_bytes();

        let vec: ZeroVec<TinyAsciiStr<7>> = ZeroVec::parse_bytes(bytes).unwrap();

        assert_eq!(&*vec.get(0).unwrap(), "foobar");
        assert_eq!(&*vec.get(1).unwrap(), "baz");
        assert_eq!(&*vec.get(2).unwrap(), "quux");
    }
}
