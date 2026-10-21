#[test]
fn bitfield_unit_set_bit() {
    let mut unit =
        __BindgenBitfieldUnit::<[u8; 2]>::new([0b00000000, 0b00000000]);

    for i in 0..16 {
        if i % 3 == 0 {
            unit.set_bit(i, true);
        }
    }

    for i in 0..16 {
        assert_eq!(unit.get_bit(i), i % 3 == 0);
    }

    let mut unit =
        __BindgenBitfieldUnit::<[u8; 2]>::new([0b11111111, 0b11111111]);

    for i in 0..16 {
        if i % 3 == 0 {
            unit.set_bit(i, false);
        }
    }

    for i in 0..16 {
        assert_eq!(unit.get_bit(i), i % 3 != 0);
    }
}
