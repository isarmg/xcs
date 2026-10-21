#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn termios_layouts() {
        check_renamed_type!(InputModes, tcflag_t);
        check_renamed_type!(OutputModes, tcflag_t);
        check_renamed_type!(ControlModes, tcflag_t);
        check_renamed_type!(LocalModes, tcflag_t);
        static_assertions::assert_eq_size!(u8, libc::cc_t);
        static_assertions::assert_eq_size!(types::tcflag_t, libc::tcflag_t);

        check_renamed_struct!(Winsize, winsize, ws_row, ws_col, ws_xpixel, ws_ypixel);

        // On platforms with a termios/termios2 split, check `termios`.
        #[cfg(linux_raw)]
        {
            check_renamed_type!(Termios, termios2);
            check_renamed_struct_renamed_field!(Termios, termios2, input_modes, c_iflag);
            check_renamed_struct_renamed_field!(Termios, termios2, output_modes, c_oflag);
            check_renamed_struct_renamed_field!(Termios, termios2, control_modes, c_cflag);
            check_renamed_struct_renamed_field!(Termios, termios2, local_modes, c_lflag);
            check_renamed_struct_renamed_field!(Termios, termios2, line_discipline, c_line);
            check_renamed_struct_renamed_field!(Termios, termios2, special_codes, c_cc);
            check_renamed_struct_renamed_field!(Termios, termios2, input_speed, c_ispeed);
            check_renamed_struct_renamed_field!(Termios, termios2, output_speed, c_ospeed);

            // We assume that `termios` has the same layout as `termios2` minus the
            // `c_ispeed` and `c_ospeed` fields.
            check_renamed_struct_renamed_field!(Termios, termios, input_modes, c_iflag);
            check_renamed_struct_renamed_field!(Termios, termios, output_modes, c_oflag);
            check_renamed_struct_renamed_field!(Termios, termios, control_modes, c_cflag);
            check_renamed_struct_renamed_field!(Termios, termios, local_modes, c_lflag);
            check_renamed_struct_renamed_field!(Termios, termios, special_codes, c_cc);

            // On everything except PowerPC, `termios` matches `termios2` except
            // for the addition of `c_ispeed` and `c_ospeed`.
            #[cfg(not(any(target_arch = "powerpc", target_arch = "powerpc64")))]
            static_assertions::const_assert_eq!(
                memoffset::offset_of!(Termios, input_speed),
                core::mem::size_of::<c::termios>()
            );

            // On PowerPC, `termios2` is `termios`.
            #[cfg(any(target_arch = "powerpc", target_arch = "powerpc64"))]
            static_assertions::assert_eq_size!(c::termios2, c::termios);
        }

        #[cfg(not(linux_raw))]
        {
            // On MIPS, SPARC, and Android, the libc lacks the ospeed and ispeed
            // fields.
            #[cfg(all(
                not(all(
                    target_env = "gnu",
                    any(
                        target_arch = "mips",
                        target_arch = "mips32r6",
                        target_arch = "mips64",
                        target_arch = "mips64r6",
                        target_arch = "sparc",
                        target_arch = "sparc64"
                    )
                )),
                not(all(libc, target_os = "android"))
            ))]
            check_renamed_type!(Termios, termios);
            #[cfg(not(all(
                not(all(
                    target_env = "gnu",
                    any(
                        target_arch = "mips",
                        target_arch = "mips32r6",
                        target_arch = "mips64",
                        target_arch = "mips64r6",
                        target_arch = "sparc",
                        target_arch = "sparc64"
                    )
                )),
                not(all(libc, target_os = "android"))
            )))]
            static_assertions::const_assert!(
                core::mem::size_of::<Termios>() >= core::mem::size_of::<c::termios>()
            );

            check_renamed_struct_renamed_field!(Termios, termios, input_modes, c_iflag);
            check_renamed_struct_renamed_field!(Termios, termios, output_modes, c_oflag);
            check_renamed_struct_renamed_field!(Termios, termios, control_modes, c_cflag);
            check_renamed_struct_renamed_field!(Termios, termios, local_modes, c_lflag);
            #[cfg(any(
                linux_like,
                target_env = "newlib",
                target_os = "fuchsia",
                target_os = "haiku",
                target_os = "redox"
            ))]
            check_renamed_struct_renamed_field!(Termios, termios, line_discipline, c_line);
            check_renamed_struct_renamed_field!(Termios, termios, special_codes, c_cc);
            #[cfg(not(any(
                linux_kernel,
                solarish,
                target_os = "emscripten",
                target_os = "fuchsia"
            )))]
            {
                check_renamed_struct_renamed_field!(Termios, termios, input_speed, c_ispeed);
                check_renamed_struct_renamed_field!(Termios, termios, output_speed, c_ospeed);
            }
            #[cfg(any(target_env = "musl", target_os = "fuchsia"))]
            {
                check_renamed_struct_renamed_field!(Termios, termios, input_speed, __c_ispeed);
                check_renamed_struct_renamed_field!(Termios, termios, output_speed, __c_ospeed);
            }
        }

        check_renamed_type!(OptionalActions, c_int);
        check_renamed_type!(QueueSelector, c_int);
        check_renamed_type!(Action, c_int);
    }

    #[test]
    #[cfg(not(any(
        solarish,
        target_os = "cygwin",
        target_os = "emscripten",
        target_os = "haiku",
        target_os = "redox",
    )))]
    fn termios_legacy() {
        // Check that our doc aliases above are correct.
        static_assertions::const_assert_eq!(c::EXTA, c::B19200);
        static_assertions::const_assert_eq!(c::EXTB, c::B38400);
    }

    #[cfg(bsd)]
    #[test]
    fn termios_bsd() {
        // On BSD platforms we can assume that the `B*` constants have their
        // arbitrary integer speed value. Confirm this.
        static_assertions::const_assert_eq!(c::B0, 0);
        static_assertions::const_assert_eq!(c::B50, 50);
        static_assertions::const_assert_eq!(c::B19200, 19200);
        static_assertions::const_assert_eq!(c::B38400, 38400);
    }

    #[test]
    #[cfg(not(bsd))]
    fn termios_speed_encoding() {
        assert_eq!(speed::encode(0), Some(c::B0));
        assert_eq!(speed::encode(50), Some(c::B50));
        assert_eq!(speed::encode(19200), Some(c::B19200));
        assert_eq!(speed::encode(38400), Some(c::B38400));
        assert_eq!(speed::encode(1), None);
        assert_eq!(speed::encode(!0), None);

        #[cfg(not(linux_kernel))]
        {
            assert_eq!(speed::decode(c::B0), Some(0));
            assert_eq!(speed::decode(c::B50), Some(50));
            assert_eq!(speed::decode(c::B19200), Some(19200));
            assert_eq!(speed::decode(c::B38400), Some(38400));
        }
    }

    #[cfg(linux_kernel)]
    #[test]
    fn termios_ioctl_contiguity() {
        // When using `termios2`, we assume that we can add the optional actions
        // value to the ioctl request code. Test this assumption.

        static_assertions::const_assert_eq!(c::TCSETS2, c::TCSETS2 + 0);
        static_assertions::const_assert_eq!(c::TCSETSW2, c::TCSETS2 + 1);
        static_assertions::const_assert_eq!(c::TCSETSF2, c::TCSETS2 + 2);

        static_assertions::const_assert_eq!(c::TCSANOW - c::TCSANOW, 0);
        static_assertions::const_assert_eq!(c::TCSADRAIN - c::TCSANOW, 1);
        static_assertions::const_assert_eq!(c::TCSAFLUSH - c::TCSANOW, 2);

        // MIPS is different here.
        #[cfg(any(
            target_arch = "mips",
            target_arch = "mips32r6",
            target_arch = "mips64",
            target_arch = "mips64r6"
        ))]
        {
            assert_eq!(i128::from(c::TCSANOW) - i128::from(c::TCSETS), 0);
            assert_eq!(i128::from(c::TCSADRAIN) - i128::from(c::TCSETS), 1);
            assert_eq!(i128::from(c::TCSAFLUSH) - i128::from(c::TCSETS), 2);
        }
        #[cfg(not(any(
            target_arch = "mips",
            target_arch = "mips32r6",
            target_arch = "mips64",
            target_arch = "mips64r6"
        )))]
        {
            static_assertions::const_assert_eq!(c::TCSANOW, 0);
            static_assertions::const_assert_eq!(c::TCSADRAIN, 1);
            static_assertions::const_assert_eq!(c::TCSAFLUSH, 2);
        }
    }

    #[cfg(linux_kernel)]
    #[test]
    fn termios_cibaud() {
        // Test an assumption.
        static_assertions::const_assert_eq!(c::CIBAUD, c::CBAUD << c::IBSHIFT);
    }
}
