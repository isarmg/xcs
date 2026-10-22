use super::{Parser, Receiver};

#[derive(Debug, PartialEq)]
enum Event {
    Codepoint(char),
    Invalid,
}

#[derive(Default)]
struct Events(Vec<Event>);

impl Receiver for Events {
    fn codepoint(&mut self, c: char) {
        self.0.push(Event::Codepoint(c));
    }

    fn invalid_sequence(&mut self) {
        self.0.push(Event::Invalid);
    }
}

fn parse_chunks(bytes: &[u8], size: usize) -> Events {
    let mut parser = Parser::new();
    let mut events = Events::default();
    for chunk in bytes.chunks(size) {
        for &byte in chunk {
            parser.advance(&mut events, byte);
        }
    }
    events
}

#[test]
fn utf8_valid_boundary_scalars_survive_every_chunk_size() {
    let text = "\0\u{7f}\u{80}\u{7ff}\u{800}\u{d7ff}\u{e000}\u{ffff}\u{10000}\u{10ffff}hé世界🦀";
    let expected: Vec<_> = text.chars().map(Event::Codepoint).collect();
    for size in 1..=text.len() {
        assert_eq!(parse_chunks(text.as_bytes(), size).0, expected);
    }
}

#[test]
fn utf8_invalid_sequences_preserve_exact_callback_contract() {
    // The rejecting byte is consumed, not replayed as a new leading byte.
    let cases: &[(&[u8], usize)] = &[
        (&[0x80], 1),
        (&[0xc0, 0xaf], 2),
        (&[0xc1, 0xbf], 2),
        (&[0xe0, 0x80, 0x80], 2),
        (&[0xed, 0xa0, 0x80], 2),
        (&[0xf0, 0x80, 0x80, 0x80], 3),
        (&[0xf4, 0x90, 0x80, 0x80], 3),
        (&[0xf5, 0x80, 0x80, 0x80], 4),
        (&[0xff], 1),
        (&[0xe2, b'x'], 1),
        (&[0xc2, 0xc2], 1),
    ];
    for &(bytes, invalid) in cases {
        let mut bytes = bytes.to_vec();
        bytes.extend_from_slice("Aé".as_bytes());
        let mut expected: Vec<_> = (0..invalid).map(|_| Event::Invalid).collect();
        expected.extend([Event::Codepoint('A'), Event::Codepoint('é')]);
        for size in 1..=bytes.len() {
            assert_eq!(
                parse_chunks(&bytes, size).0,
                expected,
                "{bytes:?}, chunk {size}"
            );
        }
    }
}

#[test]
fn utf8_partial_scalar_keeps_state_without_spurious_events() {
    let mut parser = Parser::new();
    let mut events = Events::default();
    for &byte in &[0xf0, 0x9f, 0xa6] {
        parser.advance(&mut events, byte);
        assert!(events.0.is_empty());
    }
    let mut cloned = parser.clone();
    parser.advance(&mut events, 0x80);
    assert_eq!(events.0, [Event::Codepoint('🦀')]);
    let mut alternate = Events::default();
    cloned.advance(&mut alternate, 0x81);
    assert_eq!(alternate.0, [Event::Codepoint('🦁')]);
    assert_eq!(parser, Parser::new());
    assert_eq!(cloned, Parser::default());
}
