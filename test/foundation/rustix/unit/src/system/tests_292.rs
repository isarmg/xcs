#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::backend::c;

    #[cfg(linux_kernel)]
    #[test]
    fn test_sysinfo_layouts() {
        // Don't assert the size for `Sysinfo` because `c::sysinfo` has a
        // computed-size padding field at the end that bindgen doesn't support,
        // and `c::sysinfo` may add fields over time.
        assert_eq!(
            core::mem::align_of::<Sysinfo>(),
            core::mem::align_of::<c::sysinfo>()
        );
        check_renamed_struct_field!(Sysinfo, sysinfo, uptime);
        check_renamed_struct_field!(Sysinfo, sysinfo, loads);
        check_renamed_struct_field!(Sysinfo, sysinfo, totalram);
        check_renamed_struct_field!(Sysinfo, sysinfo, freeram);
        check_renamed_struct_field!(Sysinfo, sysinfo, sharedram);
        check_renamed_struct_field!(Sysinfo, sysinfo, bufferram);
        check_renamed_struct_field!(Sysinfo, sysinfo, totalswap);
        check_renamed_struct_field!(Sysinfo, sysinfo, freeswap);
        check_renamed_struct_field!(Sysinfo, sysinfo, procs);
        check_renamed_struct_field!(Sysinfo, sysinfo, totalhigh);
        check_renamed_struct_field!(Sysinfo, sysinfo, freehigh);
        check_renamed_struct_field!(Sysinfo, sysinfo, mem_unit);
    }
}
