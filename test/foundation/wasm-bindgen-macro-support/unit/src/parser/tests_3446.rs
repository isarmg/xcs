#[cfg(test)]
mod tests {
    #[test]
    fn test_try_unescape() {
        use super::try_unescape;
        assert_eq!(try_unescape("hello").unwrap(), "hello");
        assert_eq!(try_unescape("\"hello").unwrap(), "hello");
        assert_eq!(try_unescape("hello\"").unwrap(), "hello");
        assert_eq!(try_unescape("\"hello\"").unwrap(), "hello");
        assert_eq!(try_unescape("hello\\\\").unwrap(), "hello\\");
        assert_eq!(try_unescape("hello\\n").unwrap(), "hello\n");
        assert_eq!(try_unescape("hello\\u"), None);
        assert_eq!(try_unescape("hello\\u{"), None);
        assert_eq!(try_unescape("hello\\u{}"), None);
        assert_eq!(try_unescape("hello\\u{0}").unwrap(), "hello\0");
        assert_eq!(try_unescape("hello\\u{000000}").unwrap(), "hello\0");
        assert_eq!(try_unescape("hello\\u{0000000}"), None);
    }

    /// Return the namespace for each parsed import.
    fn import_namespaces(
        block_opts: proc_macro2::TokenStream,
        block: rust_syntax::ItemForeignMod,
    ) -> Result<Vec<Option<Vec<String>>>, super::Diagnostic> {
        use super::{ast, BindgenAttrs, MacroParse};

        let opts: BindgenAttrs = rust_syntax::parse2(block_opts).unwrap();
        let mut program = ast::Program::default();
        block.macro_parse(&mut program, opts)?;
        Ok(program
            .imports
            .into_iter()
            .map(|import| import.js_namespace)
            .collect())
    }

    fn ns(segments: &[&str]) -> Option<Vec<String>> {
        Some(segments.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn js_namespace_only_on_block() {
        let namespaces = import_namespaces(
            quote::quote! { js_namespace = ["a"] },
            rust_syntax::parse_quote! {
                extern "C" {
                    fn my_function();
                }
            },
        )
        .unwrap();
        assert_eq!(namespaces, vec![ns(&["a"])]);
    }

    #[test]
    fn js_namespace_only_on_item() {
        let namespaces = import_namespaces(
            quote::quote! {},
            rust_syntax::parse_quote! {
                extern "C" {
                    #[wasm_bindgen(js_namespace = ["b"])]
                    fn my_function();
                }
            },
        )
        .unwrap();
        assert_eq!(namespaces, vec![ns(&["b"])]);
    }

    #[test]
    fn js_namespace_absent_everywhere() {
        let namespaces = import_namespaces(
            quote::quote! {},
            rust_syntax::parse_quote! {
                extern "C" {
                    fn my_function();
                }
            },
        )
        .unwrap();
        assert_eq!(namespaces, vec![None]);
    }
}
