#[cfg(test)]
#[cfg(all(not(feature = "gnu_legacy"), feature = "std"))]
mod test {

    use crate::style::Color::*;
    use crate::style::Style;
    use crate::Color;
    use std::default::Default;

    test!(plain:                 Style::default();                  "text/plain" => "text/plain");
    test!(red:                   Red;                               "hi" => "\x1B[31mhi\x1B[0m");
    test!(black:                 Black.normal();                    "hi" => "\x1B[30mhi\x1B[0m");
    test!(yellow_bold:           Yellow.bold();                     "hi" => "\x1B[1;33mhi\x1B[0m");
    test!(yellow_bold_2:         Yellow.normal().bold();            "hi" => "\x1B[1;33mhi\x1B[0m");
    test!(blue_underline:        Blue.underline();                  "hi" => "\x1B[4;34mhi\x1B[0m");
    test!(green_bold_ul:         Green.bold().underline();          "hi" => "\x1B[1;4;32mhi\x1B[0m");
    test!(green_bold_ul_2:       Green.underline().bold();          "hi" => "\x1B[1;4;32mhi\x1B[0m");
    test!(purple_on_white:       Purple.on(White);                  "hi" => "\x1B[47;35mhi\x1B[0m");
    test!(purple_on_white_2:     Purple.normal().on(White);         "hi" => "\x1B[47;35mhi\x1B[0m");
    test!(yellow_on_blue:        Style::new().on(Blue).fg(Yellow);  "hi" => "\x1B[44;33mhi\x1B[0m");
    test!(magenta_on_white:      Magenta.on(White);                  "hi" => "\x1B[47;35mhi\x1B[0m");
    test!(magenta_on_white_2:    Magenta.normal().on(White);         "hi" => "\x1B[47;35mhi\x1B[0m");
    test!(yellow_on_blue_2:      Cyan.on(Blue).fg(Yellow);          "hi" => "\x1B[44;33mhi\x1B[0m");
    test!(yellow_on_blue_reset:  Cyan.on(Blue).reset_before_style().fg(Yellow); "hi" => "\x1B[0m\x1B[44;33mhi\x1B[0m");
    test!(yellow_on_blue_reset_2: Cyan.on(Blue).fg(Yellow).reset_before_style(); "hi" => "\x1B[0m\x1B[44;33mhi\x1B[0m");
    test!(cyan_bold_on_white:    Cyan.bold().on(White);             "hi" => "\x1B[1;47;36mhi\x1B[0m");
    test!(cyan_ul_on_white:      Cyan.underline().on(White);        "hi" => "\x1B[4;47;36mhi\x1B[0m");
    test!(cyan_bold_ul_on_white: Cyan.bold().underline().on(White); "hi" => "\x1B[1;4;47;36mhi\x1B[0m");
    test!(cyan_ul_bold_on_white: Cyan.underline().bold().on(White); "hi" => "\x1B[1;4;47;36mhi\x1B[0m");
    test!(fixed:                 Fixed(100);                        "hi" => "\x1B[38;5;100mhi\x1B[0m");
    test!(fixed_on_purple:       Fixed(100).on(Purple);             "hi" => "\x1B[45;38;5;100mhi\x1B[0m");
    test!(fixed_on_fixed:        Fixed(100).on(Fixed(200));         "hi" => "\x1B[48;5;200;38;5;100mhi\x1B[0m");
    test!(rgb:                   Rgb(70,130,180);                   "hi" => "\x1B[38;2;70;130;180mhi\x1B[0m");
    test!(rgb_on_blue:           Rgb(70,130,180).on(Blue);          "hi" => "\x1B[44;38;2;70;130;180mhi\x1B[0m");
    test!(blue_on_rgb:           Blue.on(Rgb(70,130,180));          "hi" => "\x1B[48;2;70;130;180;34mhi\x1B[0m");
    test!(rgb_on_rgb:            Rgb(70,130,180).on(Rgb(5,10,15));  "hi" => "\x1B[48;2;5;10;15;38;2;70;130;180mhi\x1B[0m");
    test!(bold:                  Style::new().bold();               "hi" => "\x1B[1mhi\x1B[0m");
    test!(bold_with_reset:       Style::new().reset_before_style().bold(); "hi" => "\x1B[0m\x1B[1mhi\x1B[0m");
    test!(bold_with_reset_2:     Style::new().bold().reset_before_style(); "hi" => "\x1B[0m\x1B[1mhi\x1B[0m");
    test!(underline:             Style::new().underline();          "hi" => "\x1B[4mhi\x1B[0m");
    test!(bunderline:            Style::new().bold().underline();   "hi" => "\x1B[1;4mhi\x1B[0m");
    test!(dimmed:                Style::new().dimmed();             "hi" => "\x1B[2mhi\x1B[0m");
    test!(italic:                Style::new().italic();             "hi" => "\x1B[3mhi\x1B[0m");
    test!(blink:                 Style::new().blink();              "hi" => "\x1B[5mhi\x1B[0m");
    test!(reverse:               Style::new().reverse();            "hi" => "\x1B[7mhi\x1B[0m");
    test!(hidden:                Style::new().hidden();             "hi" => "\x1B[8mhi\x1B[0m");
    test!(stricken:              Style::new().strikethrough();      "hi" => "\x1B[9mhi\x1B[0m");
    test!(lr_on_lr:              LightRed.on(LightRed);             "hi" => "\x1B[101;91mhi\x1B[0m");

    #[test]
    fn test_infix() {
        assert_eq!(
            Style::new().dimmed().infix(Style::new()).to_string(),
            "\x1B[0m"
        );
        assert_eq!(
            White.dimmed().infix(White.normal()).to_string(),
            "\x1B[0m\x1B[37m"
        );
        assert_eq!(White.normal().infix(White.bold()).to_string(), "\x1B[1m");
        assert_eq!(White.normal().infix(Blue.normal()).to_string(), "\x1B[34m");
        assert_eq!(Blue.bold().infix(Blue.bold()).to_string(), "");
    }

    #[test]
    fn test_write_prefix_no_gnu_compat_order() {
        let style = Style {
            foreground: Some(Color::Red),
            background: Some(Color::Blue),
            ..Default::default()
        };
        assert_eq!(
            style.paint("file").to_string(),
            "\u{1b}[44;31mfile\u{1b}[0m".to_string()
        );
    }
}
