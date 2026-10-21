#[cfg(test)]
mod tests {
    use core::time::Duration;

    use pki_types::UnixTime;

    use super::*;

    #[test]
    fn basic_pairwise_test() {
        let t = Ticketer::new().unwrap();
        assert!(t.enabled());
        let cipher = t.encrypt(b"hello world").unwrap();
        let plain = t.decrypt(&cipher).unwrap();
        assert_eq!(plain, b"hello world");
    }

    #[test]
    fn refuses_decrypt_before_encrypt() {
        let t = Ticketer::new().unwrap();
        assert_eq!(t.decrypt(b"hello"), None);
    }

    #[test]
    fn refuses_decrypt_larger_than_largest_encryption() {
        let t = Ticketer::new().unwrap();
        let mut cipher = t.encrypt(b"hello world").unwrap();
        assert_eq!(t.decrypt(&cipher), Some(b"hello world".to_vec()));

        // obviously this would never work anyway, but this
        // and `cannot_decrypt_before_encrypt` exercise the
        // first branch in `decrypt()`
        cipher.push(0);
        assert_eq!(t.decrypt(&cipher), None);
    }

    #[test]
    fn ticketrotator_switching_test() {
        let t = Arc::new(crate::ticketer::TicketRotator::new(1, make_ticket_generator).unwrap());
        let now = UnixTime::now();
        let cipher1 = t.encrypt(b"ticket 1").unwrap();
        assert_eq!(t.decrypt(&cipher1).unwrap(), b"ticket 1");
        {
            // Trigger new ticketer
            t.maybe_roll(UnixTime::since_unix_epoch(Duration::from_secs(
                now.as_secs() + 10,
            )));
        }
        let cipher2 = t.encrypt(b"ticket 2").unwrap();
        assert_eq!(t.decrypt(&cipher1).unwrap(), b"ticket 1");
        assert_eq!(t.decrypt(&cipher2).unwrap(), b"ticket 2");
        {
            // Trigger new ticketer
            t.maybe_roll(UnixTime::since_unix_epoch(Duration::from_secs(
                now.as_secs() + 20,
            )));
        }
        let cipher3 = t.encrypt(b"ticket 3").unwrap();
        assert!(t.decrypt(&cipher1).is_none());
        assert_eq!(t.decrypt(&cipher2).unwrap(), b"ticket 2");
        assert_eq!(t.decrypt(&cipher3).unwrap(), b"ticket 3");
    }

    #[test]
    fn ticketrotator_remains_usable_over_temporary_ticketer_creation_failure() {
        let mut t = crate::ticketer::TicketRotator::new(1, make_ticket_generator).unwrap();
        let now = UnixTime::now();
        let cipher1 = t.encrypt(b"ticket 1").unwrap();
        assert_eq!(t.decrypt(&cipher1).unwrap(), b"ticket 1");
        t.generator = fail_generator;
        {
            // Failed new ticketer; this means we still need to
            // rotate.
            t.maybe_roll(UnixTime::since_unix_epoch(Duration::from_secs(
                now.as_secs() + 10,
            )));
        }

        // check post-failure encryption/decryption still works
        let cipher2 = t.encrypt(b"ticket 2").unwrap();
        assert_eq!(t.decrypt(&cipher1).unwrap(), b"ticket 1");
        assert_eq!(t.decrypt(&cipher2).unwrap(), b"ticket 2");

        // do the rotation for real
        t.generator = make_ticket_generator;
        {
            t.maybe_roll(UnixTime::since_unix_epoch(Duration::from_secs(
                now.as_secs() + 20,
            )));
        }
        let cipher3 = t.encrypt(b"ticket 3").unwrap();
        assert!(t.decrypt(&cipher1).is_some());
        assert_eq!(t.decrypt(&cipher2).unwrap(), b"ticket 2");
        assert_eq!(t.decrypt(&cipher3).unwrap(), b"ticket 3");
    }

    #[test]
    fn ticketswitcher_switching_test() {
        #[expect(deprecated)]
        let t = Arc::new(crate::ticketer::TicketSwitcher::new(1, make_ticket_generator).unwrap());
        let now = UnixTime::now();
        let cipher1 = t.encrypt(b"ticket 1").unwrap();
        assert_eq!(t.decrypt(&cipher1).unwrap(), b"ticket 1");
        {
            // Trigger new ticketer
            t.maybe_roll(UnixTime::since_unix_epoch(Duration::from_secs(
                now.as_secs() + 10,
            )));
        }
        let cipher2 = t.encrypt(b"ticket 2").unwrap();
        assert_eq!(t.decrypt(&cipher1).unwrap(), b"ticket 1");
        assert_eq!(t.decrypt(&cipher2).unwrap(), b"ticket 2");
        {
            // Trigger new ticketer
            t.maybe_roll(UnixTime::since_unix_epoch(Duration::from_secs(
                now.as_secs() + 20,
            )));
        }
        let cipher3 = t.encrypt(b"ticket 3").unwrap();
        assert!(t.decrypt(&cipher1).is_none());
        assert_eq!(t.decrypt(&cipher2).unwrap(), b"ticket 2");
        assert_eq!(t.decrypt(&cipher3).unwrap(), b"ticket 3");
    }

    #[test]
    fn ticketswitcher_recover_test() {
        #[expect(deprecated)]
        let mut t = crate::ticketer::TicketSwitcher::new(1, make_ticket_generator).unwrap();
        let now = UnixTime::now();
        let cipher1 = t.encrypt(b"ticket 1").unwrap();
        assert_eq!(t.decrypt(&cipher1).unwrap(), b"ticket 1");
        t.generator = fail_generator;
        {
            // Failed new ticketer
            t.maybe_roll(UnixTime::since_unix_epoch(Duration::from_secs(
                now.as_secs() + 10,
            )));
        }
        t.generator = make_ticket_generator;
        let cipher2 = t.encrypt(b"ticket 2").unwrap();
        assert_eq!(t.decrypt(&cipher1).unwrap(), b"ticket 1");
        assert_eq!(t.decrypt(&cipher2).unwrap(), b"ticket 2");
        {
            // recover
            t.maybe_roll(UnixTime::since_unix_epoch(Duration::from_secs(
                now.as_secs() + 20,
            )));
        }
        let cipher3 = t.encrypt(b"ticket 3").unwrap();
        assert!(t.decrypt(&cipher1).is_none());
        assert_eq!(t.decrypt(&cipher2).unwrap(), b"ticket 2");
        assert_eq!(t.decrypt(&cipher3).unwrap(), b"ticket 3");
    }

    #[test]
    fn aeadticketer_is_debug_and_producestickets() {
        use alloc::format;

        use super::*;

        let t = make_ticket_generator().unwrap();

        let expect = format!("AeadTicketer {{ alg: {TICKETER_AEAD:?}, lifetime: 43200 }}");
        assert_eq!(format!("{t:?}"), expect);
        assert!(t.enabled());
        assert_eq!(t.lifetime(), 43200);
    }

    fn fail_generator() -> Result<Box<dyn ProducesTickets>, GetRandomFailed> {
        Err(GetRandomFailed)
    }
}
