use super::*;

#[test]
fn base_styles_and_classes_keep_order_before_additions_and_spread() {
    let mut nodes = rstml::parse2(quote! {
        <div id="first" style="display:block" class="base"
            class:toggled=true style:color="red"
            class=("later", true) style=("font-size", "12px")
            {..extra} title="after-spread" />
    })
    .unwrap();
    render_view(&mut nodes, None, None, true).unwrap();
    let Node::Element(element) = &nodes[0] else {
        panic!("expected element");
    };
    let keys = element
        .attributes()
        .iter()
        .map(|attribute| match attribute {
            NodeAttribute::Attribute(attribute) => attribute.key.to_string(),
            NodeAttribute::Block(_) => "SPREAD".into(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        [
            "style",
            "class",
            "id",
            "class:toggled",
            "style:color",
            "class",
            "style",
            "SPREAD",
            "title"
        ]
    );
}

#[test]
fn many_tuple_additions_keep_source_order_without_invalid_comparison() {
    let additions = (0..96)
        .map(|index| {
            let name = format!("toggle-{index}");
            if index % 2 == 0 {
                quote! { class=(#name, true) }
            } else {
                quote! { style=(#name, "value") }
            }
        })
        .collect::<TokenStream>();
    let mut nodes = rstml::parse2(quote! { <div #additions /> }).unwrap();
    let Node::Element(element) = &nodes[0] else {
        panic!("expected element");
    };
    let before = element
        .attributes()
        .iter()
        .map(ToTokens::to_token_stream)
        .map(|tokens| tokens.to_string())
        .collect::<Vec<_>>();
    render_view(&mut nodes, None, None, true).unwrap();
    let Node::Element(element) = &nodes[0] else {
        panic!("expected element");
    };
    let after = element
        .attributes()
        .iter()
        .map(ToTokens::to_token_stream)
        .map(|tokens| tokens.to_string())
        .collect::<Vec<_>>();
    assert_eq!(after, before);
}
