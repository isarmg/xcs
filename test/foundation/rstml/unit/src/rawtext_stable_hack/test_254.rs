#[cfg(test)]
mod test {
    use quote::quote;

    use super::MacroPattern;

    #[test]
    fn macro_content_matcher() {
        let pattern = MacroPattern::from_token_stream(quote! {html!{ctx, other_arg, %%}}).unwrap();
        let content = pattern
            .match_content(quote! {html!{ctx, div, <div> <Foo/> </div>}})
            .unwrap();

        assert_eq!(
            content.to_string(),
            quote! {<div> <Foo/> </div>}.to_string()
        );
    }

    #[test]
    fn macro_content_matcher_ignore_single_percent() {
        let pattern = MacroPattern::from_token_stream(quote! {html!{ctx, %other_arg, %%}}).unwrap();
        // also check that ignore percent inside html input
        let content = pattern
            .match_content(quote! {html!{ctx, %div, <div>%% <Foo/> </div>}})
            .unwrap();

        assert_eq!(
            content.to_string(),
            quote! {<div>%% <Foo/> </div>}.to_string()
        );
    }

    #[test]
    fn macro_content_matcher_group() {
        // group type does not count
        let pattern =
            MacroPattern::from_token_stream(quote! {html!{ctx, other_arg, {%%}}}).unwrap();
        let content = pattern
            .match_content(quote! {html!{ctx, div, [<div> <Foo/> </div>]}})
            .unwrap();

        assert_eq!(
            content.to_string(),
            quote! {<div> <Foo/> </div>}.to_string()
        );
    }

    #[test]
    fn macro_content_matcher_with_postfix_group() {
        // group type does not count
        let pattern = MacroPattern::from_token_stream(
            quote! {html!{ctx, other_arg, {%%}, any other context}},
        )
        .unwrap();
        let content = pattern
            .match_content(quote! {html!{ctx, div, [<div> <Foo/> </div>], foo}})
            .unwrap();

        assert_eq!(
            content.to_string(),
            quote! {<div> <Foo/> </div>}.to_string()
        );
    }

    #[test]
    fn extend_macro_matcher_using_until_token() {
        // group type does not count
        let pattern = MacroPattern::from_token_stream(
            quote! {html!{ctx, other_arg, {%%}, any other context}},
        )
        .unwrap()
        .skip_until(quote! {<Baz/>});

        let content = pattern
            .match_content(quote! {html!{ctx, div, [any content before Baz tag is ignored <div> </div> {foo:red} <Baz/> <div> <Foo/> </div>], foo}})
            .unwrap();

        assert_eq!(
            content.to_string(),
            quote! {<div> <Foo/> </div>}.to_string()
        );
    }

    #[test]
    fn check_macro_patch_differ() {
        let pattern = MacroPattern::from_token_stream(quote! {html!{ctx, other_arg, %%}}).unwrap();
        let content = pattern
            .match_content(
                quote! {some_other::path::to_macro::html!{ctx, div, <div> <Foo/> </div>}},
            )
            .unwrap();

        assert_eq!(
            content.to_string(),
            quote! {<div> <Foo/> </div>}.to_string()
        );
    }
}
