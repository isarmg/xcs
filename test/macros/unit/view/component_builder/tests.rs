use super::*;

#[test]
fn attribute_blocks_do_not_shift_the_explicit_props_boundary() {
    let mut nodes = rstml::parse2(quote! {
        <Widget {..extra} value=3 {..} title="html-attribute" />
    })
    .unwrap();
    let Node::Element(element) = &mut nodes[0] else {
        panic!("expected component");
    };
    let tokens = component_to_tokens(element, None, true, &mut Errors::default())
        .unwrap()
        .to_string();
    assert!(
        tokens.contains(". value ("),
        "required component prop is preserved"
    );
    assert!(
        !tokens.contains(". title ("),
        "HTML title must not be passed to the props builder"
    );
    assert!(
        tokens.contains("attribute :: title"),
        "title remains a spread HTML attribute"
    );
}
