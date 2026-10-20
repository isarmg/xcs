# Convert Case Extras

Extra patterns, boundaries, cases, and utilities on top of [`convert_case`](https://github.com/rutrum/convert-case).

## Changelog

### convert-case-extras v0.2.0

* `is_case(s, case)`: checks if a string matches a specified case
* `CaseDetector`: a builder-pattern struct for detecting which cases a string matches
    * `CaseDetector::new()`: creates an empty detector
    * `CaseDetector::default()`: creates a detector with all standard cases
    * `.add_case()` / `.add_cases()`: add cases to the detection pool
    * `.remove_case()` / `.remove_cases()`: remove cases from the pool
    * `.detect_cases(s)`: returns all cases from the pool that match the string

tr# convert-case-extras v0.1.0

* `case::TOGGLE`: words start with lower case and remainder are upper case
* `case::ALTERNATING`: words alternate between upper and lower case
* `random` feature:
    * `case::RANDOM`: letters are randomly lower case or upper case
    * `case::PSEUDO_RANDOM`: pairs of letters are randomly lower-upper, or upper-lower
