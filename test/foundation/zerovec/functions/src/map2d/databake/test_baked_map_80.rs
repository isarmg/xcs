#[test]
fn test_baked_map() {
    test_bake!(
        ZeroMap2d<str, str, str>,
        const,
        unsafe {
            #[allow(unused_unsafe)]
            crate::ZeroMap2d::from_parts_unchecked(
                unsafe {
                    crate::vecs::VarZeroVec16::from_bytes_unchecked(
                        b"\x0E\0\0\0\0\0\x05\0\x07\0\t\0\x0B\0\x10\0\x12\0\x14\0\x1C\0\x1E\0#\0%\0'\0,\0arcazcuenffgrckkkukylifmanmnpapalsdtgugunruzyuezh"
                    )
                },
                unsafe {
                    crate::ZeroVec::from_bytes_unchecked(
                        b"\x02\0\0\0\x03\0\0\0\x04\0\0\0\x05\0\0\0\x06\0\0\0\x07\0\0\0\x08\0\0\0\n\0\0\0\x0C\0\0\0\r\0\0\0\x0E\0\0\0\x0F\0\0\0\x10\0\0\0\x11\0\0\0\x14\0\0\0\x15\0\0\0\x16\0\0\0\x17\0\0\0\x18\0\0\0\x19\0\0\0\x1C\0\0\0"
                    )
                },
                unsafe {
                    crate::vecs::VarZeroVec16::from_bytes_unchecked(
                        b"\x1C\0\0\0\0\0\x04\0\x08\0\x0C\0\x10\0\x14\0\x18\0\x1C\0 \0$\0(\0,\x000\x004\08\0<\0@\0D\0H\0L\0P\0T\0X\0\\\0`\0d\0h\0l\0NbatPalmArabGlagShawAdlmLinbArabArabYeziArabLatnLimbNkooMongArabPhlpDevaKhojSindArabCyrlDevaArabHansBopoHanbHant"
                    )
                },
                unsafe {
                    crate::vecs::VarZeroVec16::from_bytes_unchecked(
                        b"\x1C\0\0\0\0\0\x02\0\x04\0\x06\0\x08\0\n\0\x0C\0\x0E\0\x10\0\x12\0\x14\0\x16\0\x18\0\x1A\0\x1C\0\x1E\0 \0\"\0$\0&\0(\0*\0,\0.\x000\x002\x004\x006\0JOSYIRBGGBGNGRCNIQGECNTRINGNCNPKCNINININPKKZNPAFCNTWTWTW"
                    )
                },
            )
        },
        zerovec
    );
}
