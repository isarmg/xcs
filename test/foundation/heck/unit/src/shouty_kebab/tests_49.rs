#[cfg(test)]
mod tests {
    use super::ToShoutyKebabCase;

    macro_rules! t {
        ($t:ident : $s1:expr => $s2:expr) => {
            #[test]
            fn $t() {
                assert_eq!($s1.to_shouty_kebab_case(), $s2)
            }
        };
    }

    t!(test1: "CamelCase" => "CAMEL-CASE");
    t!(test2: "This is Human case." => "THIS-IS-HUMAN-CASE");
    t!(test3: "MixedUP CamelCase, with some Spaces" => "MIXED-UP-CAMEL-CASE-WITH-SOME-SPACES");
    t!(test4: "mixed_up_ snake_case with some _spaces" => "MIXED-UP-SNAKE-CASE-WITH-SOME-SPACES");
    t!(test5: "kebab-case" => "KEBAB-CASE");
    t!(test6: "SHOUTY_SNAKE_CASE" => "SHOUTY-SNAKE-CASE");
    t!(test7: "snake_case" => "SNAKE-CASE");
    t!(test8: "this-contains_ ALLKinds OfWord_Boundaries" => "THIS-CONTAINS-ALL-KINDS-OF-WORD-BOUNDARIES");
    t!(test9: "XΣXΣ baﬄe" => "XΣXΣ-BAFFLE");
    t!(test10: "XMLHttpRequest" => "XML-HTTP-REQUEST");
    t!(test11: "SHOUTY-KEBAB-CASE" => "SHOUTY-KEBAB-CASE");
}
