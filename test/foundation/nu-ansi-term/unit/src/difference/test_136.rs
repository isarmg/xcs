#[cfg(test)]
mod test {
    use super::Difference::*;
    use super::*;
    use crate::style::Color::*;
    use crate::style::Style;

    fn style() -> Style {
        Style::new()
    }

    macro_rules! test {
        ($name: ident: $first: expr; $next: expr => $result: expr) => {
            #[test]
            fn $name() {
                assert_eq!($result, Difference::between(&$first, &$next));
            }
        };
    }

    test!(nothing:    Green.normal(); Green.normal()  => Empty);
    test!(uppercase:  Green.normal(); Green.bold()    => ExtraStyles(style().bold()));
    test!(lowercase:  Green.bold();   Green.normal()  => Reset);
    test!(nothing2:   Green.bold();   Green.bold()    => Empty);

    test!(color_change: Red.normal(); Blue.normal() => ExtraStyles(Blue.normal()));

    test!(addition_of_blink:          style(); style().blink()          => ExtraStyles(style().blink()));
    test!(addition_of_dimmed:         style(); style().dimmed()         => ExtraStyles(style().dimmed()));
    test!(addition_of_hidden:         style(); style().hidden()         => ExtraStyles(style().hidden()));
    test!(addition_of_reverse:        style(); style().reverse()        => ExtraStyles(style().reverse()));
    test!(addition_of_strikethrough:  style(); style().strikethrough()  => ExtraStyles(style().strikethrough()));

    test!(removal_of_strikethrough:   style().strikethrough(); style()  => Reset);
    test!(removal_of_reverse:         style().reverse();       style()  => Reset);
    test!(removal_of_hidden:          style().hidden();        style()  => Reset);
    test!(removal_of_dimmed:          style().dimmed();        style()  => Reset);
    test!(removal_of_blink:           style().blink();         style()  => Reset);
}
