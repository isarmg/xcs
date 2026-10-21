#[cfg(all(test, feature = "proc-macro2"))]
mod test {
    use proc_macro2::{Punct, Spacing, TokenTree};
    use quote::quote;

    use super::*;

    #[test]
    fn punctuation() {
        let mut tokens = quote! {=<>!$~+-*/%^|@.,;:#$?'a}.into_iter();
        assert!(tokens.next().unwrap().is_equals());
        assert!(tokens.next().unwrap().is_less_than());
        assert!(tokens.next().unwrap().is_greater_than());
        assert!(tokens.next().unwrap().is_exclamation());
        assert!(tokens.next().unwrap().is_dollar());
        assert!(tokens.next().unwrap().is_tilde());
        assert!(tokens.next().unwrap().is_plus());
        assert!(tokens.next().unwrap().is_minus());
        assert!(tokens.next().unwrap().is_asterix());
        assert!(tokens.next().unwrap().is_slash());
        assert!(tokens.next().unwrap().is_percent());
        assert!(tokens.next().unwrap().is_caret());
        assert!(tokens.next().unwrap().is_pipe());
        assert!(tokens.next().unwrap().is_at());
        assert!(tokens.next().unwrap().is_dot());
        assert!(tokens.next().unwrap().is_comma());
        assert!(tokens.next().unwrap().is_semi());
        assert!(tokens.next().unwrap().is_colon());
        assert!(tokens.next().unwrap().is_pound());
        assert!(tokens.next().unwrap().is_dollar());
        assert!(tokens.next().unwrap().is_question());
        assert!(tokens.next().unwrap().is_quote());
    }

    #[test]
    fn token_stream_ext() {
        let mut tokens = quote!(a);
        tokens.push(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
        assert_eq!(tokens.to_string(), "a ,");
    }

    #[test]
    fn token_tree_ext() {
        let mut tokens = quote!({group} ident + "literal").into_iter().peekable();
        assert!(tokens.peek().unwrap().is_group());
        assert!(matches!(
            tokens.next().unwrap().group().unwrap().to_string().as_str(),
            "{ group }" | "{group}"
        ));
        assert!(tokens.peek().unwrap().is_ident());
        assert_eq!(tokens.next().unwrap().ident().unwrap().to_string(), "ident");
        assert!(tokens.peek().unwrap().is_punct());
        assert_eq!(tokens.next().unwrap().punct().unwrap().to_string(), "+");
        assert!(tokens.peek().unwrap().is_literal());
        assert_eq!(
            tokens.next().unwrap().literal().unwrap().to_string(),
            "\"literal\""
        );
    }

    #[test]
    fn test() {}
}
