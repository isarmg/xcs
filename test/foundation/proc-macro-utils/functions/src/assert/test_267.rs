#[test]
fn test() {
    // TODO testing with quote is incomplete `":::"` can be joint joint alone if
    // produced directly not with quote.
    use quote::quote;
    assert_tokens!(quote!(ident ident, { group/test, vec![a, (a + b)] }, "literal" $), {
        ident ident, { group /test, vec![a,(a+b)] }, "literal" $
    });
    assert_tokens!(quote!(:::), {
        :::
    });
    assert_tokens!(quote!(more:::test::test:: hello :-D $$$ It should just work), {
        more ::: test ::test:: hello :-D $$$ It should just work
    });

    assert_tokens!(quote!(:$), {: $});
}
