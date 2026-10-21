use unicode_normalizer::{ComposingNormalizer, ComposingNormalizerBorrowed, DecomposingNormalizerBorrowed};

#[test]
fn unicode_normalization_conformance_corpus() {
    let nfc = ComposingNormalizerBorrowed::new_nfc();
    let nfd = DecomposingNormalizerBorrowed::new_nfd();
    let nfkc = ComposingNormalizerBorrowed::new_nfkc();
    let nfkd = DecomposingNormalizerBorrowed::new_nfkd();
    let data = include_str!("../foundation/unicode-normalizer/tests/data/NormalizationTest.txt");
    let mut count = 0;
    for (number, raw) in data.lines().enumerate() {
        let line = raw.split('#').next().unwrap().trim();
        if line.is_empty() || line.starts_with('@') { continue; }
        let columns: Vec<String> = line.split(';').take(5).map(|column| {
            column.split_whitespace().map(|hex| {
                char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap()
            }).collect()
        }).collect();
        assert_eq!(columns.len(), 5, "line {}", number + 1);
        for (index, input) in columns.iter().enumerate() {
            assert_eq!(nfc.normalize(input), columns[if index < 3 { 1 } else { 3 }], "NFC line {}", number + 1);
            assert_eq!(nfd.normalize(input), columns[if index < 3 { 2 } else { 4 }], "NFD line {}", number + 1);
            assert_eq!(nfkc.normalize(input), columns[3], "NFKC line {}", number + 1);
            assert_eq!(nfkd.normalize(input), columns[4], "NFKD line {}", number + 1);
        }
        count += 1;
    }
    assert!(count > 19_000, "normalization corpus was truncated: {count}");
}

#[test]
fn supplied_baked_provider_and_utf8_edge_vectors() {
    let supplied = ComposingNormalizer::try_new_nfc_unstable(&unicode_normalizer::provider::Baked).unwrap();
    let baked = ComposingNormalizerBorrowed::new_nfc();
    for text in ["", "plain ASCII", "a\u{0308}", "e\u{0323}\u{0302}", "\u{1100}\u{1161}", "𝅗𝅥", "\u{10ffff}"] {
        assert_eq!(supplied.as_borrowed().normalize(text), baked.normalize(text));
    }
    assert_eq!(baked.normalize("a\u{0308}"), "ä");
    assert_eq!(baked.normalize("\u{1100}\u{1161}"), "가");
    assert_eq!(DecomposingNormalizerBorrowed::new_nfkd().normalize("ﬁ"), "fi");
}

#[test]
fn uts46_mapping_retains_case_width_and_ignorable_behavior() {
    let mapper = unicode_normalizer::uts46::Uts46MapperBorrowed::new();
    assert_eq!(mapper.map_normalize("BÜCHER".chars()).collect::<String>(), "bücher");
    assert_eq!(mapper.map_normalize("ＦＯＯ".chars()).collect::<String>(), "foo");
    assert_eq!(mapper.map_normalize("a\u{00ad}b".chars()).collect::<String>(), "ab");
}
