#[cfg(test)]
mod test {
    use quote::quote;

    use super::*;
    use crate::assert_tokens;

    #[test]
    fn ty() {
        let mut at = TokenParser::new(quote! {Name, <Some, Generic, Type>});
        assert_tokens!(at.next_type().unwrap(), { Name });
        at.next();
        assert_tokens!(
            at.next_type().unwrap(),
            { < Some , Generic , Type > }
        );
    }

    #[test]
    fn expr() {
        let mut at = TokenParser::new(
            quote! {a + b, <Some, Generic, Type>::something + <a,b> * a < b, "hi" => hello},
        );
        assert_tokens!(at.next_expression().unwrap(), { a + b });
        at.next();
        assert_tokens!(
            at.next_expression().unwrap(), { <Some, Generic, Type>::something + <a,b> * a < b }
        );
        at.next();
        assert_tokens!(at.next_expression().unwrap(), { "hi" });
        at.next();
        at.next();
        assert_tokens!(at.next_expression().unwrap(), { hello });

        let mut at = TokenParser::from_str("1..,").unwrap();
        let expr: Vec<_> = at.next_expression().unwrap().into_iter().collect();
        assert!(expr.last().unwrap().is_alone());
        assert_tokens!(expr, { 1.. });
    }

    #[test]
    fn combined_tokens() {
        // using from_str to be able to verify behavior of splitting the input correctly
        // into tts
        let mut parser = TokenParser::from_str("->&&..=>=+,-..,+=").unwrap();
        assert_tokens!(parser.next_tt_r_arrow().unwrap(), { -> });
        assert_tokens!(parser.next_tt_and_and().unwrap(), { && });
        assert_tokens!(parser.next_tt_dot_dot_eq().unwrap(), { ..= });
        assert_tokens!(parser.next_tt_ge().unwrap(), { >= });
        assert_tokens!(parser.next_tt_plus().unwrap(), { + });
        assert_tokens!(parser.next_tt_comma().unwrap(), { , });
        assert_tokens!(parser.next_tt_minus().unwrap(), { - });
        assert_tokens!(parser.next_tt_dot_dot().unwrap(), { .. });
        assert_tokens!(parser.next_tt_comma().unwrap(), { , });
        assert_tokens!(parser.next_tt_plus_eq().unwrap(), { += });
    }

    #[test]
    fn peek() {
        let mut parser = TokenParser::new(quote! {
            0 {} 2 3 += .. =
        });
        assert_eq!(parser.peek().unwrap().to_string(), "0");
        assert_eq!(parser.peek_n(0).unwrap().to_string(), "0");
        assert_eq!(parser.peek_n(1).unwrap().to_string().replace(' ', ""), "{}");
        assert_eq!(parser.peek_n(2).unwrap().to_string(), "2");

        assert_eq!(parser.peek_literal().unwrap().to_string(), "0");
        assert!(parser.peek_group().is_none());
        parser.next().unwrap();
        assert!(parser.peek_group().is_some());
        assert!(parser.peek_n_tt_plus_eq(3).is_some());
        assert!(parser.peek_n_tt_dot_dot(5).is_some());
    }

    #[test]
    fn keyword() {
        let mut parser: TokenParser<_, 4> = TokenParser::from(quote! {
            in out and or
        });
        assert_eq!(parser.next_keyword("in").unwrap().to_string(), "in");
        assert_eq!(parser.next_keyword("out").unwrap().to_string(), "out");
        assert!(parser.next_keyword("or").is_none());
        assert_eq!(parser.next_keyword("and").unwrap().to_string(), "and");
        assert_eq!(parser.next_keyword("or").unwrap().to_string(), "or");
        assert!(parser.next_keyword("or").is_none());
    }
}
