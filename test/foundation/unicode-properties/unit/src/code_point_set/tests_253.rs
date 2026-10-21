#[cfg(test)]
mod tests {
    #[test]
    fn test_general_category() {
        use crate::CodePointMapData;
        use crate::props::GeneralCategory;
        use crate::props::GeneralCategoryGroup;

        let digits_data = CodePointMapData::<GeneralCategory>::new()
            .get_set_for_value_group(GeneralCategoryGroup::Number);
        let digits = digits_data.as_borrowed();

        assert!(digits.contains('5'));
        assert!(digits.contains('\u{0665}')); // U+0665 ARABIC-INDIC DIGIT FIVE
        assert!(digits.contains('\u{096b}')); // U+0969 DEVANAGARI DIGIT FIVE

        assert!(!digits.contains('A'));
    }

    #[test]
    fn test_script() {
        use crate::CodePointMapData;
        use crate::props::Script;

        let thai_data = CodePointMapData::<Script>::new().get_set_for_value(Script::Thai);
        let thai = thai_data.as_borrowed();

        assert!(thai.contains('\u{0e01}')); // U+0E01 THAI CHARACTER KO KAI
        assert!(thai.contains('\u{0e50}')); // U+0E50 THAI DIGIT ZERO

        assert!(!thai.contains('A'));
        assert!(!thai.contains('\u{0e3f}')); // U+0E50 THAI CURRENCY SYMBOL BAHT
    }

    #[test]
    fn test_gc_groupings() {
        use crate::CodePointMapData;
        use crate::props::{GeneralCategory, GeneralCategoryGroup};
        use icu_collections::codepointinvlist::CodePointInversionListBuilder;

        let test_group = |category: GeneralCategoryGroup, subcategories: &[GeneralCategory]| {
            let category_set =
                CodePointMapData::<GeneralCategory>::new().get_set_for_value_group(category);
            let category_set = category_set
                .as_code_point_inversion_list()
                .expect("The data should be valid");

            let mut builder = CodePointInversionListBuilder::new();
            for &subcategory in subcategories {
                let gc_set_data =
                    CodePointMapData::<GeneralCategory>::new().get_set_for_value(subcategory);
                let gc_set = gc_set_data.as_borrowed();
                for range in gc_set.iter_ranges() {
                    builder.add_range32(range);
                }
            }
            let combined_set = builder.build();
            println!("{category:?} {subcategories:?}");
            assert_eq!(
                category_set.get_inversion_list_vec(),
                combined_set.get_inversion_list_vec()
            );
        };

        test_group(
            GeneralCategoryGroup::Letter,
            &[
                GeneralCategory::UppercaseLetter,
                GeneralCategory::LowercaseLetter,
                GeneralCategory::TitlecaseLetter,
                GeneralCategory::ModifierLetter,
                GeneralCategory::OtherLetter,
            ],
        );
        test_group(
            GeneralCategoryGroup::Other,
            &[
                GeneralCategory::Control,
                GeneralCategory::Format,
                GeneralCategory::Unassigned,
                GeneralCategory::PrivateUse,
                GeneralCategory::Surrogate,
            ],
        );
        test_group(
            GeneralCategoryGroup::Mark,
            &[
                GeneralCategory::SpacingMark,
                GeneralCategory::EnclosingMark,
                GeneralCategory::NonspacingMark,
            ],
        );
        test_group(
            GeneralCategoryGroup::Number,
            &[
                GeneralCategory::DecimalNumber,
                GeneralCategory::LetterNumber,
                GeneralCategory::OtherNumber,
            ],
        );
        test_group(
            GeneralCategoryGroup::Punctuation,
            &[
                GeneralCategory::ConnectorPunctuation,
                GeneralCategory::DashPunctuation,
                GeneralCategory::ClosePunctuation,
                GeneralCategory::FinalPunctuation,
                GeneralCategory::InitialPunctuation,
                GeneralCategory::OtherPunctuation,
                GeneralCategory::OpenPunctuation,
            ],
        );
        test_group(
            GeneralCategoryGroup::Symbol,
            &[
                GeneralCategory::CurrencySymbol,
                GeneralCategory::ModifierSymbol,
                GeneralCategory::MathSymbol,
                GeneralCategory::OtherSymbol,
            ],
        );
        test_group(
            GeneralCategoryGroup::Separator,
            &[
                GeneralCategory::LineSeparator,
                GeneralCategory::ParagraphSeparator,
                GeneralCategory::SpaceSeparator,
            ],
        );
    }

    #[test]
    fn test_gc_surrogate() {
        use crate::CodePointMapData;
        use crate::props::GeneralCategory;

        let surrogates_data = CodePointMapData::<GeneralCategory>::new()
            .get_set_for_value(GeneralCategory::Surrogate);
        let surrogates = surrogates_data.as_borrowed();

        assert!(surrogates.contains32(0xd800));
        assert!(surrogates.contains32(0xd900));
        assert!(surrogates.contains32(0xdfff));

        assert!(!surrogates.contains('A'));
    }
}
