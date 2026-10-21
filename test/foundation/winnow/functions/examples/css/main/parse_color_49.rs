#[test]
fn parse_color() {
    assert_eq!(
        hex_color.parse_peek("#2F14DF"),
        Ok((
            "",
            parser::Color {
                red: 47,
                green: 20,
                blue: 223,
            }
        ))
    );
}
