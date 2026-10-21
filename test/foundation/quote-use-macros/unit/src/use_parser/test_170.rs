#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use quote::ToTokens;
    use rust_syntax::parse::Parser;
    use rust_syntax::parse_str;

    use super::*;

    macro_rules! assert_use_item {
        ($use:literal, $($path:literal as $ident:ident),* $(,)*) => {
            let UseItem(uses) = parse_str($use).unwrap();
            let mut uses = uses.into_iter();
            $(
                let Use(path, ident) = uses.next().unwrap();
                assert_eq!(path.into_token_stream().to_string().replace(' ', ""), $path);
                assert_eq!(ident, stringify!($ident));
            )*
        };
    }

    #[test]
    fn use_item() {
        assert_use_item!("use ::a::b;", "::a::b" as b);
        assert_use_item!(
            "use a::{c, self, b};",
            "::a::c" as c,
            "::a" as a,
            "::a::b" as b
        );
        assert_use_item!("use a::{self as c, b as a};", "::a" as c, "::a::b" as a);
        assert_use_item!(
            "use a::{b::{a, b}, c};",
            "::a::b::a" as a,
            "::a::b::b" as b,
            "::a::c" as c
        );
        assert_use_item!("use #var::a;", "#var::a" as a);
        assert_use_item!("use ::a::#var::a;", "::a::#var::a" as a);
        assert_use_item!("use ::a::#var as a;", "::a::#var" as a);
    }

    macro_rules! assert_error {
        ($use:literal) => {
            UseItem::parse.parse_str($use).unwrap_err();
        };
    }

    #[test]
    fn error() {
        assert_error!("use ::a::#b;");
    }
}
