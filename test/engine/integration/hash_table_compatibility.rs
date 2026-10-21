//! First-party regression contracts for consolidated hash-table dependencies.
//! Unordered tables preserve contents, not bucket/iteration order. Linked tables
//! preserve their distinct ordering, equality, LRU and ownership semantics.
use hashbrown::{HashMap, HashSet};
use hashlink::{LinkedHashMap, LruCache};
use sqlx_core::common::StatementCache;
use std::{
    collections::BTreeMap,
    hash::{BuildHasherDefault, Hasher},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Default)]
struct CollisionHasher;
impl Hasher for CollisionHasher {
    fn finish(&self) -> u64 {
        0
    }
    fn write(&mut self, _: &[u8]) {}
}
type Collisions = BuildHasherDefault<CollisionHasher>;

#[derive(Debug)]
struct DropCounter(Arc<AtomicUsize>);
impl Drop for DropCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn colliding_keys_survive_lookup_rehash_retain_and_borrowed_entry() {
    let mut map = HashMap::<String, u64, Collisions>::default();
    let mut expected = BTreeMap::new();
    for i in 0..700 {
        map.insert(i.to_string(), i);
        expected.insert(i.to_string(), i);
    }
    for i in (0..700).step_by(3) {
        assert_eq!(
            map.remove(i.to_string().as_str()),
            expected.remove(i.to_string().as_str())
        );
    }
    map.reserve(1000);
    map.shrink_to_fit();
    map.retain(|_, value| *value % 2 == 0);
    expected.retain(|_, value| *value % 2 == 0);
    assert_eq!(map.len(), expected.len());
    for (key, value) in expected {
        assert_eq!(map.get(key.as_str()), Some(&value));
    }
    assert_eq!(map, map.clone());
    assert_eq!(*map.entry_ref("borrowed").or_insert(999), 999);
    assert_eq!(*map.entry_ref("borrowed").or_insert(1000), 999);
    assert_eq!(
        map.raw_entry()
            .from_key("borrowed")
            .map(|(_, value)| *value),
        Some(999)
    );
}

#[test]
fn unordered_equality_and_linked_order_are_distinct_contracts() {
    let mut first = HashMap::<_, _, Collisions>::default();
    let mut second = HashMap::<_, _, Collisions>::default();
    first.extend([(1, 10), (2, 20)]);
    second.extend([(2, 20), (1, 10)]);
    assert_eq!(first, second);
    let a: HashSet<_, Collisions> = [1, 2, 3].into_iter().collect();
    let b: HashSet<_, Collisions> = [3, 2, 1].into_iter().collect();
    assert_eq!(a, b);
    let mut first = LinkedHashMap::<_, _, Collisions>::with_hasher(Default::default());
    let mut second = LinkedHashMap::<_, _, Collisions>::with_hasher(Default::default());
    first.extend([(1, 10), (2, 20)]);
    second.extend([(2, 20), (1, 10)]);
    assert_ne!(
        first, second,
        "linked equality is deliberately order-sensitive"
    );
    second.to_front(&1);
    assert_eq!(first, second);
}

#[test]
fn linked_order_survives_collisions_and_replacement() {
    let mut map = LinkedHashMap::<String, i32, Collisions>::with_hasher(Default::default());
    for i in 0..100 {
        map.insert(i.to_string(), i);
    }
    assert_eq!(map.insert("0".into(), 101), Some(0));
    map.to_front("50");
    map.to_back("1");
    let keys: Vec<_> = map.keys().map(|s| s.parse::<i32>().unwrap()).collect();
    let mut expected = vec![50];
    expected.extend((2..100).filter(|key| *key != 50));
    expected.extend([0, 1]);
    assert_eq!(keys, expected);
    assert_eq!(map.remove("50"), Some(50));
    assert_eq!(map.pop_front(), Some(("2".into(), 2)));
}

#[test]
fn lru_reads_touch_but_membership_checks_do_not() {
    let mut cache = LruCache::<String, i32, Collisions>::with_hasher(2, Default::default());
    cache.insert("a".into(), 1);
    cache.insert("b".into(), 2);
    assert!(cache.contains_key("a"));
    cache.insert("c".into(), 3);
    assert!(
        !cache.contains_key("a"),
        "membership must not change recency"
    );
    assert_eq!(cache.get_mut("b"), Some(&mut 2));
    cache.insert("d".into(), 4);
    assert!(!cache.contains_key("c"));
    assert_eq!(cache.remove_lru(), Some(("b".into(), 2)));
    assert_eq!(cache.remove_lru(), Some(("d".into(), 4)));
    assert!(cache.is_empty());
}

#[test]
fn collision_removal_clear_and_lru_eviction_drop_values_once() {
    let drops = Arc::new(AtomicUsize::new(0));
    {
        let mut map =
            LinkedHashMap::<usize, DropCounter, Collisions>::with_hasher(Default::default());
        for i in 0..200 {
            map.insert(i, DropCounter(drops.clone()));
        }
        for i in 0..50 {
            map.remove(&i);
        }
        assert_eq!(drops.load(Ordering::SeqCst), 50);
        map.insert(100, DropCounter(drops.clone()));
        assert_eq!(drops.load(Ordering::SeqCst), 51);
        map.clear();
        assert_eq!(drops.load(Ordering::SeqCst), 201);
        map.insert(1, DropCounter(drops.clone()));
    }
    assert_eq!(drops.load(Ordering::SeqCst), 202);
    let drops = Arc::new(AtomicUsize::new(0));
    {
        let mut map = HashMap::<usize, DropCounter, Collisions>::default();
        for i in 0..200 {
            map.insert(i, DropCounter(drops.clone()));
        }
        map.retain(|key, _| key % 2 == 0);
        assert_eq!(drops.load(Ordering::SeqCst), 100);
        map.clear();
    }
    assert_eq!(drops.load(Ordering::SeqCst), 200);
    let drops = Arc::new(AtomicUsize::new(0));
    {
        let mut cache =
            LruCache::<usize, DropCounter, Collisions>::with_hasher(1, Default::default());
        cache.insert(1, DropCounter(drops.clone()));
        cache.insert(2, DropCounter(drops.clone()));
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        cache.insert(2, DropCounter(drops.clone()));
        assert_eq!(drops.load(Ordering::SeqCst), 2);
    }
    assert_eq!(drops.load(Ordering::SeqCst), 3);
}

#[test]
fn sql_statement_cache_preserves_eviction_replacement_and_disabled_behavior() {
    let mut cache = StatementCache::new(2);
    assert_eq!(cache.insert("select a", 1), None);
    assert_eq!(cache.insert("select b", 2), None);
    assert_eq!(cache.get_mut("select a"), Some(&mut 1));
    assert_eq!(cache.insert("select c", 3), Some(2));
    assert_eq!(cache.insert("select a", 4), Some(1));
    assert_eq!(cache.remove_lru(), Some(3));
    assert_eq!(cache.remove_lru(), Some(4));
    assert!(cache.is_empty());
    let drops = Arc::new(AtomicUsize::new(0));
    let mut disabled = StatementCache::new(0);
    assert!(!disabled.is_enabled());
    disabled.insert("select 1", DropCounter(drops.clone()));
    assert_eq!(disabled.len(), 0);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
