#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write;

    mod data {
        include!("../../../tests/data/data.rs");
    }

    #[test]
    fn test_write_char() {
        for a in data::KEBAB_CASE_STRINGS {
            for b in data::KEBAB_CASE_STRINGS {
                let mut wc = WriteComparator::new(a.as_bytes());
                for ch in b.chars() {
                    wc.write_char(ch).unwrap();
                }
                assert_eq!(a.cmp(b), wc.finish(), "{a} <=> {b}");
            }
        }
    }

    #[test]
    fn test_write_str() {
        for a in data::KEBAB_CASE_STRINGS {
            for b in data::KEBAB_CASE_STRINGS {
                let mut wc = WriteComparator::new(a.as_bytes());
                wc.write_str(b).unwrap();
                assert_eq!(a.cmp(b), wc.finish(), "{a} <=> {b}");
            }
        }
    }

    #[test]
    fn test_mixed() {
        for a in data::KEBAB_CASE_STRINGS {
            for b in data::KEBAB_CASE_STRINGS {
                let mut wc = WriteComparator::new(a.as_bytes());
                let mut first = true;
                for substr in b.split('-') {
                    if first {
                        first = false;
                    } else {
                        wc.write_char('-').unwrap();
                    }
                    wc.write_str(substr).unwrap();
                }
                assert_eq!(a.cmp(b), wc.finish(), "{a} <=> {b}");
            }
        }
    }
}
