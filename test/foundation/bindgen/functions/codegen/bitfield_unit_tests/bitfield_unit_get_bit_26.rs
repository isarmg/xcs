#[test]
fn bitfield_unit_get_bit() {
    let unit = __BindgenBitfieldUnit::<[u8; 2]>::new([0b10011101, 0b00011101]);

    let mut bits = vec![];
    for i in 0..16 {
        bits.push(unit.get_bit(i));
    }

    println!();
    println!("bits = {bits:?}");
    assert_eq!(
        bits,
        &[
            // 0b10011101
            true, false, true, true, true, false, false, true,
            // 0b00011101
            true, false, true, true, true, false, false, false
        ]
    );
}
