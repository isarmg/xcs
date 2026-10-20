//! Extra utilities for [`convert_case`].
//!
//! ```
//! use convert_case::Casing;
//! use convert_case_extras::case;
//!
//! assert_eq!(
//!     "toggle_case_word".to_case(case::TOGGLE),
//!     "tOGGLE cASE wORD",
//! )
//! ```
//!
//! ## Random Feature
//!
//! The `random` feature contains `case::RANDOM` and `case::PSEUDO_RANDOM`.

use convert_case::{Boundary, Case, Casing, Pattern};

#[cfg(feature = "random")]
use rand::prelude::*;

/// Checks if a string matches the specified case.
///
/// A string matches a case if converting it to that case produces
/// the same string (i.e., `s.to_case(case) == s`).
///
/// # Example
/// ```
/// use convert_case::Case;
/// use convert_case_extras::is_case;
///
/// assert!(is_case("hello_world", Case::Snake));
/// assert!(!is_case("hello_world", Case::Kebab));
/// assert!(is_case("HelloWorld", Case::Pascal));
/// ```
pub fn is_case<T: AsRef<str>>(s: T, case: Case) -> bool {
    s.as_ref() == s.as_ref().to_case(case)
}

/// A detector for determining which cases a string matches.
///
/// `CaseDetector` maintains a pool of cases and provides a method
/// to detect which cases from the pool match a given string.
///
/// # Example
/// ```
/// use convert_case::Case;
/// use convert_case_extras::CaseDetector;
///
/// // Default detector with all standard cases
/// let detector = CaseDetector::default();
/// let matches = detector.detect_cases("my_variable_name");
/// assert!(matches.contains(&Case::Snake));
///
/// // Custom detector with specific cases
/// let detector = CaseDetector::new()
///     .add_case(Case::Snake)
///     .add_case(Case::Kebab);
/// let matches = detector.detect_cases("hello-world");
/// assert_eq!(matches, vec![Case::Kebab]);
/// ```
#[derive(Debug, Clone)]
pub struct CaseDetector {
    cases: Vec<Case<'static>>,
}

impl CaseDetector {
    /// Creates a new `CaseDetector` with an empty pool.
    ///
    /// Use builder methods like `add_case` to populate the pool.
    /// 
    /// Use `default` instead to use all of the cases available in `convert-case`.
    ///
    /// # Example
    /// ```
    /// use convert_case::Case;
    /// use convert_case_extras::CaseDetector;
    ///
    /// let detector = CaseDetector::new()
    ///     .add_case(Case::Snake)
    ///     .add_case(Case::Kebab);
    /// ```
    pub fn new() -> Self {
        Self { cases: Vec::new() }
    }

    /// Adds a case to the pool. Returns self for method chaining.
    ///
    /// # Example
    /// ```
    /// use convert_case::Case;
    /// use convert_case_extras::CaseDetector;
    ///
    /// let detector = CaseDetector::new()
    ///     .add_case(Case::Snake)
    ///     .add_case(Case::Kebab);
    /// ```
    pub fn add_case(mut self, case: Case<'static>) -> Self {
        self.cases.push(case);
        self
    }

    /// Adds multiple cases to the pool. Returns self for method chaining.
    ///
    /// # Example
    /// ```
    /// use convert_case::Case;
    /// use convert_case_extras::CaseDetector;
    ///
    /// let detector = CaseDetector::new()
    ///     .add_cases(&[Case::Snake, Case::Kebab, Case::Camel]);
    /// ```
    pub fn add_cases(mut self, cases: &[Case<'static>]) -> Self {
        self.cases.extend(cases.iter().copied());
        self
    }

    /// Removes a case from the pool. Returns self for method chaining.
    ///
    /// # Example
    /// ```
    /// use convert_case::Case;
    /// use convert_case_extras::CaseDetector;
    ///
    /// let detector = CaseDetector::default()
    ///     .remove_case(Case::Flat)
    ///     .remove_case(Case::UpperFlat);
    /// ```
    pub fn remove_case(mut self, case: Case<'static>) -> Self {
        self.cases.retain(|&c| c != case);
        self
    }

    /// Removes multiple cases from the pool. Returns self for method chaining.
    ///
    /// # Example
    /// ```
    /// use convert_case::Case;
    /// use convert_case_extras::CaseDetector;
    ///
    /// let detector = CaseDetector::default()
    ///     .remove_cases(&[Case::Flat, Case::UpperFlat]);
    /// ```
    pub fn remove_cases(mut self, cases: &[Case<'static>]) -> Self {
        for case in cases {
            self.cases.retain(|&c| c != *case);
        }
        self
    }

    /// Detects all cases from the pool that the given string matches.
    ///
    /// A string "matches" a case if converting it to that case produces
    /// the same string (i.e., `s.to_case(case) == s`).
    ///
    /// # Example
    /// ```
    /// use convert_case::Case;
    /// use convert_case_extras::CaseDetector;
    ///
    /// let detector = CaseDetector::default();
    ///
    /// let matches = detector.detect_cases("hello_world");
    /// assert!(matches.contains(&Case::Snake));
    /// assert!(!matches.contains(&Case::Kebab));
    ///
    /// // Single lowercase word matches multiple cases
    /// let matches = detector.detect_cases("word");
    /// assert!(matches.contains(&Case::Snake));
    /// assert!(matches.contains(&Case::Kebab));
    /// assert!(matches.contains(&Case::Flat));
    /// ```
    pub fn detect_cases<T: AsRef<str>>(&self, s: T) -> Vec<Case<'static>> {
        let s = s.as_ref();
        self.cases
            .iter()
            .filter(|&&case| is_case(s, case))
            .copied()
            .collect()
    }
}

impl Default for CaseDetector {
    /// Creates a `CaseDetector` with all standard cases from `Case::all_cases()`.
    fn default() -> Self {
        Self {
            cases: Case::all_cases().to_vec(),
        }
    }
}

pub mod pattern {
    use super::*;

    /// Makes the first letter of each word lowercase
    /// and the remaining letters of each word uppercase.
    /// ```
    /// use convert_case_extras::pattern;
    ///
    /// assert_eq!(
    ///     pattern::TOGGLE.mutate(&["Case", "CONVERSION", "library"]),
    ///     vec!["cASE", "cONVERSION", "lIBRARY"],
    /// );
    /// ```
    pub const TOGGLE: Pattern = Pattern::Custom(|words| {
        words
            .iter()
            .map(|word| {
                let mut chars = word.chars();

                if let Some(c) = chars.next() {
                    [c.to_lowercase().collect(), chars.as_str().to_uppercase()].concat()
                } else {
                    String::new()
                }
            })
            .collect()
    });

    /// Makes each letter of each word alternate between lowercase and uppercase.
    ///
    /// It alternates across words,
    /// which means the last letter of one word and the first letter of the
    /// next will not be the same letter casing.
    /// ```
    /// use convert_case_extras::pattern;
    ///
    /// assert_eq!(
    ///     pattern::ALTERNATING.mutate(&["Case", "CONVERSION", "library"]),
    ///     vec!["cAsE", "cOnVeRsIoN", "lIbRaRy"],
    /// );
    /// assert_eq!(
    ///     pattern::ALTERNATING.mutate(&["Another", "Example"]),
    ///     vec!["aNoThEr", "ExAmPlE"],
    /// );
    /// ```
    pub const ALTERNATING: Pattern = Pattern::Custom(|words| {
        let mut upper = false;
        words
            .iter()
            .map(|word| {
                word.chars()
                    .map(|letter| {
                        if letter.is_uppercase() || letter.is_lowercase() {
                            if upper {
                                upper = false;
                                letter.to_uppercase().to_string()
                            } else {
                                upper = true;
                                letter.to_lowercase().to_string()
                            }
                        } else {
                            letter.to_string()
                        }
                    })
                    .collect()
            })
            .collect()
    });

    // #[doc(cfg(feature = "random"))]
    /// Lowercases or uppercases each letter uniformly randomly.
    ///
    /// This uses the `rand` crate and is only available with the "random" feature.
    /// ```
    /// # #[cfg(any(doc, feature = "random"))]
    /// use convert_case_extras::pattern;
    ///
    /// pattern::RANDOM.mutate(&["Case", "CONVERSION", "library"]);
    /// // "casE", "coNVeRSiOn", "lIBraRY"
    /// ```
    #[cfg(feature = "random")]
    pub const RANDOM: Pattern = Pattern::Custom(|words| {
        let mut rng = rand::thread_rng();
        words
            .iter()
            .map(|word| {
                word.chars()
                    .map(|letter| {
                        if rng.gen::<f32>() > 0.5 {
                            letter.to_uppercase().to_string()
                        } else {
                            letter.to_lowercase().to_string()
                        }
                    })
                    .collect()
            })
            .collect()
    });

    /// Case each letter in random-like patterns.
    ///
    /// Instead of randomizing
    /// each letter individually, it mutates each pair of characters
    /// as either (Lowercase, Uppercase) or (Uppercase, Lowercase).  This generates
    /// more "random looking" words.  A consequence of this algorithm for randomization
    /// is that there will never be three consecutive letters that are all lowercase
    /// or all uppercase.  This uses the `rand` crate and is only available with the "random"
    /// feature.
    ///
    /// This uses the `rand` crate and is only available with the "random" feature.
    /// ```
    /// # #[cfg(any(doc, feature = "random"))]
    /// use convert_case_extras::pattern;
    ///
    /// pattern::PSEUDO_RANDOM.mutate(&["Case", "CONVERSION", "library"]);
    /// // "cAsE", "cONveRSioN", "lIBrAry"
    /// ```
    #[cfg(feature = "random")]
    pub const PSEUDO_RANDOM: Pattern = Pattern::Custom(|words| {
        let mut rng = rand::thread_rng();

        // Keeps track of when to alternate
        let mut alt: Option<bool> = None;
        words
            .iter()
            .map(|word| {
                word.chars()
                    .map(|letter| {
                        match alt {
                            // No existing pattern, start one
                            None => {
                                if rng.gen::<f32>() > 0.5 {
                                    alt = Some(false); // Make the next char lower
                                    letter.to_uppercase().to_string()
                                } else {
                                    alt = Some(true); // Make the next char upper
                                    letter.to_lowercase().to_string()
                                }
                            }
                            // Existing pattern, do what it says
                            Some(upper) => {
                                alt = None;
                                if upper {
                                    letter.to_uppercase().to_string()
                                } else {
                                    letter.to_lowercase().to_string()
                                }
                            }
                        }
                    })
                    .collect()
            })
            .collect()
    });
}

pub mod case {
    use super::*;

    /// Toggle case strings are delimited by spaces.  All characters are uppercase except
    /// for the leading character of each word, which is lowercase.
    /// * Boundaries: [Space](`Boundary::Space`)
    /// * Pattern: [Toggle](`pattern::TOGGLE`)
    /// * Delimiter: Space `" "`
    ///
    /// ```
    /// use convert_case::Casing;
    /// use convert_case_extras::case;
    /// assert_eq!("My variable NAME".to_case(case::TOGGLE), "mY vARIABLE nAME");
    /// ```
    pub const TOGGLE: Case = Case::Custom {
        boundaries: &[Boundary::Space],
        pattern: pattern::TOGGLE,
        delimiter: " ",
    };

    /// Alternating case strings are delimited by spaces.  Characters alternate between uppercase
    /// and lowercase.
    /// * Boundaries: [Space](Boundary::Space)
    /// * Pattern: [Alternating](Pattern::Alternating)
    /// * Delimiter: Space `" "`
    ///
    /// ```
    /// use convert_case::Casing;
    /// use convert_case_extras::case;
    /// assert_eq!("My variable NAME".to_case(case::ALTERNATING), "mY vArIaBlE nAmE");
    /// ```
    pub const ALTERNATING: Case = Case::Custom {
        boundaries: &[Boundary::Space],
        pattern: pattern::ALTERNATING,
        delimiter: " ",
    };

    /// Random case strings are delimited by spaces and characters are
    /// randomly upper case or lower case.
    ///
    /// This uses the `rand` crate
    /// and is only available with the "random" feature.
    /// * Boundaries: [Space](Boundary::Space)
    /// * Pattern: [Random](pattern::RANDOM)
    /// * Delimiter: Space `" "`
    ///
    /// ```
    /// use convert_case::Casing;
    /// use convert_case_extras::case;
    /// "My variable NAME".to_case(case::RANDOM);
    /// // "My vaRIAbLE nAme"
    /// ```
    #[cfg(any(doc, feature = "random"))]
    #[cfg(feature = "random")]
    pub const RANDOM: Case = Case::Custom {
        boundaries: &[Boundary::Space],
        pattern: pattern::RANDOM,
        delimiter: " ",
    };

    /// Pseudo-random case strings are delimited by spaces and characters are randomly
    /// upper case or lower case, but there will never more than two consecutive lower
    /// case or upper case letters in a row.
    ///
    /// This uses the `rand` crate and is
    /// only available with the "random" feature.
    /// * Boundaries: [Space](Boundary::Space)
    /// * Pattern: [Pseudo random](pattern::PSEUDO_RANDOM)
    /// * Delimiter: Space `" "`
    ///
    /// ```
    /// use convert_case::Casing;
    /// use convert_case_extras::case;
    /// let new = "My variable NAME".to_case(case::PSEUDO_RANDOM);
    /// ```
    /// String `new` could be "mY vArIAblE NamE" for example.
    #[cfg(any(doc, feature = "random"))]
    #[cfg(feature = "random")]
    pub const PSEUDO_RANDOM: Case = Case::Custom {
        boundaries: &[Boundary::Space],
        pattern: pattern::PSEUDO_RANDOM,
        delimiter: " ",
    };
}

#[cfg(test)]
mod test {
    use super::*;

    use convert_case::Casing;

    #[test]
    fn toggle_case() {
        assert_eq!("test_toggle".to_case(case::TOGGLE), "tEST tOGGLE");
    }

    #[cfg(feature = "random")]
    #[test]
    fn pseudo_no_triples() {
        let words = vec!["abcdefg", "hijklmnop", "qrstuv", "wxyz"];
        for _ in 0..5 {
            let new = pattern::PSEUDO_RANDOM.mutate(&words).join("");
            let mut iter = new
                .chars()
                .zip(new.chars().skip(1))
                .zip(new.chars().skip(2));
            assert!(!iter
                .clone()
                .any(|((a, b), c)| a.is_lowercase() && b.is_lowercase() && c.is_lowercase()));
            assert!(
                !iter.any(|((a, b), c)| a.is_uppercase() && b.is_uppercase() && c.is_uppercase())
            );
        }
    }

    #[cfg(feature = "random")]
    #[test]
    fn randoms_are_random() {
        let words = vec!["abcdefg", "hijklmnop", "qrstuv", "wxyz"];

        for _ in 0..5 {
            let transformed = pattern::PSEUDO_RANDOM.mutate(&words);
            assert_ne!(words, transformed);
            let transformed = pattern::RANDOM.mutate(&words);
            assert_ne!(words, transformed);
        }
    }
}
