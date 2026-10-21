#[cfg(test)]
#[cfg(feature = "read")]
mod tests {
    use super::*;
    use crate::arch::X86_64;
    use crate::read;
    use crate::write::EndianVec;
    use crate::{LittleEndian, Vendor};

    #[test]
    fn test_frame_table() {
        for &version in &[1, 3, 4] {
            for &address_size in &[4, 8] {
                for &format in &[Format::Dwarf32, Format::Dwarf64] {
                    let encoding = Encoding {
                        format,
                        version,
                        address_size,
                    };
                    let mut frames = FrameTable::default();

                    let cie1 = CommonInformationEntry::new(encoding, 1, 8, X86_64::RA);
                    let cie1_id = frames.add_cie(cie1.clone());
                    assert_eq!(cie1_id, frames.add_cie(cie1.clone()));

                    let mut cie2 = CommonInformationEntry::new(encoding, 1, 8, X86_64::RA);
                    cie2.lsda_encoding = Some(constants::DW_EH_PE_absptr);
                    cie2.personality =
                        Some((constants::DW_EH_PE_absptr, Address::Constant(0x1234)));
                    cie2.signal_trampoline = true;
                    let cie2_id = frames.add_cie(cie2.clone());
                    assert_ne!(cie1_id, cie2_id);
                    assert_eq!(cie2_id, frames.add_cie(cie2.clone()));

                    let fde1 = FrameDescriptionEntry::new(Address::Constant(0x1000), 0x10);
                    frames.add_fde(cie1_id, fde1.clone());

                    let fde2 = FrameDescriptionEntry::new(Address::Constant(0x2000), 0x20);
                    frames.add_fde(cie1_id, fde2.clone());

                    let mut fde3 = FrameDescriptionEntry::new(Address::Constant(0x3000), 0x30);
                    fde3.lsda = Some(Address::Constant(0x3300));
                    frames.add_fde(cie2_id, fde3.clone());

                    let mut fde4 = FrameDescriptionEntry::new(Address::Constant(0x4000), 0x40);
                    fde4.lsda = Some(Address::Constant(0x4400));
                    frames.add_fde(cie2_id, fde4.clone());

                    let mut cie3 = CommonInformationEntry::new(encoding, 1, 8, X86_64::RA);
                    cie3.fde_address_encoding =
                        constants::DW_EH_PE_pcrel | constants::DW_EH_PE_sdata4;
                    cie3.lsda_encoding =
                        Some(constants::DW_EH_PE_pcrel | constants::DW_EH_PE_sdata4);
                    cie3.personality = Some((
                        constants::DW_EH_PE_pcrel | constants::DW_EH_PE_sdata4,
                        Address::Constant(0x1235),
                    ));
                    cie3.signal_trampoline = true;
                    let cie3_id = frames.add_cie(cie3.clone());
                    assert_ne!(cie2_id, cie3_id);
                    assert_eq!(cie3_id, frames.add_cie(cie3.clone()));

                    let mut fde5 = FrameDescriptionEntry::new(Address::Constant(0x5000), 0x50);
                    fde5.lsda = Some(Address::Constant(0x5500));
                    frames.add_fde(cie3_id, fde5.clone());

                    // Test writing `.debug_frame`.
                    let mut debug_frame = DebugFrame::from(EndianVec::new(LittleEndian));
                    frames.write_debug_frame(&mut debug_frame).unwrap();

                    let mut read_debug_frame =
                        read::DebugFrame::new(debug_frame.slice(), LittleEndian);
                    read_debug_frame.set_address_size(address_size);
                    let convert_frames = FrameTable::from(&read_debug_frame, &|address| {
                        Some(Address::Constant(address))
                    })
                    .unwrap();
                    assert_eq!(frames.cies, convert_frames.cies);
                    assert_eq!(frames.fdes.len(), convert_frames.fdes.len());
                    for (a, b) in frames.fdes.iter().zip(convert_frames.fdes.iter()) {
                        assert_eq!(a.1, b.1);
                    }

                    if version == 1 {
                        // Test writing `.eh_frame`.
                        let mut eh_frame = EhFrame::from(EndianVec::new(LittleEndian));
                        frames.write_eh_frame(&mut eh_frame).unwrap();

                        let mut read_eh_frame = read::EhFrame::new(eh_frame.slice(), LittleEndian);
                        read_eh_frame.set_address_size(address_size);
                        let convert_frames = FrameTable::from(&read_eh_frame, &|address| {
                            Some(Address::Constant(address))
                        })
                        .unwrap();
                        assert_eq!(frames.cies, convert_frames.cies);
                        assert_eq!(frames.fdes.len(), convert_frames.fdes.len());
                        for (a, b) in frames.fdes.iter().zip(convert_frames.fdes.iter()) {
                            assert_eq!(a.1, b.1);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_frame_instruction() {
        let mut expression = Expression::new();
        expression.op_constu(0);

        let cie_instructions = [
            CallFrameInstruction::Cfa(X86_64::RSP, 8),
            CallFrameInstruction::Offset(X86_64::RA, -8),
        ];

        let fde_instructions = [
            (0, CallFrameInstruction::Cfa(X86_64::RSP, 0)),
            (0, CallFrameInstruction::Cfa(X86_64::RSP, -8)),
            (2, CallFrameInstruction::CfaRegister(X86_64::RBP)),
            (4, CallFrameInstruction::CfaOffset(8)),
            (4, CallFrameInstruction::CfaOffset(0)),
            (4, CallFrameInstruction::CfaOffset(-8)),
            (6, CallFrameInstruction::CfaExpression(expression.clone())),
            (8, CallFrameInstruction::Restore(Register(1))),
            (8, CallFrameInstruction::Restore(Register(101))),
            (10, CallFrameInstruction::Undefined(Register(2))),
            (12, CallFrameInstruction::SameValue(Register(3))),
            (14, CallFrameInstruction::Offset(Register(4), 16)),
            (14, CallFrameInstruction::Offset(Register(104), 16)),
            (16, CallFrameInstruction::ValOffset(Register(5), -24)),
            (16, CallFrameInstruction::ValOffset(Register(5), 24)),
            (18, CallFrameInstruction::Register(Register(6), Register(7))),
            (
                20,
                CallFrameInstruction::Expression(Register(8), expression.clone()),
            ),
            (
                22,
                CallFrameInstruction::ValExpression(Register(9), expression.clone()),
            ),
            (24 + 0x80, CallFrameInstruction::RememberState),
            (26 + 0x280, CallFrameInstruction::RestoreState),
            (28 + 0x20280, CallFrameInstruction::ArgsSize(23)),
        ];

        let fde_instructions_aarch64 = [(0, CallFrameInstruction::NegateRaState)];

        for &version in &[1, 3, 4] {
            for &address_size in &[4, 8] {
                for &vendor in &[Vendor::Default, Vendor::AArch64] {
                    for &format in &[Format::Dwarf32, Format::Dwarf64] {
                        let encoding = Encoding {
                            format,
                            version,
                            address_size,
                        };
                        let mut frames = FrameTable::default();

                        let mut cie = CommonInformationEntry::new(encoding, 2, 8, X86_64::RA);
                        for i in &cie_instructions {
                            cie.add_instruction(i.clone());
                        }
                        let cie_id = frames.add_cie(cie);

                        let mut fde = FrameDescriptionEntry::new(Address::Constant(0x1000), 0x10);
                        for (o, i) in &fde_instructions {
                            fde.add_instruction(*o, i.clone());
                        }
                        frames.add_fde(cie_id, fde);

                        if vendor == Vendor::AArch64 {
                            let mut fde =
                                FrameDescriptionEntry::new(Address::Constant(0x2000), 0x10);
                            for (o, i) in &fde_instructions_aarch64 {
                                fde.add_instruction(*o, i.clone());
                            }
                            frames.add_fde(cie_id, fde);
                        }

                        let mut debug_frame = DebugFrame::from(EndianVec::new(LittleEndian));
                        frames.write_debug_frame(&mut debug_frame).unwrap();

                        let mut read_debug_frame =
                            read::DebugFrame::new(debug_frame.slice(), LittleEndian);
                        read_debug_frame.set_address_size(address_size);
                        read_debug_frame.set_vendor(vendor);
                        let frames = FrameTable::from(&read_debug_frame, &|address| {
                            Some(Address::Constant(address))
                        })
                        .unwrap();

                        assert_eq!(
                            &frames.cies.get_index(0).unwrap().instructions,
                            &cie_instructions
                        );
                        assert_eq!(&frames.fdes[0].1.instructions, &fde_instructions);
                        if vendor == Vendor::AArch64 {
                            assert_eq!(&frames.fdes[1].1.instructions, &fde_instructions_aarch64);
                        }
                    }
                }
            }
        }
    }
}
