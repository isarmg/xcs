#[cfg(test)]
mod test {
    use proc_macro::Span;
    use quote::quote;

    use crate::{Emitter, ErrorMessage};

    macro_rules! returned {
        ($ty:ty, $expr:expr) => {
            #[allow(unreachable_code)]
            (|| -> $ty {
                $expr;
                unreachable!();
            })()
        };
    }
    #[test]
    fn bail() {
        assert_eq!(
            returned!(Result<(), ErrorMessage>, bail!("format"))
                .unwrap_err()
                .to_string(),
            "format"
        );
        assert_eq!(
            returned!(Result<(), ErrorMessage>, bail!("format {}", 1))
                .unwrap_err()
                .to_string(),
            "format 1"
        );
        let b = "ho";
        assert_eq!(
            returned!(Result<(), ErrorMessage>, bail!("format {} {a} {} {b}", 1, 2, a = 4))
                .unwrap_err()
                .to_string(),
            "format 1 4 2 ho"
        );
    }

    #[test]
    fn error_message() {
        assert_eq!(error_message!("test").to_string(), "test");
        assert_eq!(error_message!("test";).to_string(), "test");
        assert_eq!(
            error_message!(
                "test";
                error = "hello {} {a}", 1 + 4, a = ""
            )
            .to_string(),
            "test\n\n  = error: hello 5 \n"
        );
        assert_eq!(
            error_message!(
                "test";
                error = "hello {} {a}", 1 + 4, a = "";
            )
            .to_string(),
            "test\n\n  = error: hello 5 \n"
        );
        assert_eq!(
            error_message!(
                "test";
                error = "hello {} {a}", 1 + 4, a = "",;
                hint = "a hint"
            )
            .to_string(),
            "test\n\n  = error: hello 5 \n  = hint: a hint\n"
        );
    }

    #[test]
    fn emit() {
        let mut emitter = Emitter::new();
        emit!(emitter, "an error message"; error = "with attachments");
        let span = proc_macro2::Span::call_site();
        // explicitly use `emit` in an expression position
        match () {
            () => emit!(emitter, span, "error message"),
        }
        #[cfg(feature = "syn")]
        {
            let error = syn::Error::new(proc_macro2::Span::call_site(), "an error");
            emit!(emitter, error);
        }
    }

    // Only tests that it compiles
    fn _error_message_spanned() {
        let span = Span::call_site();
        _ = error_message!(span, "test");

        let span = proc_macro::Span::call_site();
        _ = error_message!(span, "test");

        let tokens = quote!(test);
        _ = error_message!(tokens, "an error message",);
        _ = error_message!(tokens, "an error message",;);
        _ = error_message!(tokens, "an error message",; warning="and a warning";);
    }
}
