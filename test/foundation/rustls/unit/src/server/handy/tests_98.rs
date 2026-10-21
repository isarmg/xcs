#[cfg(test)]
    mod tests {
        use std::vec;

        use super::*;
        use crate::server::StoresServerSessions;

        #[test]
        fn test_serversessionmemorycache_accepts_put() {
            let c = ServerSessionMemoryCache::new(4);
            assert!(c.put(vec![0x01], vec![0x02]));
        }

        #[test]
        fn test_serversessionmemorycache_persists_put() {
            let c = ServerSessionMemoryCache::new(4);
            assert!(c.put(vec![0x01], vec![0x02]));
            assert_eq!(c.get(&[0x01]), Some(vec![0x02]));
            assert_eq!(c.get(&[0x01]), Some(vec![0x02]));
        }

        #[test]
        fn test_serversessionmemorycache_overwrites_put() {
            let c = ServerSessionMemoryCache::new(4);
            assert!(c.put(vec![0x01], vec![0x02]));
            assert!(c.put(vec![0x01], vec![0x04]));
            assert_eq!(c.get(&[0x01]), Some(vec![0x04]));
        }

        /// After five sessions are put into a cache of size 2, exactly the
        /// two most recently added sessions remain.
        #[test]
        fn test_serversessionmemorycache_drops_to_maintain_size_invariant() {
            let c = ServerSessionMemoryCache::new(2);
            assert!(c.put(vec![0x01], vec![0x02]));
            assert!(c.put(vec![0x03], vec![0x04]));
            assert!(c.put(vec![0x05], vec![0x06]));
            assert!(c.put(vec![0x07], vec![0x08]));
            assert!(c.put(vec![0x09], vec![0x0a]));

            assert_eq!(c.get(&[0x01]), None);
            assert_eq!(c.get(&[0x03]), None);
            assert_eq!(c.get(&[0x05]), None);
            assert_eq!(c.get(&[0x07]), Some(vec![0x08]));
            assert_eq!(c.get(&[0x09]), Some(vec![0x0a]));
        }

        /// A cache of size 1 keeps the session put into it, and a second
        /// put replaces it.
        #[test]
        fn test_serversessionmemorycache_size_one_retains_session() {
            let c = ServerSessionMemoryCache::new(1);
            assert!(c.put(vec![0x01], vec![0x02]));
            assert_eq!(c.get(&[0x01]), Some(vec![0x02]));

            assert!(c.put(vec![0x03], vec![0x04]));
            assert_eq!(c.get(&[0x01]), None);
            assert_eq!(c.get(&[0x03]), Some(vec![0x04]));
        }
    }
