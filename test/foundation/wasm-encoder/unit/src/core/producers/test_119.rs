#[cfg(test)]
mod test {
    #[test]
    fn roundtrip_example() {
        use crate::{Module, ProducersField, ProducersSection};
        use wasmparser::{KnownCustom, Parser, Payload};

        // Create a new producers section.
        let mut field = ProducersField::new();
        field.value("clang", "14.0.4");
        field.value("rustc", "1.66.1");
        let mut producers = ProducersSection::new();
        producers.field("processed-by", &field);

        // Add the producers section to a new Wasm module and get the encoded bytes.
        let mut module = Module::new();
        module.section(&producers);
        let wasm_bytes = module.finish();

        let mut parser = Parser::new(0).parse_all(&wasm_bytes);
        let payload = parser
            .next()
            .expect("parser is not empty")
            .expect("element is a payload");
        match payload {
            Payload::Version { .. } => {}
            _ => panic!(""),
        }
        let payload = parser
            .next()
            .expect("parser is not empty")
            .expect("element is a payload");
        match payload {
            Payload::CustomSection(c) => {
                assert_eq!(c.name(), "producers");
                let mut section = match c.as_known() {
                    KnownCustom::Producers(s) => s.into_iter(),
                    _ => panic!("unknown custom section"),
                };
                let field = section
                    .next()
                    .expect("section has an element")
                    .expect("element is a producers field");
                assert_eq!(field.name, "processed-by");
                let mut values = field.values.into_iter();
                let value = values
                    .next()
                    .expect("values has an element")
                    .expect("element is a producers field value");
                assert_eq!(value.name, "clang");
                assert_eq!(value.version, "14.0.4");

                let value = values
                    .next()
                    .expect("values has another element")
                    .expect("element is a producers field value");
                assert_eq!(value.name, "rustc");
                assert_eq!(value.version, "1.66.1");
            }
            _ => panic!("unexpected payload"),
        }
    }
}
