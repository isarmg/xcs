#[cfg(test)]
mod test {
    #[allow(unused)]
    use std::f32::consts::PI;

    #[allow(unused)]
    macro_rules! assert_fmt {
        ($fmt:tt, $context:tt. $($fn:tt),+($key:tt), $expected:tt) => {
            assert_eq!(
                format!(
                    $fmt,
                    $($context.get(stringify!($key)).unwrap().$fn().unwrap()),+),
                    $expected
                );
        };
    }

    #[test]
    #[cfg(all(feature = "debug", feature = "number", feature = "pointer"))]
    fn context() {
        let pointer = &32;
        let context = context! {
            a = "",
            a:? = "",
            b:?:b = 10,
            d: :? = "hello",
        };
        assert_fmt!("{}", context.get_display(a), "");
        assert_fmt!("{:?}", context.get_debug(a), "\"\"");
        assert_fmt!("{:?}", context.get_debug(b), "10");
        assert_fmt!("{:b}", context.get_binary(b), "1010");
        assert_fmt!("{:}", context.get_display(d), "hello");
        assert_fmt!("{:?}", context.get_debug(d), "\"hello\"");
        let context = context! {
            debug1:? = "debug",
            debug2:debug = "debug",
            display1 = "display",
            display2: = "display",
            display3:display = "display",
            integer:integer = 42,
            float:float = PI,
            binary1:b = 12,
            binary2:binary = 12,
            lower_exp1:e = 0.01,
            lower_exp2:lower_exp = 0.01,
            lower_hex1:x = 0xA9,
            lower_hex2:lower_hex = 0xA9,
            octal1:o = 0o20,
            octal2:octal = 0o20,
            upper_exp1:E = 0.01,
            upper_exp2:upper_exp = 0.01,
            upper_hex1:X = 0xA9,
            upper_hex2:upper_hex = 0xA9,
            pointer1:p = pointer,
            pointer2:pointer = pointer,
            // iter:i:? = [1, 2],
        };
        assert_fmt!("{}", context.get_display(display1), "display");
        assert_fmt!("{}", context.get_display(display2), "display");
        assert_fmt!("{}", context.get_display(display3), "display");
        assert_fmt!("{:?}", context.get_debug(debug1), "\"debug\"");
        assert_fmt!("{:?}", context.get_debug(debug2), "\"debug\"");
        assert_fmt!(
            "{:b} {:?} {:} {:e} {:x} {:o} {:E} {:X}",
            context.get_binary,
            get_debug,
            get_display,
            get_lower_exp,
            get_lower_hex,
            get_octal,
            get_upper_exp,
            get_upper_hex(integer),
            "101010 42 42 4.2e1 2a 52 4.2E1 2A"
        );
        assert_fmt!(
            "{:?} {:} {:e} {:E}",
            context.get_debug,
            get_display,
            get_lower_exp,
            get_upper_exp(float),
            "3.1415927 3.1415927 3.1415927e0 3.1415927E0"
        );
        assert_fmt!("{:b}", context.get_binary(binary1), "1100");
        assert_fmt!("{:b}", context.get_binary(binary2), "1100");
        assert_fmt!("{:e}", context.get_lower_exp(lower_exp1), "1e-2");
        assert_fmt!("{:e}", context.get_lower_exp(lower_exp2), "1e-2");
        assert_fmt!("{:x}", context.get_lower_hex(lower_hex1), "a9");
        assert_fmt!("{:x}", context.get_lower_hex(lower_hex2), "a9");
        assert_fmt!("{:o}", context.get_octal(octal1), "20");
        assert_fmt!("{:o}", context.get_octal(octal2), "20");
        assert_fmt!("{:E}", context.get_upper_exp(upper_exp1), "1E-2");
        assert_fmt!("{:E}", context.get_upper_exp(upper_exp2), "1E-2");
        assert_fmt!("{:X}", context.get_upper_hex(upper_hex1), "A9");
        assert_fmt!("{:X}", context.get_upper_hex(upper_hex2), "A9");
        assert_fmt!(
            "{:p}",
            context.get_pointer(pointer1),
            (format!("{pointer:p}"))
        );
        assert_fmt!(
            "{:p}",
            context.get_pointer(pointer2),
            (format!("{pointer:p}"))
        );
    }
    #[allow(unused)]
    macro_rules! assert_i {
        ($fmt:literal, $variable:ident, $rhs:literal) => {
            assert_eq!(
                iformat!(concat!("{list:i(", $fmt, ")(,)}"), $variable: i).unwrap(),
                $rhs
            );
        };
    }

    #[test]
    #[cfg(feature = "iter")]
    fn list() {
        let list = list!["a", 1];
        assert_i!("{}", list, "a,1");
        #[cfg(feature = "debug")]
        {
            let list = list![:?; "a", 1];
            assert_i!("{:?}", list, "\"a\",1");
            #[cfg(feature = "number")]
            {
                let list = list![::?:X; 2, 1];
                assert_i!("{}{:?}{:X}", list, "222,111");
            }
        }
    }
}
