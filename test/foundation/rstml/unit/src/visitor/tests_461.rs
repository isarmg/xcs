#[cfg(test)]
mod tests {

    use quote::{quote, ToTokens};
    use rust_syntax::parse_quote;

    use super::*;
    use crate::Infallible;
    #[test]
    fn collect_node_names() {
        #[derive(Default)]
        struct TestVisitor {
            collected_names: Vec<NodeName>,
        }
        impl<C: CustomNode> Visitor<C> for TestVisitor {
            fn visit_node_name(&mut self, name: &mut NodeName) -> bool {
                self.collected_names.push(name.clone());
                true
            }
        }
        // empty impl
        impl rust_syntax::visit_mut::VisitMut for TestVisitor {}

        let stream = quote! {
            <div>
                <span></span>
                <span></span>
            </div>
            <!-- "comment" -->
            <foo attr key=value> </foo>
        };
        let mut nodes = crate::parse2(stream).unwrap();
        let visitor = visit_nodes(&mut nodes, TestVisitor::default());
        // convert node_names to string;
        let node_names = visitor
            .collected_names
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>();

        assert_eq!(
            node_names,
            vec!["div", "span", "span", "span", "span", "div", "foo", "key", "foo"]
        );
    }

    #[test]
    fn collect_node_elements() {
        #[derive(Default)]
        struct TestVisitor {
            collected_names: Vec<NodeName>,
        }
        impl<C: CustomNode> Visitor<C> for TestVisitor {
            fn visit_element(&mut self, node: &mut NodeElement<C>) -> bool {
                self.collected_names.push(node.open_tag.name.clone());
                true
            }
        }
        // empty impl
        impl rust_syntax::visit_mut::VisitMut for TestVisitor {}

        let stream = quote! {
            <div>
                <span></span>
                <span></span>
            </div>
            <!-- "comment" -->
            <foo attr key=value> </foo>
        };
        let mut nodes = crate::parse2(stream).unwrap();
        let visitor = visit_nodes(&mut nodes, TestVisitor::default());
        // convert node_names to string;
        let node_names = visitor
            .collected_names
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>();

        assert_eq!(node_names, vec!["div", "span", "span", "foo"]);
    }

    #[test]
    fn collect_rust_blocks() {
        #[derive(Default)]
        struct TestVisitor {
            collected_blocks: Vec<rust_syntax::Block>,
        }
        // empty impl
        impl<C: CustomNode> Visitor<C> for TestVisitor {}
        impl rust_syntax::visit_mut::VisitMut for TestVisitor {
            fn visit_block_mut(&mut self, i: &mut rust_syntax::Block) {
                self.collected_blocks.push(i.clone());
            }
        }

        let stream = quote! {
            <div>
            { let block = "in node position"; }
                <span { block_in_attr_position = foo }></span>
                <span var = {block_in_value}></span>
            </div>
            <!-- "comment" -->
        };
        let mut nodes = crate::parse2(stream).unwrap();
        let visitor = visit_nodes(&mut nodes, TestVisitor::default());
        // convert node_names to string;
        let blocks = visitor
            .collected_blocks
            .iter()
            .map(|block| block.to_token_stream().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            blocks,
            vec![
                "{ let block = \"in node position\" ; }",
                "{ block_in_attr_position = foo }",
                "{ block_in_value }",
            ]
        );
    }

    #[test]
    fn collect_raw_text() {
        #[derive(Default)]
        struct TestVisitor {
            collected_raw_text: Vec<RawText<Infallible>>,
        }
        impl<C: CustomNode> Visitor<C> for TestVisitor {
            fn visit_raw_node<AnyC: CustomNode>(&mut self, node: &mut RawText<AnyC>) -> bool {
                let raw = node.clone().convert_custom::<Infallible>();
                self.collected_raw_text.push(raw);
                true
            }
        }
        // empty impl
        impl rust_syntax::visit_mut::VisitMut for TestVisitor {}

        let stream = quote! {
            <!Doctype Other raw text >
            <div>
                <span>Some raw text</span>
                <span></span> And text after span
            </div>
            <!-- comment -->
            <foo attr key=value> </foo>
        };
        let mut nodes = crate::parse2(stream).unwrap();
        let visitor = visit_nodes(&mut nodes, TestVisitor::default());
        // convert collected_raw_text to string;
        let raw_text = visitor
            .collected_raw_text
            .iter()
            .map(crate::node::RawText::to_string_best)
            .collect::<Vec<_>>();

        assert_eq!(
            raw_text,
            vec![
                "Other raw text",
                "Some raw text",
                "And text after span",
                "comment"
            ]
        );
    }

    #[test]
    fn collect_string_literals() {
        #[derive(Default)]
        struct TestVisitor {
            collected_literals: Vec<rust_syntax::LitStr>,
        }
        impl<C: CustomNode> Visitor<C> for TestVisitor {}
        impl rust_syntax::visit_mut::VisitMut for TestVisitor {
            fn visit_lit_str_mut(&mut self, i: &mut rust_syntax::LitStr) {
                self.collected_literals.push(i.clone());
            }
        }

        let stream = quote! {
            <!Doctype Other raw text >
            <div>
                <span>"Some raw text"</span>
                <span></span>"And text after span"
            </div>
            <foo attr key=value> </foo>
        };
        let mut nodes = crate::parse2(stream).unwrap();
        let visitor = visit_nodes(&mut nodes, TestVisitor::default());
        // convert collected_literals to string;
        let literals = visitor
            .collected_literals
            .iter()
            .map(rust_syntax::LitStr::value)
            .collect::<Vec<_>>();

        assert_eq!(literals, vec!["Some raw text", "And text after span"]);
    }

    #[test]
    fn modify_text_visitor() {
        struct TestVisitor;
        impl<C: CustomNode> Visitor<C> for TestVisitor {
            fn visit_text_node(&mut self, node: &mut NodeText) -> bool {
                *node = parse_quote!("modified");
                true
            }
        }
        impl rust_syntax::visit_mut::VisitMut for TestVisitor {}

        let mut visitor = Walker::new(TestVisitor);

        let tokens = quote! {
            <div>
                <span>"Some raw text"</span>
                <span></span>"And text after span"
            </div>
        };
        let mut nodes = crate::parse2(tokens).unwrap();
        for node in &mut nodes {
            visitor.visit_node(node);
        }
        let result = quote! {
            #(#nodes)*
        };
        assert_eq!(
            result.to_string(),
            quote! {
                <div>
                    <span>"modified"</span>
                    <span></span>"modified"
                </div>
            }
            .to_string()
        );
    }
}
