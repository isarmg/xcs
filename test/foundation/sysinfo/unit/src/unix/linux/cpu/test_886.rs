#[cfg(test)]
mod test {
    use super::get_vendor_id_and_brand_inner;

    // The iterator was skipping the `is_new_processor` check because we already moved past
    // the line where `processor]\t` is located.
    //
    // Regression test for <https://github.com/GuillaumeGomez/sysinfo/issues/1527>.
    #[test]
    fn test_cpu_retrieval() {
        const DATA: &str = r#"
processor		: 1
cpu model		: Loongson-3 V0.4  FPU V0.1
model name		: Loongson-3A R4 (Loongson-3B4000) @ 1800MHz
CPU MHz			: 1800.00
core			: 1

processor		: 2
cpu model		: Loongson-3 V0.4  FPU V0.1
model name		: Loongson-3A R4 (Loongson-3B4000) @ 1800MHz
CPU MHz			: 1800.00
package			: 0
core			: 2

processor		: 3
cpu model		: Loongson-3 V0.4  FPU V0.1
model name		: Loongson-3A R4 (Loongson-3B4000) @ 1800MHz
CPU MHz			: 1800.00
core			: 3

processor		: 4
cpu model		: Loongson-3 V0.4  FPU V0.1
model name		: Loongson-3A R4 (Loongson-3B4000) @ 1800MHz
CPU MHz			: 1800.00
core			: 0

processor		: 5
cpu model		: Loongson-3 V0.4  FPU V0.1
model name		: Loongson-3A R4 (Loongson-3B4000) @ 1800MHz
CPU MHz			: 1800.00
core			: 1

processor		: 6
cpu model		: Loongson-3 V0.4  FPU V0.1
model name		: Loongson-3A R4 (Loongson-3B4000) @ 1800MHz
CPU MHz			: 1800.00
core			: 2

processor		: 7
cpu model		: Loongson-3 V0.4  FPU V0.1
model name		: Loongson-3A R4 (Loongson-3B4000) @ 1800MHz
CPU MHz			: 1800.00
core			: 3"#;

        let cpus = get_vendor_id_and_brand_inner(DATA);
        assert_eq!(cpus.len(), 7);
    }

    #[test]
    fn test_cpu_brand_armv7() {
        const DATA: &str = r#"
Processor       : ARMv7 Processor rev 5 (v7l)
processor       : 0
BogoMIPS        : 12.80

processor       : 1
BogoMIPS        : 12.80

processor       : 2
BogoMIPS        : 12.80

processor       : 3
BogoMIPS        : 12.80

processor       : 4
BogoMIPS        : 38.40

processor       : 5
BogoMIPS        : 38.40

processor       : 6
BogoMIPS        : 38.40

processor       : 7
BogoMIPS        : 38.40

Features        : swp half thumb fastmult vfp edsp neon vfpv3 tls vfpv4 idiva idivt 
CPU implementer : 0x41
CPU architecture: 7
CPU variant     : 0x0 & 0x3
CPU part        : 0xc07 & 0xc0f
CPU revision    : 5 & 3

Hardware        : Kirin925
Revision        : 0000
Serial          : 0000000000000000"#;
        let cpus = get_vendor_id_and_brand_inner(DATA);
        assert_eq!(cpus.len(), 8);
    }
}
