#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_address_generator() {
        let mut module = crate::Module::default();

        let mut func1 = crate::LocalFunction::new(
            Vec::new(),
            crate::FunctionBuilder::new(&mut module.types, &[], &[]),
        );

        func1.original_range = Some(Range { start: 20, end: 30 });

        let id1 = module.funcs.add_local(func1);

        let mut func2 = crate::LocalFunction::new(
            Vec::new(),
            crate::FunctionBuilder::new(&mut module.types, &[], &[]),
        );

        func2.original_range = Some(Range { start: 30, end: 50 });

        let id2 = module.funcs.add_local(func2);

        let address_converter = CodeAddressGenerator::new(&module.funcs);

        assert_eq!(
            address_converter.find_address(10, AddressSearchPreference::InclusiveFunctionEnd),
            CodeAddress::Unknown
        );
        assert_eq!(
            address_converter.find_address(20, AddressSearchPreference::ExclusiveFunctionEnd),
            CodeAddress::OffsetInFunction { id: id1, offset: 0 }
        );
        assert_eq!(
            address_converter.find_address(20, AddressSearchPreference::InclusiveFunctionEnd),
            CodeAddress::Unknown
        );
        assert_eq!(
            address_converter.find_address(25, AddressSearchPreference::ExclusiveFunctionEnd),
            CodeAddress::OffsetInFunction { id: id1, offset: 5 }
        );
        assert_eq!(
            address_converter.find_address(29, AddressSearchPreference::ExclusiveFunctionEnd),
            CodeAddress::OffsetInFunction { id: id1, offset: 9 }
        );
        assert_eq!(
            address_converter.find_address(29, AddressSearchPreference::InclusiveFunctionEnd),
            CodeAddress::OffsetInFunction { id: id1, offset: 9 }
        );
        assert_eq!(
            address_converter.find_address(30, AddressSearchPreference::InclusiveFunctionEnd),
            CodeAddress::FunctionEdge { id: id1 }
        );
        assert_eq!(
            address_converter.find_address(30, AddressSearchPreference::ExclusiveFunctionEnd),
            CodeAddress::OffsetInFunction { id: id2, offset: 0 }
        );
        assert_eq!(
            address_converter.find_address(50, AddressSearchPreference::InclusiveFunctionEnd),
            CodeAddress::FunctionEdge { id: id2 }
        );
        assert_eq!(
            address_converter.find_address(50, AddressSearchPreference::ExclusiveFunctionEnd),
            CodeAddress::Unknown
        );
    }

    #[test]
    fn test_code_address_converter() {
        let mut module = crate::Module::default();

        let func1 = crate::LocalFunction::new(
            Vec::new(),
            crate::FunctionBuilder::new(&mut module.types, &[], &[]),
        );
        let id1 = module.funcs.add_local(func1);
        let instr_id1 = InstrLocId::new(1);
        let instr_id2 = InstrLocId::new(2);

        let mut code_transform = CodeTransform::default();
        {
            code_transform
                .function_ranges
                .push((id1, Range { start: 50, end: 80 }));
            code_transform.instruction_map.push((instr_id1, 60));
            code_transform.instruction_map.push((instr_id2, 65));
        }

        let converter = CodeAddressConverter::new(&code_transform);

        assert_eq!(
            converter.find_address(CodeAddress::OffsetInFunction { id: id1, offset: 5 }),
            Some(55)
        );
        assert_eq!(
            converter.find_address(CodeAddress::InstrInFunction {
                instr_id: instr_id1
            }),
            Some(60)
        );
        assert_eq!(
            converter.find_address(CodeAddress::InstrEdge {
                instr_id: instr_id2
            }),
            Some(64)
        );
        assert_eq!(
            converter.find_address(CodeAddress::InstrInFunction {
                instr_id: instr_id2
            }),
            Some(65)
        );
        assert_eq!(
            converter.find_address(CodeAddress::FunctionEdge { id: id1 }),
            Some(80)
        );
        assert_eq!(converter.find_address(CodeAddress::Unknown), None);
    }
}
