#[cfg(test)]
mod tests {
    use super::ToTrainCase;

    macro_rules! t {
        ($t:ident : $s1:expr => $s2:expr) => {
            #[test]
            fn $t() {
                assert_eq!($s1.to_train_case(), $s2)
            }
        };
    }

    t!(test1: "CamelCase" => "Camel-Case");
    t!(test2: "This is Human case." => "This-Is-Human-Case");
    t!(test3: "MixedUP CamelCase, with some Spaces" => "Mixed-Up-Camel-Case-With-Some-Spaces");
    t!(test4: "mixed_up_ snake_case with some _spaces" => "Mixed-Up-Snake-Case-With-Some-Spaces");
    t!(test5: "kebab-case" => "Kebab-Case");
    t!(test6: "SHOUTY_SNAKE_CASE" => "Shouty-Snake-Case");
    t!(test7: "snake_case" => "Snake-Case");
    t!(test8: "this-contains_ ALLKinds OfWord_Boundaries" => "This-Contains-All-Kinds-Of-Word-Boundaries");

    t!(test10: "XMLHttpRequest" => "Xml-Http-Request");
    t!(test11: "FIELD_NAME11" => "Field-Name11");
    t!(test12: "99BOTTLES" => "99bottles");
    t!(test13: "FieldNamE11" => "Field-Nam-E11");
    t!(test14: "abc123def456" => "Abc123def456");
    t!(test16: "abc123DEF456" => "Abc123-Def456");
    t!(test17: "abc123Def456" => "Abc123-Def456");
    t!(test18: "abc123DEf456" => "Abc123-D-Ef456");
    t!(test19: "ABC123def456" => "Abc123def456");
    t!(test20: "ABC123DEF456" => "Abc123def456");
    t!(test21: "ABC123Def456" => "Abc123-Def456");
    t!(test22: "ABC123DEf456" => "Abc123d-Ef456");
    t!(test23: "ABC123dEEf456FOO" => "Abc123d-E-Ef456-Foo");
    t!(test24: "abcDEF" => "Abc-Def");
    t!(test25: "ABcDE" => "A-Bc-De");
}
