#[cfg(test)]
mod tests {
    use core::any::TypeId;
    use core::prelude::v1::*;

    use crate::types::CEnumRepr;

    #[test]
    fn c_enum_basic() {
        // By default, variants get sequential values.
        c_enum! {
            pub enum e {
                VAR0,
                VAR1,
                VAR2,
            }

            // Also check enums that don't create a type.
            pub enum #anon {
                ANON0,
                ANON1,
                ANON2,
            }

            // No visibility required.
            enum #anon {
                ANON3,
                ANON4,
                ANON5,
            }
        }

        assert_eq!(TypeId::of::<e>(), TypeId::of::<CEnumRepr>());
        assert_eq!(VAR0, 0 as CEnumRepr);
        assert_eq!(VAR1, 1 as CEnumRepr);
        assert_eq!(VAR2, 2 as CEnumRepr);

        assert_eq!(type_id_of_val(&ANON0), TypeId::of::<CEnumRepr>());
        assert_eq!(ANON0, 0 as CEnumRepr);
        assert_eq!(ANON1, 1 as CEnumRepr);
        assert_eq!(ANON2, 2 as CEnumRepr);

        assert_eq!(type_id_of_val(&ANON3), TypeId::of::<CEnumRepr>());
        assert_eq!(ANON3, 0 as CEnumRepr);
        assert_eq!(ANON4, 1 as CEnumRepr);
        assert_eq!(ANON5, 2 as CEnumRepr);
    }

    #[test]
    fn c_enum_repr() {
        // Check specifying the integer representation
        c_enum! {
            #[repr(u16)]
            pub enum e {
                VAR0,
            }

            #[repr(u16)]
            pub enum #anon {
                ANON0,
            }
        }

        assert_eq!(TypeId::of::<e>(), TypeId::of::<u16>());
        assert_eq!(VAR0, 0_u16);

        assert_eq!(type_id_of_val(&ANON0), TypeId::of::<u16>());
        assert_eq!(ANON0, 0_u16);
    }

    #[test]
    fn c_enum_set_value() {
        // Setting an explicit value resets the count.
        c_enum! {
            pub enum e {
                VAR2 = 2,
                VAR3,
                VAR4,
            }
        }

        assert_eq!(VAR2, 2 as CEnumRepr);
        assert_eq!(VAR3, 3 as CEnumRepr);
        assert_eq!(VAR4, 4 as CEnumRepr);
    }

    #[test]
    fn c_enum_multiple_set_value() {
        // C enums always take one more than the previous value, unless set to a
        // specific value. Duplicates are allowed.
        c_enum! {
            pub enum e {
                VAR0,
                VAR2_0 = 2,
                VAR3_0,
                VAR4_0,
                VAR2_1 = 2,
                VAR3_1,
                VAR4_1,
            }
        }

        assert_eq!(VAR0, 0 as CEnumRepr);
        assert_eq!(VAR2_0, 2 as CEnumRepr);
        assert_eq!(VAR3_0, 3 as CEnumRepr);
        assert_eq!(VAR4_0, 4 as CEnumRepr);
        assert_eq!(VAR2_1, 2 as CEnumRepr);
        assert_eq!(VAR3_1, 3 as CEnumRepr);
        assert_eq!(VAR4_1, 4 as CEnumRepr);
    }

    #[test]
    fn c_enum_vis() {
        mod priv1 {
            c_enum! {
                #[repr(u8)]
                pub enum e1 {
                    PRIV_ON_1 = 10,
                    // Variant should still be usable within its visibility
                    pub PUB1 = PRIV_ON_1 * 2,
                }
            }
        }
        mod priv2 {
            c_enum! {
                #[repr(u16)]
                pub enum e2 {
                    pub PRIV_ON_1 = 42,
                    pub PUB2 = PRIV_ON_1 * 2,
                }
            }
        }

        use priv1::*;
        use priv2::*;

        assert_eq!(TypeId::of::<e1>(), TypeId::of::<u8>());
        assert_eq!(TypeId::of::<e2>(), TypeId::of::<u16>());
        assert_eq!(PUB1, 10u8 * 2);
        assert_eq!(PUB2, 42u16 * 2);
        // Verify that the default is private. If `PRIV_ON_1` was actually
        // public in `priv1`, this would be an ambiguous import and/or type
        // mismatch error.
        assert_eq!(PRIV_ON_1, 42u16);
    }

    #[test]
    fn c_enum_attrs() {
        // Note this can't work with `#[cfg]` currently because our expansion uses `previous + 1`
        c_enum! {
            pub enum e {
                VAR0,
                WITH_CFG = if cfg!(target_arch = "x86_64") { 86 } else { 1234 },
                #[deprecated]
                DEPRECATED,
                NOT_DEPRECATED,
            }
        }

        if cfg!(target_arch = "x86_64") {
            assert_eq!(WITH_CFG, 86);
        } else {
            assert_eq!(WITH_CFG, 1234);
        }

        #[expect(deprecated)]
        let _ = DEPRECATED;
        #[deny(deprecated)]
        let _ = NOT_DEPRECATED;
    }

    #[test]
    #[deny(unused_unsafe)]
    fn f_safety() {
        // Enusure the created functions are safe / unsafe / const as expected
        f! {
            pub unsafe fn unsafe_foo() -> u32 { 100 }
            pub const unsafe fn const_unsafe_foo() -> u32 { 101 }
            pub safe fn safe_foo() -> u32 { 200 }
            pub const safe fn const_safe_foo() -> u32 { 201 }
        }

        assert_eq!(unsafe { unsafe_foo() }, 100u32);
        assert_eq!(const { unsafe { const_unsafe_foo() } }, 101u32);
        assert_eq!(safe_foo(), 200u32);
        assert_eq!(const { const_safe_foo() }, 201u32);

        // Check the ABI
        let _: unsafe extern "C" fn() -> u32 = unsafe_foo;
        let _: unsafe extern "C" fn() -> u32 = const_unsafe_foo;
        let _: extern "C" fn() -> u32 = safe_foo;
        let _: extern "C" fn() -> u32 = const_safe_foo;
    }

    fn type_id_of_val<T: 'static>(_: &T) -> TypeId {
        TypeId::of::<T>()
    }

    #[test]
    fn test_offset_of() {
        #[repr(C)]
        struct Off1 {
            a: u8,
            b: u32,
            c: Off2,
            d: u64,
        }

        #[repr(C)]
        #[repr(align(128))]
        struct Off2 {}

        assert_eq!(core::mem::offset_of!(Off1, a), offset_of!(Off1, a));
        assert_eq!(core::mem::offset_of!(Off1, b), offset_of!(Off1, b));
        assert_eq!(core::mem::offset_of!(Off1, c), offset_of!(Off1, c));
        assert_eq!(core::mem::offset_of!(Off1, d), offset_of!(Off1, d));
    }

    #[test]
    fn s2_is_non_exhaustive() {
        // Without `#[exhaustive]`, the record should have an additional field
        // added at the end. If this test compiles, it has it.
        s2! {
            struct Something {
                a: u32,
            }
        }

        let s = Something::default();
        assert_eq!(s.__non_exhaustive, ());
    }

    #[test]
    fn s2_uses_exhaustive() {
        // With `#[exhaustive]`, the record should be regurgitated as-is. If
        // this test compiles, then it works.
        s2! {
            #[exhaustive]
            struct Something {
                a: u32,
            }
        }

        #[allow(unused)]
        let s = Something {
            a: Default::default(),
        };
    }

    #[test]
    fn s2_uses_mixed_exhaustive() {
        // `#[exhaustive]` should work when sandwiched between attributes. If
        // the test compiles, then it works.
        s2! {
            #[repr(align(8))]
            #[exhaustive]
            #[repr(align(2))]
            struct Something {
                a: u32,
            }
        }

        #[allow(unused)]
        let s = Something {
            a: Default::default(),
        };
    }

    #[test]
    fn s2_uses_custom_default() {
        // A non-default value proves `custom_default` is used rather than a
        // derived default.
        s2! {
            struct CustomDefault {
                a: u32,
                #[custom_default([1; 64])]
                buf: [u8; 64],
            }
        }

        let s = CustomDefault::default();
        assert_eq!(s.a, 0);
        assert_eq!(s.buf, [1u8; 64]);
    }

    #[test]
    fn s2_keeps_field_attrs() {
        // If `custom_default` stripping ate the other field attributes, the two
        // `a` fields would collide.
        s2! {
            struct FieldAttrs {
                #[cfg(target_arch = "x86_64")]
                a: u8,
                #[cfg(not(target_arch = "x86_64"))]
                a: u64,
            }
        }

        let s = FieldAttrs::default();
        #[cfg(target_arch = "x86_64")]
        assert_eq!(s.a, 0u8);
        #[cfg(not(target_arch = "x86_64"))]
        assert_eq!(s.a, 0u64);
    }

    #[test]
    fn s2_single_cfg_field() {
        // This field only exists on x86_64, so its default init needs the same
        // `cfg` or `Default` won't build on other arches.
        s2! {
            struct SingleCfg {
                common: u32,
                #[cfg(target_arch = "x86_64")]
                x86_only: u64,
            }
        }

        let s = SingleCfg::default();
        assert_eq!(s.common, 0);
        #[cfg(target_arch = "x86_64")]
        assert_eq!(s.x86_only, 0);
    }

    #[test]
    fn s_no_extra_traits2_zeroes_union() {
        // A union field's default is supplied by
        // `custom_default(unsafe { mem::zeroed })`.
        s_no_extra_traits2! {
            union U {
                a: u32,
                b: f32,
            }

            struct HasUnion {
                x: u16,
                #[custom_default(unsafe { ::core::mem::zeroed::<U>() })]
                u: U,
            }
        }

        let s = HasUnion::default();
        assert_eq!(s.x, 0);
        assert_eq!(unsafe { s.u.a }, 0);
    }

    #[test]
    fn s2_keeps_struct_cfg() {
        // The opposite of the configured-out types in `macro_checks`. With the
        // `cfg` true the type and its `Default` both exist, and the other
        // attributes still apply.
        s2! {
            #[cfg(true)]
            #[repr(align(8))]
            /// a doc comment
            struct EnabledCfg {
                a: u32,
                #[cfg(target_arch = "x86_64")]
                x86_only: u8,
                #[custom_default([1; 40])]
                buf: [u8; 40],
            }
        }

        let s = EnabledCfg::default();
        assert_eq!(s.a, 0);
        assert_eq!(s.buf, [1u8; 40]);
        assert_eq!(align_of::<EnabledCfg>(), 8);
        #[cfg(target_arch = "x86_64")]
        assert_eq!(s.x86_only, 0);
    }

    #[test]
    fn s2_union_default() {
        s_no_extra_traits2! {
            #[unsafe(union_default_via_zeroed)]
            union Foo {
                a: u32,
                b: u64
            }

            #[expect(unused)]
            union Bar {
                a: u32,
                b: u64
            }
        }

        let f: Foo = Foo::default();
        assert_eq!(unsafe { f.b }, 0);

        // Ensure it isn't always implemented, otherwise we would get a conflict.
        impl Default for Bar {
            fn default() -> Self {
                unimplemented!()
            }
        }
    }
}
