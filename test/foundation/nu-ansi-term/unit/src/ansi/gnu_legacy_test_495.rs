#[cfg(test)]
#[cfg(feature = "gnu_legacy")]
mod gnu_legacy_test {
    use crate::style::Color::*;
    use crate::style::Style;
    use crate::Color;
    use std::default::Default;

    test!(plain:                 Style::default();                  "text/plain" => "text/plain");
    test!(red:                   Red;                               "hi" => "\x1B[31mhi\x1B[0m");
    test!(black:                 Black.normal();                    "hi" => "\x1B[30mhi\x1B[0m");
    test!(yellow_bold:           Yellow.bold();                     "hi" => "\x1B[01;33mhi\x1B[0m");
    test!(yellow_bold_2:         Yellow.normal().bold();            "hi" => "\x1B[01;33mhi\x1B[0m");
    test!(blue_underline:        Blue.underline();                  "hi" => "\x1B[04;34mhi\x1B[0m");
    test!(green_bold_ul:         Green.bold().underline();          "hi" => "\x1B[01;04;32mhi\x1B[0m");
    test!(green_bold_ul_2:       Green.underline().bold();          "hi" => "\x1B[01;04;32mhi\x1B[0m");
    test!(purple_on_white:       Purple.on(White);                  "hi" => "\x1B[35;47mhi\x1B[0m");
    test!(purple_on_white_2:     Purple.normal().on(White);         "hi" => "\x1B[35;47mhi\x1B[0m");
    test!(yellow_on_blue:        Style::new().on(Blue).fg(Yellow);  "hi" => "\x1B[33;44mhi\x1B[0m");
    test!(yellow_on_blue_reset_2: Cyan.on(Blue).fg(Yellow).reset_before_style(); "hi" => "\x1B[0m\x1B[33;44mhi\x1B[0m");
    test!(magenta_on_white:      Magenta.on(White);                  "hi" => "\x1B[35;47mhi\x1B[0m");
    test!(magenta_on_white_2:    Magenta.normal().on(White);         "hi" => "\x1B[35;47mhi\x1B[0m");
    test!(yellow_on_blue_2:      Cyan.on(Blue).fg(Yellow);          "hi" => "\x1B[33;44mhi\x1B[0m");
    test!(cyan_bold_on_white:    Cyan.bold().on(White);             "hi" => "\x1B[01;36;47mhi\x1B[0m");
    test!(cyan_ul_on_white:      Cyan.underline().on(White);        "hi" => "\x1B[04;36;47mhi\x1B[0m");
    test!(cyan_bold_ul_on_white: Cyan.bold().underline().on(White); "hi" => "\x1B[01;04;36;47mhi\x1B[0m");
    test!(cyan_ul_bold_on_white: Cyan.underline().bold().on(White); "hi" => "\x1B[01;04;36;47mhi\x1B[0m");
    test!(fixed:                 Fixed(100);                        "hi" => "\x1B[38;5;100mhi\x1B[0m");
    test!(fixed_on_purple:       Fixed(100).on(Purple);             "hi" => "\x1B[38;5;100;45mhi\x1B[0m");
    test!(fixed_on_fixed:        Fixed(100).on(Fixed(200));         "hi" => "\x1B[38;5;100;48;5;200mhi\x1B[0m");
    test!(rgb:                   Rgb(70,130,180);                   "hi" => "\x1B[38;2;70;130;180mhi\x1B[0m");
    test!(rgb_on_blue:           Rgb(70,130,180).on(Blue);          "hi" => "\x1B[38;2;70;130;180;44mhi\x1B[0m");
    test!(blue_on_rgb:           Blue.on(Rgb(70,130,180));          "hi" => "\x1B[34;48;2;70;130;180mhi\x1B[0m");
    test!(rgb_on_rgb:            Rgb(70,130,180).on(Rgb(5,10,15));  "hi" => "\x1B[38;2;70;130;180;48;2;5;10;15mhi\x1B[0m");
    test!(bold:                  Style::new().bold();               "hi" => "\x1B[01mhi\x1B[0m");
    test!(bold_with_reset:       Style::new().reset_before_style().bold(); "hi" => "\x1B[0m\x1B[01mhi\x1B[0m");
    test!(bold_with_reset_2:     Style::new().bold().reset_before_style(); "hi" => "\x1B[0m\x1B[01mhi\x1B[0m");
    test!(underline:             Style::new().underline();          "hi" => "\x1B[04mhi\x1B[0m");
    test!(bunderline:            Style::new().bold().underline();   "hi" => "\x1B[01;04mhi\x1B[0m");
    test!(dimmed:                Style::new().dimmed();             "hi" => "\x1B[02mhi\x1B[0m");
    test!(italic:                Style::new().italic();             "hi" => "\x1B[03mhi\x1B[0m");
    test!(blink:                 Style::new().blink();              "hi" => "\x1B[05mhi\x1B[0m");
    test!(reverse:               Style::new().reverse();            "hi" => "\x1B[07mhi\x1B[0m");
    test!(hidden:                Style::new().hidden();             "hi" => "\x1B[08mhi\x1B[0m");
    test!(stricken:              Style::new().strikethrough();      "hi" => "\x1B[09mhi\x1B[0m");
    test!(lr_on_lr:              LightRed.on(LightRed);             "hi" => "\x1B[91;101mhi\x1B[0m");

    #[test]
    fn test_write_prefix_gnu_compat_order() {
        let style = Style {
            foreground: Some(Color::Red),
            background: Some(Color::Blue),
            ..Default::default()
        };
        assert_eq!(
            style.paint("file").to_string(),
            "\u{1b}[31;44mfile\u{1b}[0m".to_string()
        );
    }
}
