#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headers_foreach_cases_preserved() {
        let mut headers = hyper_headers::default();

        let name1 = b"Set-CookiE";
        let value1 = b"a=b";
        hyper_headers_add(
            &mut headers,
            name1.as_ptr(),
            name1.len(),
            value1.as_ptr(),
            value1.len(),
        );

        let name2 = b"SET-COOKIE";
        let value2 = b"c=d";
        hyper_headers_add(
            &mut headers,
            name2.as_ptr(),
            name2.len(),
            value2.as_ptr(),
            value2.len(),
        );

        let mut vec = Vec::<u8>::new();
        hyper_headers_foreach(&headers, concat, &mut vec as *mut _ as *mut c_void);

        assert_eq!(vec, b"Set-CookiE: a=b\r\nSET-COOKIE: c=d\r\n");

        extern "C" fn concat(
            vec: *mut c_void,
            name: *const u8,
            name_len: usize,
            value: *const u8,
            value_len: usize,
        ) -> c_int {
            unsafe {
                let vec = &mut *(vec as *mut Vec<u8>);
                let name = std::slice::from_raw_parts(name, name_len);
                let value = std::slice::from_raw_parts(value, value_len);
                vec.extend(name);
                vec.extend(b": ");
                vec.extend(value);
                vec.extend(b"\r\n");
            }
            HYPER_ITER_CONTINUE
        }
    }

    #[cfg(all(feature = "http1", feature = "ffi"))]
    #[test]
    fn test_headers_foreach_order_preserved() {
        let mut headers = hyper_headers::default();

        let name1 = b"Set-CookiE";
        let value1 = b"a=b";
        hyper_headers_add(
            &mut headers,
            name1.as_ptr(),
            name1.len(),
            value1.as_ptr(),
            value1.len(),
        );

        let name2 = b"Content-Encoding";
        let value2 = b"gzip";
        hyper_headers_add(
            &mut headers,
            name2.as_ptr(),
            name2.len(),
            value2.as_ptr(),
            value2.len(),
        );

        let name3 = b"SET-COOKIE";
        let value3 = b"c=d";
        hyper_headers_add(
            &mut headers,
            name3.as_ptr(),
            name3.len(),
            value3.as_ptr(),
            value3.len(),
        );

        let mut vec = Vec::<u8>::new();
        hyper_headers_foreach(&headers, concat, &mut vec as *mut _ as *mut c_void);

        println!("{}", std::str::from_utf8(&vec).unwrap());
        assert_eq!(
            vec,
            b"Set-CookiE: a=b\r\nContent-Encoding: gzip\r\nSET-COOKIE: c=d\r\n"
        );

        extern "C" fn concat(
            vec: *mut c_void,
            name: *const u8,
            name_len: usize,
            value: *const u8,
            value_len: usize,
        ) -> c_int {
            unsafe {
                let vec = &mut *(vec as *mut Vec<u8>);
                let name = std::slice::from_raw_parts(name, name_len);
                let value = std::slice::from_raw_parts(value, value_len);
                vec.extend(name);
                vec.extend(b": ");
                vec.extend(value);
                vec.extend(b"\r\n");
            }
            HYPER_ITER_CONTINUE
        }
    }
}
