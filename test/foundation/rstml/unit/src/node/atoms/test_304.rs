#[cfg(test)]
mod test {
    use rust_syntax::custom_punctuation;

    use super::*;

    macro_rules! parse_quote {
            ($mod_name:ident, $name: ident=> $($tts:tt)*) => {
                mod $mod_name {
                    use super::*;
                    // use super::tokens::*;
                    #[test]
                    fn parse_quote() {

                        let tts = quote::quote!{
                            $($tts)*
                        };
                        rust_syntax::parse2::<$name>(tts).unwrap();

                    }
                }
            }
        }

    parse_quote! {docstart, DocStart => <!}

    parse_quote! {comstart, ComStart => <!--}

    parse_quote! {comend, ComEnd => -->}

    parse_quote! {open_tag_end1, OpenTagEnd => >}

    parse_quote! {open_tag_end2, OpenTagEnd => />}

    parse_quote! {close_tag_start, CloseTagStart => </}

    /// Custom punctuation wasnt compatible with quote,
    /// check if it now compatible to replace simple parser with it  
    #[test]
    fn parse_quote_doc_comp_custom_punct() {
        custom_punctuation!(CloseTagStart2, </);
        let tts = quote::quote! {
            </
        };
        rust_syntax::parse2::<CloseTagStart2>(tts).unwrap_err();
    }
}
