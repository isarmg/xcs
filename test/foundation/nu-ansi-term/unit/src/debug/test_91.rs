#[cfg(test)]
mod test {
    use crate::style::Color::*;
    use crate::style::Style;

    macro_rules! test {
        ($name: ident: $obj: expr => $result: expr) => {
            #[test]
            fn $name() {
                assert_eq!($result, format!("{:?}", $obj));
            }
        };
    }

    test!(empty:   Style::new()                  => "Style {}");
    test!(bold:    Style::new().bold()           => "Style { bold }");
    test!(italic:  Style::new().italic()         => "Style { italic }");
    test!(both:    Style::new().bold().italic()  => "Style { bold, italic }");

    test!(red:     Red.normal()                     => "Style { fg(Red) }");
    test!(redblue: Red.normal().on(Rgb(3, 2, 4))    => "Style { fg(Red), on(Rgb(3, 2, 4)) }");

    test!(everything:
            Red.on(Blue).blink().bold().dimmed().hidden().italic().reverse().strikethrough().underline() =>
            "Style { fg(Red), on(Blue), blink, bold, dimmed, hidden, italic, reverse, strikethrough, underline }");

    #[test]
    fn long_and_detailed() {
        let expected_debug = "Style { fg(Blue), bold }";
        let expected_pretty_repat = r"Style {
    foreground: Some(
        Blue,
    ),
    background: None,
    blink: false,
    bold: true,
    dimmed: false,
    hidden: false,
    italic: false,
    reverse: false,
    strikethrough: false,
    underline: false,
}";

        let style = Blue.bold();
        let style_fmt_debug = format!("{:?}", style);
        let style_fmt_pretty = format!("{:#?}", style);
        println!("style_fmt_debug:\n{}", style_fmt_debug);
        println!("style_fmt_pretty:\n{}", style_fmt_pretty);

        assert_eq!(expected_debug, style_fmt_debug);
        assert_eq!(expected_pretty_repat, style_fmt_pretty);
    }
}
