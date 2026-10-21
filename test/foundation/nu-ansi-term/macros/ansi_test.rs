#[cfg(test)]
#[allow(unused)]
macro_rules! test {
    ($name: ident: $style: expr; $input: expr => $result: expr) => {
        #[test]
        fn $name() {
            assert_eq!($style.paint($input).to_string(), $result.to_string());

            let mut v = Vec::new();
            $style.paint($input.as_bytes()).write_to(&mut v).unwrap();
            assert_eq!(v.as_slice(), $result.as_bytes());
        }
    };
}
