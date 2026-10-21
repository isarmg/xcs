#[cfg(test)]
mod tests {
    use serde_test::{assert_tokens, Configure, Token::*};

    bitflags! {
        #[derive(serde_lib::Serialize, serde_lib::Deserialize, Debug, PartialEq, Eq)]
        #[serde(crate = "serde_lib", transparent)]
        struct SerdeFlags: u32 {
            const A = 1;
            const B = 2;
            const C = 4;
            const D = 8;
        }
    }

    #[test]
    fn test_serde_bitflags_default() {
        assert_tokens(&SerdeFlags::empty().readable(), &[Str("")]);

        assert_tokens(&SerdeFlags::empty().compact(), &[U32(0)]);

        assert_tokens(&(SerdeFlags::A | SerdeFlags::B).readable(), &[Str("A | B")]);

        assert_tokens(&(SerdeFlags::A | SerdeFlags::B).compact(), &[U32(1 | 2)]);
    }
}
