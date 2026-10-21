#[cfg(test)]
mod test {
    use super::*;
    use crate::Color::*;

    #[test]
    fn test() {
        let l = [
            Black.paint("first"),
            Red.paint("-second"),
            White.paint("-third"),
        ];
        let a = AnsiStrings(&l);
        assert_eq!(unstyle(&a), "first-second-third");
        assert_eq!(unstyled_len(&a), 18);

        let l2 = [Black.paint("st"), Red.paint("-second"), White.paint("-t")];
        assert_eq!(sub_string(3, 11, &a), l2);
    }
}
