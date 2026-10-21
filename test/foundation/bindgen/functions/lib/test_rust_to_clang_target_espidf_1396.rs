#[test]
fn test_rust_to_clang_target_espidf() {
    assert_eq!(
        rust_to_clang_target("riscv32imc-esp-espidf").as_ref(),
        "riscv32-esp-elf"
    );
    assert_eq!(
        rust_to_clang_target("xtensa-esp32-espidf").as_ref(),
        "xtensa-esp32-elf"
    );
}
