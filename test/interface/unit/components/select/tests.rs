use super::*;
#[test]
fn keyboard_navigation_wraps_and_skips_disabled_choices() {
    let mut options = vec![
        SelectOption::new("", "Default"),
        SelectOption::new("old", "Unavailable"),
        SelectOption::new("new", "New"),
    ];
    options[1].disabled = true;
    assert_eq!(next_enabled(&options, None, true), Some(0));
    assert_eq!(next_enabled(&options, Some(0), true), Some(2));
    assert_eq!(next_enabled(&options, Some(2), true), Some(0));
    assert_eq!(next_enabled(&options, Some(0), false), Some(2));
    assert_eq!(next_enabled(&options, None, false), Some(2));
    assert_eq!(next_enabled(&options, Some(99), true), Some(0));
}
#[test]
fn empty_and_fully_disabled_lists_have_no_active_choice() {
    assert_eq!(next_enabled(&[], None, true), None);
    let options = vec![SelectOption {
        value: String::new(),
        label: "Disabled".into(),
        disabled: true,
    }];
    assert_eq!(next_enabled(&options, None, true), None);
    assert_eq!(next_enabled(&options, None, false), None);
}
