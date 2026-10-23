//! The preprocessing we apply to doc comments.
//!
//! #[derive(Parser)] works in terms of "paragraphs". Paragraph is a sequence of
//! non-empty adjacent lines, delimited by sequences of blank (whitespace only) lines.



pub(crate) fn extract_doc_comment(attrs: &[rust_syntax::Attribute]) -> Vec<String> {
    // multiline comments (`/** ... */`) may have LFs (`\n`) in them,
    // we need to split so we could handle the lines correctly
    //
    // we also need to remove leading and trailing blank lines
    let mut lines: Vec<_> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| {
            // non #[doc = "..."] attributes are not our concern
            // we leave them for rustc to handle
            match &attr.meta {
                rust_syntax::Meta::NameValue(rust_syntax::MetaNameValue {
                    value:
                        rust_syntax::Expr::Lit(rust_syntax::ExprLit {
                            lit: rust_syntax::Lit::Str(s),
                            ..
                        }),
                    ..
                }) => Some(s.value()),
                _ => None,
            }
        })
        .skip_while(|s| is_blank(s))
        .flat_map(|s| {
            let lines = s
                .split('\n')
                .map(|s| {
                    // remove one leading space no matter what
                    let s = s.strip_prefix(' ').unwrap_or(s);
                    s.to_owned()
                })
                .collect::<Vec<_>>();
            lines
        })
        .collect();

    while let Some(true) = lines.last().map(|s| is_blank(s)) {
        lines.pop();
    }

    lines
}

pub(crate) fn format_doc_comment(
    lines: &[String],
    preprocess: bool,
    force_long: bool,
) -> (Option<String>, Option<String>) {
    if preprocess {
        let (short, long) = parse_markdown(lines);
        let long = long.or_else(|| force_long.then(|| short.clone()));

        (Some(remove_period(short)), long)
    } else if let Some(first_blank) = lines.iter().position(|s| is_blank(s)) {
        let short = lines[..first_blank].join("\n");
        let long = lines.join("\n");

        (Some(short), Some(long))
    } else {
        let short = lines.join("\n");
        let long = force_long.then(|| short.clone());

        (Some(short), long)
    }
}

fn split_paragraphs(lines: &[String]) -> Vec<String> {
    use std::iter;

    let mut last_line = 0;
    iter::from_fn(|| {
        let slice = &lines[last_line..];
        let start = slice.iter().position(|s| !is_blank(s)).unwrap_or(0);

        let slice = &slice[start..];
        let len = slice
            .iter()
            .position(|s| is_blank(s))
            .unwrap_or(slice.len());

        last_line += start + len;

        if len != 0 {
            Some(merge_lines(&slice[..len]))
        } else {
            None
        }
    })
    .collect()
}

fn remove_period(mut s: String) -> String {
    if s.ends_with('.') && !s.ends_with("..") {
        s.pop();
    }
    s
}

fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

fn merge_lines(lines: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    lines
        .into_iter()
        .map(|s| s.as_ref().trim().to_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_markdown(lines: &[String]) -> (String, Option<String>) {
    if lines.iter().any(|s| is_blank(s)) {
        let paragraphs = split_paragraphs(lines);
        let short = paragraphs[0].clone();
        let long = paragraphs.join("\n\n");
        (short, Some(long))
    } else {
        let short = merge_lines(lines);
        (short, None)
    }
}
