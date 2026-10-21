#[cfg(test)]
mod tests {
    use super::Entry;
    use super::Map;

    #[test]
    fn various() {
        let mut m: Map<char, u8, 10> =
            Map::from_iter([('a', 97), ('d', 100), ('c', 99), ('b', 98)]);
        let e: Entry<'_, char, u8, 10> = m.entry('c');
        assert_eq!(e.key(), &'c');
        m.entry('e').or_insert(b'e');
        m.entry('e').or_insert(b'e');
        assert_eq!(*m.entry('e').and_modify(|v| *v = 42).or_default(), 42);
        assert_eq!(m.entry('g').key(), &'g');
        assert_eq!(
            *m.entry('g').and_modify(|v| *v = 42).or_default(),
            u8::default()
        );
        let value = if let Entry::Occupied(mut entry) = m.entry('e') {
            let value = *entry.get();
            assert_eq!(value, 42);
            *entry.get_mut() = b'E';
            assert_eq!(*entry.get(), 69);
            let e = entry.into_mut();
            *e = b'e';
            value
        } else {
            100
        };
        assert_eq!(*m.entry('f').or_insert_with(|| value + 1), 43); // _ -> 43
        assert_eq!(*m.entry('f').or_insert_with(|| value + 2), 43); // no change
        assert_eq!(m.remove_entry(&'f'), Some(('f', 43))); // 43 -> _
        assert_eq!(
            *m.entry('f')
                .or_insert_with_key(|&key| key.try_into().unwrap()),
            102
        ); // _ -> 102
        assert_eq!(*m.entry('f').or_insert_with_key(|&_| 255), 102); // no change
        if let Entry::Occupied(entry) = m.entry('e') {
            assert_eq!(entry.remove(), 101);
        }
        if let Entry::Vacant(entry) = m.entry('e') {
            assert_eq!(entry.key(), &'e');
            assert_eq!(entry.into_key(), 'e');
        }
        if let Entry::Vacant(entry) = m.entry('e') {
            assert_eq!(entry.key(), &'e');
            entry.insert(b'e');
        }
        if let Entry::Occupied(mut entry) = m.entry('e') {
            entry.insert(b'E');
            assert_eq!(entry.remove_entry(), ('e', b'E'));
        }
        let occupied_entry = m.entry('e').insert_entry(b'e');
        assert_eq!(occupied_entry.get(), &b'e');
    }
}
