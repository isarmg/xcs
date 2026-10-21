#[cfg(test)]
mod tests {
    use crate::ObjectIdentifier;

    use super::rfc4519::CN;

    #[test]
    fn by_oid() {
        let cn = super::DB.by_oid(&CN).expect("cn not found");
        assert_eq!("cn", cn);

        let none = ObjectIdentifier::new_unwrap("0.1.2.3.4.5.6.7.8.9");
        assert_eq!(None, super::DB.by_oid(&none));
    }

    #[test]
    fn by_name() {
        let cn = super::DB.by_name("CN").expect("cn not found");
        assert_eq!(&CN, cn);

        assert_eq!(None, super::DB.by_name("purplePeopleEater"));
    }
}
