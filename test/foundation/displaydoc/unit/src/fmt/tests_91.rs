#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use proc_macro2::Span;

    fn assert(input: &str, fmt: &str, args: &str) {
        let mut display = Display {
            fmt: LitStr::new(input, Span::call_site()),
            args: TokenStream::new(),
        };
        display.expand_shorthand();
        assert_eq!(fmt, display.fmt.value());
        assert_eq!(args, display.args.to_string());
    }

    #[test]
    fn test_expand() {
        assert("fn main() {{ }}", "fn main() {{ }}", "");
    }

    #[test]
    #[cfg_attr(not(feature = "std"), ignore)]
    fn test_std_expand() {
        assert(
            "{v} {v:?} {0} {0:?}",
            "{} {:?} {} {:?}",
            ", v . __displaydoc_display () , v , _0 . __displaydoc_display () , _0",
        );
        assert("error {var}", "error {}", ", var . __displaydoc_display ()");

        assert(
            "error {var1}",
            "error {}",
            ", var1 . __displaydoc_display ()",
        );

        assert(
            "error {var1var}",
            "error {}",
            ", var1var . __displaydoc_display ()",
        );

        assert(
            "The path {0}",
            "The path {}",
            ", _0 . __displaydoc_display ()",
        );
        assert("The path {0:?}", "The path {:?}", ", _0");
    }

    #[test]
    #[cfg_attr(feature = "std", ignore)]
    fn test_nostd_expand() {
        assert(
            "{v} {v:?} {0} {0:?}",
            "{} {:?} {} {:?}",
            ", v , v , _0 , _0",
        );
        assert("error {var}", "error {}", ", var");

        assert("The path {0}", "The path {}", ", _0");
        assert("The path {0:?}", "The path {:?}", ", _0");

        assert("error {var1}", "error {}", ", var1");

        assert("error {var1var}", "error {}", ", var1var");
    }
}
