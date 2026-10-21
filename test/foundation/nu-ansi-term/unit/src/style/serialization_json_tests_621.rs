#[cfg(test)]
#[cfg(feature = "derive_serialization_style")]
mod serialization_json_tests {
    use super::{Color, Style};

    #[test]
    fn color_serialization() {
        let colors = &[
            Color::Red,
            Color::Blue,
            Color::Rgb(123, 123, 123),
            Color::Fixed(255),
        ];

        assert_eq!(
            json_codec::to_string(&colors).unwrap(),
            "[\"Red\",\"Blue\",{\"Rgb\":[123,123,123]},{\"Fixed\":255}]"
        );
    }

    #[test]
    fn color_deserialization() {
        let colors = [
            Color::Red,
            Color::Blue,
            Color::Rgb(123, 123, 123),
            Color::Fixed(255),
        ];

        for color in colors {
            let serialized = json_codec::to_string(&color).unwrap();
            let deserialized: Color = json_codec::from_str(&serialized).unwrap();

            assert_eq!(color, deserialized);
        }
    }

    #[test]
    fn style_serialization() {
        let style = Style::default();

        assert_eq!(json_codec::to_string(&style).unwrap(), "{\"foreground\":null,\"background\":null,\"is_bold\":false,\"is_dimmed\":false,\"is_italic\":false,\"is_underline\":false,\"is_blink\":false,\"is_reverse\":false,\"is_hidden\":false,\"is_strikethrough\":false,\"prefix_with_reset\":false}".to_string());
    }
}
