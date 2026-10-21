#[cfg(test)]
mod tests {
    use super::*;
    use crate::Module;
    use wasmparser::{KnownCustom, Parser, Payload};

    // Create new core dump section and test whether it is properly encoded and
    // parsed back out by wasmparser
    #[test]
    fn test_roundtrip_core() {
        let core = CoreDumpSection::new("test.wasm");
        let mut module = Module::new();
        module.section(&core);

        let wasm_bytes = module.finish();

        let mut parser = Parser::new(0).parse_all(&wasm_bytes);
        match parser.next() {
            Some(Ok(Payload::Version { .. })) => {}
            _ => panic!(""),
        }

        let payload = parser
            .next()
            .expect("parser is not empty")
            .expect("element is a payload");
        match payload {
            Payload::CustomSection(section) => {
                assert_eq!(section.name(), "core");
                let core = match section.as_known() {
                    KnownCustom::CoreDump(s) => s,
                    _ => panic!("not coredump"),
                };
                assert_eq!(core.name, "test.wasm");
            }
            _ => panic!("unexpected payload"),
        }
    }

    #[test]
    fn test_roundtrip_coremodules() {
        let mut coremodules = CoreDumpModulesSection::new();
        coremodules.module("test_module");

        let mut module = crate::Module::new();
        module.section(&coremodules);

        let wasm_bytes = module.finish();

        let mut parser = Parser::new(0).parse_all(&wasm_bytes);
        match parser.next() {
            Some(Ok(Payload::Version { .. })) => {}
            _ => panic!(""),
        }

        let payload = parser
            .next()
            .expect("parser is not empty")
            .expect("element is a payload");
        match payload {
            Payload::CustomSection(section) => {
                assert_eq!(section.name(), "coremodules");
                let modules = match section.as_known() {
                    KnownCustom::CoreDumpModules(s) => s,
                    _ => panic!("not coremodules"),
                };
                assert_eq!(modules.modules[0], "test_module");
            }
            _ => panic!("unexpected payload"),
        }
    }

    #[test]
    fn test_roundtrip_coreinstances() {
        let mut coreinstances = CoreDumpInstancesSection::new();
        let module_index = 0;
        let memories = vec![42];
        let globals = vec![17];
        coreinstances.instance(module_index, memories, globals);

        let mut module = Module::new();
        module.section(&coreinstances);
        let wasm_bytes = module.finish();

        let mut parser = Parser::new(0).parse_all(&wasm_bytes);
        match parser.next() {
            Some(Ok(Payload::Version { .. })) => {}
            _ => panic!(""),
        }

        let payload = parser
            .next()
            .expect("parser is not empty")
            .expect("element is a payload");
        match payload {
            Payload::CustomSection(section) => {
                assert_eq!(section.name(), "coreinstances");
                let coreinstances = match section.as_known() {
                    KnownCustom::CoreDumpInstances(s) => s,
                    _ => panic!("not coreinstances"),
                };
                assert_eq!(coreinstances.instances.len(), 1);
                let instance = coreinstances
                    .instances
                    .first()
                    .expect("instance is encoded");
                assert_eq!(instance.module_index, 0);
                assert_eq!(instance.memories.len(), 1);
                assert_eq!(instance.globals.len(), 1);
            }
            _ => panic!("unexpected payload"),
        }
    }

    // Create new corestack section and test whether it is properly encoded and
    // parsed back out by wasmparser
    #[test]
    fn test_roundtrip_corestack() {
        let mut corestack = CoreDumpStackSection::new("main");
        corestack.frame(
            0,
            12,
            0,
            vec![CoreDumpValue::I32(10)],
            vec![CoreDumpValue::I32(42)],
        );
        let mut module = Module::new();
        module.section(&corestack);
        let wasm_bytes = module.finish();

        let mut parser = Parser::new(0).parse_all(&wasm_bytes);
        match parser.next() {
            Some(Ok(Payload::Version { .. })) => {}
            _ => panic!(""),
        }

        let payload = parser
            .next()
            .expect("parser is not empty")
            .expect("element is a payload");
        match payload {
            Payload::CustomSection(section) => {
                assert_eq!(section.name(), "corestack");
                let corestack = match section.as_known() {
                    KnownCustom::CoreDumpStack(s) => s,
                    _ => panic!("not a corestack section"),
                };
                assert_eq!(corestack.name, "main");
                assert_eq!(corestack.frames.len(), 1);
                let frame = corestack
                    .frames
                    .first()
                    .expect("frame is encoded in corestack");
                assert_eq!(frame.instanceidx, 0);
                assert_eq!(frame.funcidx, 12);
                assert_eq!(frame.codeoffset, 0);
                assert_eq!(frame.locals.len(), 1);
                match frame.locals.first().expect("frame contains a local") {
                    &wasmparser::CoreDumpValue::I32(val) => assert_eq!(val, 10),
                    _ => panic!("unexpected local value"),
                }
                assert_eq!(frame.stack.len(), 1);
                match frame.stack.first().expect("stack contains a value") {
                    &wasmparser::CoreDumpValue::I32(val) => assert_eq!(val, 42),
                    _ => panic!("unexpected stack value"),
                }
            }
            _ => panic!("unexpected payload"),
        }
    }

    #[test]
    fn test_encode_coredump_section() {
        let core = CoreDumpSection::new("test");

        let mut encoded = vec![];
        core.encode(&mut encoded);

        #[rustfmt::skip]
        assert_eq!(encoded, vec![
            // section length
            11,
            // name length
            4,
            // section name (core)
            b'c',b'o',b'r',b'e',
            // process-info (0, data length, data)
            0, 4, b't', b'e', b's', b't',
        ]);
    }

    #[test]
    fn test_encode_coremodules_section() {
        let mut modules = CoreDumpModulesSection::new();
        modules.module("mod1");
        modules.module("mod2");

        let mut encoded = vec![];
        modules.encode(&mut encoded);

        #[rustfmt::skip]
        assert_eq!(encoded, vec![
            // section length
            25,
            // name length
            11,
            // section name (coremodules)
            b'c',b'o',b'r',b'e',b'm',b'o',b'd',b'u',b'l',b'e',b's',
            // module count
            2,
            // 0x0, name-length, module name (mod1)
            0x0, 4, b'm',b'o',b'd',b'1',
            // 0x0, name-length, module name (mod2)
            0x0, 4, b'm',b'o',b'd',b'2'
        ]);
    }

    #[test]
    fn test_encode_coreinstances_section() {
        let mut instances = CoreDumpInstancesSection::new();
        instances.instance(0, vec![42], vec![17]);

        let mut encoded = vec![];
        instances.encode(&mut encoded);

        #[rustfmt::skip]
        assert_eq!(encoded, vec![
            // section length
            21,
            // name length
            13,
            // section name (coreinstances)
            b'c',b'o',b'r',b'e',b'i',b'n',b's',b't',b'a',b'n',b'c',b'e',b's',
            // instance count
            1,
            // 0x0, module_idx
            0x0, 0,
            // memories count, memories
            1, 42,
            // globals count, globals
            1, 17
        ]);
    }

    #[test]
    fn test_encode_corestack_section() {
        let mut thread = CoreDumpStackSection::new("main");
        thread.frame(
            0,
            42,
            51,
            vec![CoreDumpValue::I32(1)],
            vec![CoreDumpValue::I32(2)],
        );

        let mut encoded = vec![];
        thread.encode(&mut encoded);

        #[rustfmt::skip]
        assert_eq!(
            encoded,
            vec![
                // section length
                27,
                // length of name.
                9,
                // section name (corestack)
                b'c',b'o',b'r',b'e',b's',b't',b'a',b'c',b'k',
                // 0x0, thread name length
                0, 4,
                // thread name (main)
                b'm',b'a',b'i',b'n',
                // frame count
                1,
                // 0x0, instanceidx, funcidx, codeoffset
                0, 0, 42, 51,
                // local count
                1,
                // local value type
                0x7F,
                // local value
                1,
                // stack count
                1,
                // stack value type
                0x7F,
                // stack value
                2

            ]
        );
    }
}
