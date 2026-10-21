use std::mem;

use ahash::AHashMap;

pub(crate) enum SmallMap<K, V, const N: usize = 2> {
    Small(micromap::Map<K, V, N>),
    Large(AHashMap<K, V>),
}

impl<K, V, const N: usize> SmallMap<K, V, N> {
    #[inline]
    pub(crate) fn new() -> Self {
        SmallMap::Small(micromap::Map::new())
    }

    #[inline]
    pub(crate) fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q> + Eq + std::hash::Hash,
        Q: std::hash::Hash + Eq + ?Sized,
    {
        match self {
            SmallMap::Small(map) => map.get(key),
            SmallMap::Large(map) => map.get(key),
        }
    }

    #[inline]
    pub(crate) fn insert(&mut self, key: K, value: V)
    where
        K: Eq + std::hash::Hash,
    {
        match self {
            SmallMap::Small(map) => {
                // Fits inline (new key with space) or overwrites existing key.
                if map.len() < N || map.get(&key).is_some() {
                    map.insert(key, value);
                    return;
                }
                // Full and key is new — fall through to promotion.
            }
            SmallMap::Large(map) => {
                map.insert(key, value);
                return;
            }
        }
        // Promotion: atomically swap self to Large, drain old Small into it.
        let old = match mem::replace(self, SmallMap::Large(AHashMap::with_capacity(N + 1))) {
            SmallMap::Small(m) => m,
            SmallMap::Large(_) => unreachable!(),
        };
        if let SmallMap::Large(new_map) = self {
            for (k, v) in old {
                new_map.insert(k, v);
            }
            new_map.insert(key, value);
        }
    }

    #[inline]
    pub(crate) fn get_or_insert_default(&mut self, key: K) -> &mut V
    where
        K: Eq + std::hash::Hash,
        V: Default,
    {
        // Determine whether we need to promote before borrowing map contents.
        let needs_promotion = match self {
            SmallMap::Small(map) => map.len() >= N && map.get(&key).is_none(),
            SmallMap::Large(_) => false,
        };
        if needs_promotion {
            // Promotion (same pattern as insert).
            let old = match mem::replace(self, SmallMap::Large(AHashMap::with_capacity(N + 1))) {
                SmallMap::Small(m) => m,
                SmallMap::Large(_) => unreachable!(),
            };
            if let SmallMap::Large(new_map) = self {
                for (k, v) in old {
                    new_map.insert(k, v);
                }
                return new_map.entry(key).or_default();
            }
            unreachable!()
        }
        match self {
            SmallMap::Small(map) => map.entry(key).or_default(),
            SmallMap::Large(map) => map.entry(key).or_default(),
        }
    }
}

impl<K, V, const N: usize> Default for SmallMap<K, V, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Clone, V: Clone, const N: usize> Clone for SmallMap<K, V, N> {
    fn clone(&self) -> Self {
        match self {
            SmallMap::Small(map) => SmallMap::Small(map.clone()),
            SmallMap::Large(map) => SmallMap::Large(map.clone()),
        }
    }
}

impl<K: std::fmt::Debug, V: std::fmt::Debug, const N: usize> std::fmt::Debug for SmallMap<K, V, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmallMap::Small(map) => write!(f, "{map:?}"),
            SmallMap::Large(map) => write!(f, "{map:?}"),
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/referencing/unit/src/small_map/tests_116.rs"));
