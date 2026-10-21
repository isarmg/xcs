#[cfg(test)]
mod test {
    use super::*;
    use crate::Arg;

    #[test]
    fn setting() {
        let m = Arg::new("setting").setting(ArgSettings::Required);
        assert!(m.is_required_set());
    }

    #[test]
    fn unset_setting() {
        let m = Arg::new("unset_setting").setting(ArgSettings::Required);
        assert!(m.is_required_set());

        let m = m.unset_setting(ArgSettings::Required);
        assert!(!m.is_required_set(), "{m:#?}");
    }
}
