#[cfg(test)]
mod tests {
    #[test]
    fn function_new_with_locals_test() {
        use super::*;

        // Test the algorithm for conversion is correct
        let f1 = Function::new_with_locals_types([
            ValType::I32,
            ValType::I32,
            ValType::I64,
            ValType::F32,
            ValType::F32,
            ValType::F32,
            ValType::I32,
            ValType::I64,
            ValType::I64,
        ]);
        let f2 = Function::new([
            (2, ValType::I32),
            (1, ValType::I64),
            (3, ValType::F32),
            (1, ValType::I32),
            (2, ValType::I64),
        ]);

        assert_eq!(f1.bytes, f2.bytes)
    }

    #[test]
    fn func_raw_bytes() {
        use super::*;

        let mut f = Function::new([(1, ValType::I32), (1, ValType::F32)]);
        f.instructions().end();
        let mut code_from_func = CodeSection::new();
        code_from_func.function(&f);
        let bytes = f.into_raw_body();
        let mut code_from_raw = CodeSection::new();
        code_from_raw.raw(&bytes[..]);

        let mut c1 = vec![];
        code_from_func.encode(&mut c1);
        let mut c2 = vec![];
        code_from_raw.encode(&mut c2);
        assert_eq!(c1, c2);
    }
}
