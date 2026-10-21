#[cfg(test)]
#[cfg(feature = "std")]
mod test_translate_position {
    use super::*;

    #[test]
    fn empty() {
        let input = b"";
        let index = 0;
        let position = translate_position(&input[..], index);
        assert_eq!(position, (0, 0));
    }

    #[test]
    fn start() {
        let input = b"Hello";
        let index = 0;
        let position = translate_position(&input[..], index);
        assert_eq!(position, (0, 0));
    }

    #[test]
    fn end() {
        let input = b"Hello";
        let index = input.len() - 1;
        let position = translate_position(&input[..], index);
        assert_eq!(position, (0, input.len() - 1));
    }

    #[test]
    fn after() {
        let input = b"Hello";
        let index = input.len();
        let position = translate_position(&input[..], index);
        assert_eq!(position, (0, input.len()));
    }

    #[test]
    fn first_line() {
        let input = b"Hello\nWorld\n";
        let index = 2;
        let position = translate_position(&input[..], index);
        assert_eq!(position, (0, 2));
    }

    #[test]
    fn end_of_line() {
        let input = b"Hello\nWorld\n";
        let index = 5;
        let position = translate_position(&input[..], index);
        assert_eq!(position, (0, 5));
    }

    #[test]
    fn start_of_second_line() {
        let input = b"Hello\nWorld\n";
        let index = 6;
        let position = translate_position(&input[..], index);
        assert_eq!(position, (1, 0));
    }

    #[test]
    fn second_line() {
        let input = b"Hello\nWorld\n";
        let index = 8;
        let position = translate_position(&input[..], index);
        assert_eq!(position, (1, 2));
    }
}
