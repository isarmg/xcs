#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_state_action() {
        match unpack(0xee) {
            (State::SosPmApcString, Action::Unhook) => (),
            _ => panic!("unpack failed"),
        }

        match unpack(0x0f) {
            (State::Utf8, Action::Nop) => (),
            _ => panic!("unpack failed"),
        }

        match unpack(0xff) {
            (State::Utf8, Action::BeginUtf8) => (),
            _ => panic!("unpack failed"),
        }
    }

    #[test]
    fn pack_state_action() {
        match unpack(0xee) {
            (State::SosPmApcString, Action::Unhook) => (),
            _ => panic!("unpack failed"),
        }

        match unpack(0x0f) {
            (State::Utf8, Action::Nop) => (),
            _ => panic!("unpack failed"),
        }

        match unpack(0xff) {
            (State::Utf8, Action::BeginUtf8) => (),
            _ => panic!("unpack failed"),
        }
    }
}
