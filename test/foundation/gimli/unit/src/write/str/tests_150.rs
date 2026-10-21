#[cfg(test)]
#[cfg(feature = "read")]
mod tests {
    use super::*;
    use crate::read;
    use crate::write::EndianVec;
    use crate::LittleEndian;

    #[test]
    fn test_string_table() {
        let mut strings = StringTable::default();
        assert_eq!(strings.count(), 0);
        let id1 = strings.add(&b"one"[..]);
        let id2 = strings.add(&b"two"[..]);
        assert_eq!(strings.add(&b"one"[..]), id1);
        assert_eq!(strings.add(&b"two"[..]), id2);
        assert_eq!(strings.get(id1), &b"one"[..]);
        assert_eq!(strings.get(id2), &b"two"[..]);
        assert_eq!(strings.count(), 2);

        let mut debug_str = DebugStr::from(EndianVec::new(LittleEndian));
        let offsets = strings.write(&mut debug_str).unwrap();
        assert_eq!(debug_str.slice(), b"one\0two\0");
        assert_eq!(offsets.get(id1), DebugStrOffset(0));
        assert_eq!(offsets.get(id2), DebugStrOffset(4));
        assert_eq!(offsets.get(offsets.get_empty().unwrap()), DebugStrOffset(3));
        assert_eq!(offsets.count(), 3);
    }

    #[test]
    fn test_string_table_read() {
        let mut strings = StringTable::default();
        let id1 = strings.add(&b"one"[..]);
        let id2 = strings.add(&b"two"[..]);

        let mut debug_str = DebugStr::from(EndianVec::new(LittleEndian));
        let offsets = strings.write(&mut debug_str).unwrap();

        let read_debug_str = read::DebugStr::new(debug_str.slice(), LittleEndian);
        let str1 = read_debug_str.get_str(offsets.get(id1)).unwrap();
        let str2 = read_debug_str.get_str(offsets.get(id2)).unwrap();
        let str3 = read_debug_str
            .get_str(offsets.get(offsets.get_empty().unwrap()))
            .unwrap();
        assert_eq!(str1.slice(), &b"one"[..]);
        assert_eq!(str2.slice(), &b"two"[..]);
        assert_eq!(str3.slice(), b"");
    }
}
