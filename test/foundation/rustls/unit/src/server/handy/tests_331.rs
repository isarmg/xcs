#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use crate::server::{ProducesTickets, StoresServerSessions};

    #[test]
    fn test_noserversessionstorage_drops_put() {
        let c = NoServerSessionStorage {};
        assert!(!c.put(vec![0x01], vec![0x02]));
    }

    #[test]
    fn test_noserversessionstorage_denies_gets() {
        let c = NoServerSessionStorage {};
        c.put(vec![0x01], vec![0x02]);
        assert_eq!(c.get(&[]), None);
        assert_eq!(c.get(&[0x01]), None);
        assert_eq!(c.get(&[0x02]), None);
    }

    #[test]
    fn test_noserversessionstorage_denies_takes() {
        let c = NoServerSessionStorage {};
        assert_eq!(c.take(&[]), None);
        assert_eq!(c.take(&[0x01]), None);
        assert_eq!(c.take(&[0x02]), None);
    }

    #[test]
    fn test_neverproducestickets_does_nothing() {
        let npt = NeverProducesTickets {};
        assert!(!npt.enabled());
        assert_eq!(0, npt.lifetime());
        assert_eq!(None, npt.encrypt(&[]));
        assert_eq!(None, npt.decrypt(&[]));
    }
}
