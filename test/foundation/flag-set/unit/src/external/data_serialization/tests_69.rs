#[cfg(test)]
mod tests {
    use serde_test::{assert_tokens, Configure, Token::*};

    bitflags! {
        #[derive(serialization_lib::Serialize, serialization_lib::Deserialize, Debug, PartialEq, Eq)]
        #[serialization(crate = "serialization_lib", transparent)]
        struct SerializedFlags: u32 {
            const A = 1;
            const B = 2;
            const C = 4;
            const D = 8;
        }
    }

    #[test]
    fn test_serialization_bitflags_default() {
        assert_tokens(&SerializedFlags::empty().readable(), &[Str("")]);

        assert_tokens(&SerializedFlags::empty().compact(), &[U32(0)]);

        assert_tokens(&(SerializedFlags::A | SerializedFlags::B).readable(), &[Str("A | B")]);

        assert_tokens(&(SerializedFlags::A | SerializedFlags::B).compact(), &[U32(1 | 2)]);
    }
}
