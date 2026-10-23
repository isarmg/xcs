// SPDX-FileCopyrightText: Copyright (c) 2023-2026 Yegor Bugayenko
// SPDX-FileCopyrightText: Copyright (c) 2025 owtotwo
// SPDX-License-Identifier: MIT

use super::Map;
use core::borrow::Borrow;

impl<K, V, const N: usize> Map<K, V, N> {
    /// Returns the number of key-value pairs the [Map] can hold,
    /// which always equal to `N`.
    ///
    /// Note that the number of the inserted pairs (with difference keys)
    /// should not exceed this value.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// const N: usize = 3;
    /// let mut m = Map::<_, _, N>::new();
    /// m.insert(1, "a");
    /// assert_eq!(m.capacity(), N);
    /// assert_eq!(m.len(), 1);
    /// ```
    #[inline]
    #[must_use]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Returns `true` if the map contains no key-value pair.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut m = Map::<_, _, 3>::new();
    /// assert!(m.is_empty());
    /// m.insert(1, "a");
    /// assert!(!m.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of key-value pairs in the map.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut m = Map::<_, _, 3>::new();
    /// assert_eq!(m.len(), 0);
    /// m.insert(1, "a");
    /// assert_eq!(m.len(), 1);
    /// ```
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Clears the map, removing all key-value pairs (drop them).
    ///
    /// But keeps the memory that was occupied when creating the [Map], that is, will not
    /// release any memory usage.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut m = Map::<_, _, 3>::new();
    /// m.insert(1, "a");
    /// m.clear();
    /// assert!(m.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        for i in 0..self.len {
            unsafe { self.item_drop(i) };
        }
        self.len = 0;
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all pairs `(k, v)` for which `f(&k, &mut v)`
    /// returns false. The elements are visited in unsorted (and unspecified)
    /// order.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut map: Map<_, _, 8> = Map::from_iter((0..8).map(|x| (x, x*10)));
    /// map.retain(|&k, _| k % 2 == 0);
    /// assert_eq!(map.len(), 4);
    /// ```
    ///
    /// # Performance
    /// In the current implementation, this operation takes O(len) time.
    #[inline]
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut f: F) {
        let mut i = 0;
        while i < self.len {
            let p = unsafe { self.item_mut(i) };
            if f(&p.0, &mut p.1) {
                // do not remove -> next index
                i += 1;
            } else {
                unsafe { self.remove_index_drop(i) };
                // recheck the same index
            }
        }
    }
}

impl<K: PartialEq, V, const N: usize> Map<K, V, N> {
    /// Returns `true` if the map contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the map’s key type, but
    /// [`PartialEq`] on the borrowed form must match those for the key
    /// type.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut m: Map<_, _, 3> = Map::new();
    /// m.insert(1, "a");
    /// assert_eq!(m.contains_key(&1), true);
    /// assert_eq!(m.contains_key(&2), false);
    /// ```
    #[inline]
    #[must_use]
    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: PartialEq + ?Sized,
    {
        self.iter().any(|(x, _)| x.borrow() == k)
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    ///
    /// The key may be any borrowed form of the map’s key type, but
    /// [`PartialEq`] on the borrowed form must match those for the key
    /// type.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut m: Map<_, _, 3> = Map::new();
    /// m.insert(1, "a");
    /// assert_eq!(m.remove(&1), Some("a"));
    /// assert_eq!(m.remove(&1), None);
    /// ```
    #[inline]
    pub fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: PartialEq + ?Sized,
    {
        let (i, _) = self.pairs[..self.len]
            .iter()
            .enumerate()
            .find(|(_, p)| unsafe { p.assume_init_ref() }.0.borrow() == k)?;
        Some(unsafe { self.remove_index_read(i).1 })
    }

    /// Insert a single key-value pair into the map.
    ///
    /// If the map did not have this key present, [None] is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned. The key is not updated, though; this matters for
    /// types that can be `==` without being identical. See the [module-level
    /// documentation] for more.
    ///
    /// If you want to update the key as well, use and check the documentation of
    /// [`insert_key_value()`][Self::insert_key_value].
    ///
    /// [module-level documentation]: std::collections#insert-and-complex-keys
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut map: Map<_, _, 3> = Map::new();
    /// assert_eq!(map.insert(37, "a"), None);
    /// assert_eq!(map.is_empty(), false);
    /// map.insert(37, "b");
    /// assert_eq!(map.insert(37, "c"), Some("b"));
    /// assert_eq!(map[&37], "c");
    /// ```
    ///
    /// # Panics
    /// It may panic if there are too many pairs in the map already. If you
    /// want to avoid this, use [`checked_insert()`][Self::checked_insert] instead.
    ///
    /// In order to comply with the memory safety of the Rust language itself, it
    /// will perform bounds checking, whether in `debug` mode or `release` mode.
    ///
    /// About performance, because the implementation of this [`insert()`][Self::insert]
    /// mainly uses iterators instead of loops, it is not much slower in practice.
    /// It is even faster when frequently inserting and replacing pairs of
    /// existing keys which already in set.
    #[inline]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        let (_, existing_pair) = self.insert_ii(k, v, false);
        existing_pair.map(|(_, v)| v)
    }

    /// Attempt to insert a pair into the map. (no panic)
    ///
    /// - If the key exists, whether the map is full or not, we update the
    ///   value (exclude key), and return `Some(Some(old_value))`;
    /// - If the key does not exist and the map is full already, we can do
    ///   nothing, so just return `None`;
    /// - If the key does not exist and the map has empty slot, we insert
    ///   into that slot and return `Some(None)`.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut m: Map<_, _, 3> = Map::new();
    /// // For `Some(None)`, `Some(_)` indicates that the insertion was successful, `None`
    /// // in the former means that the inserted key was not in map, that is, insert
    /// // the key-value pair. Otherwise, when returning to the latter, only value will be
    /// // update.
    /// assert_eq!(m.checked_insert(1, "a"), Some(None));
    /// assert_eq!(m.checked_insert(1, "A"), Some(Some("a")));
    /// assert_eq!(m.checked_insert(2, "b"), Some(None));
    /// assert_eq!(m.checked_insert(3, "c"), Some(None));
    /// assert_eq!(m.len(), m.capacity());
    /// // map is full now.
    /// assert_eq!(m.checked_insert(2, "B"), Some(Some("b")));
    /// // This insertion will cause capacity overflow, so no insertion is performed
    /// // and `None` is returned.
    /// assert_eq!(m.checked_insert(4, "d"), None);
    /// ```
    #[inline]
    pub fn checked_insert(&mut self, k: K, v: V) -> Option<Option<V>> {
        if self.len < N {
            Some(self.insert_ii(k, v, false).1.map(|(_, v)| v))
        } else {
            self.insert_ii_for_full(k, v, false)
                .map(|(_, (_, v))| Some(v))
        }
    }

    /// Insert a single key-value pair into the map, updating the key as well.
    ///
    /// If the map did not have this key present, [`None`] is returned, which
    /// means the insertion successful.
    ///
    /// If the map did have this key present, the key-value pair is updated, and
    /// the old key-value pair is returned. Note that unlike
    /// [`insert()`][Self::insert], this method updates both key and value.
    /// This matters for types that can be `==` without being identical.
    /// See the [module-level documentation] for more.
    ///
    /// [module-level documentation]: std::collections#insert-and-complex-keys
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// // Only the first element in the tuple is considered when
    /// // determining whether two `Foo`s are equal.
    /// #[derive(Debug)]
    /// struct Foo(u8, f32);
    /// impl PartialEq for Foo {
    ///    fn eq(&self, other: &Self) -> bool {
    ///       self.0 == other.0
    ///     }
    /// }
    /// // Not necessary
    /// impl Eq for Foo {}
    /// // Use Foo as a key
    /// let mut map: Map<Foo, char, 3> = Map::new();
    /// assert_eq!(map.insert_key_value(Foo(b'a', 0.123), 'a'), None);
    /// assert_eq!(map.len(), 1);
    /// assert_eq!(map.insert_key_value(Foo(b'b', 0.456), 'b'), None);
    /// assert_eq!(map.len(), 2);
    /// // Note that the f32 in `Foo` is meaningless here.
    /// assert_eq!(map.insert_key_value(Foo(b'b', 0.789), 'B'), Some((Foo(b'b', 0.456), 'b')));
    /// assert_eq!(map[&Foo(b'b', 3.1416)], 'B');
    /// assert_eq!(map.get_key_value(&Foo(b'b', 0.123)).unwrap().0.1, 0.789);
    /// ```
    #[inline]
    pub fn insert_key_value(&mut self, k: K, v: V) -> Option<(K, V)> {
        let (_, existing_pair) = self.insert_ii(k, v, true);
        existing_pair
    }

    /// Insert a single pair into the map without bound check in release mode.
    ///
    /// # Panics
    /// It may panic if there are too many pairs in the map already. Pay attention,
    /// it panics only in the `debug` mode. In the `release` mode, you are going to get
    /// **undefined behavior**. This is done for the sake of performance, in order to
    /// avoid a repetitive check for the boundary condition on every `insert()`.
    ///
    /// # Safety
    /// Calling this method to add a new key-value pair when the [`Map`] is already
    /// full is undefined behavior instead of panic. So you need to make sure that
    /// the map is not full before calling.
    #[inline]
    pub unsafe fn insert_unchecked(&mut self, k: K, v: V) -> Option<V> {
        let (_, existing_pair) = self.insert_i(k, v, false);
        existing_pair.map(|(_, v)| v)
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map’s key type, but
    /// [`PartialEq`] on the borrowed form must match those for the key
    /// type.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut m: Map<_, _, 3> = Map::new();
    /// m.insert(1, "a");
    /// assert_eq!(m.get(&1), Some(&"a"));
    /// assert_eq!(m.get(&2), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: PartialEq + ?Sized,
    {
        let pair = self.pairs[..self.len]
            .iter()
            .find(|p| unsafe { p.assume_init_ref() }.0.borrow() == k)?;
        Some(unsafe { &pair.assume_init_ref().1 })
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map’s key type, but
    /// [`PartialEq`] on the borrowed form must match those for the key
    /// type.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut m: Map<_, _, 3> = Map::new();
    /// m.insert(1, "a");
    /// if let Some(x) = m.get_mut(&1) {
    ///     *x = "b";
    /// }
    /// assert_eq!(m[&1], "b");
    /// ```
    #[inline]
    #[must_use]
    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: PartialEq + ?Sized,
    {
        let pair = self.pairs[..self.len]
            .iter_mut()
            .find(|p| unsafe { p.assume_init_ref() }.0.borrow() == k)?;
        Some(unsafe { &mut pair.assume_init_mut().1 })
    }

    /// Returns the key-value pair corresponding to the supplied key.
    ///
    /// The key may be any borrowed form of the map’s key type, but
    /// [`PartialEq`] on the borrowed form must match those for the key
    /// type.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// // Only the first element in the tuple is considered when
    /// // determining whether two `S` are equal.
    /// #[derive(Clone, Copy, Debug)]
    /// struct S {
    ///     id: u32,
    ///     name: &'static str, // ignored by equality and hashing operations
    /// }
    /// impl PartialEq for S {
    ///     fn eq(&self, other: &S) -> bool {
    ///         self.id == other.id
    ///     }
    /// }
    /// // Note the impact of `.name` in code.
    /// let j_a = S { id: 1, name: "Jessica" };
    /// let j_b = S { id: 1, name: "Jess" };
    /// let p = S { id: 2, name: "Paul" };
    /// assert_ne!(j_a.name, j_b.name);
    /// assert_eq!(j_a, j_b);
    /// let mut m: Map<_, _, 3> = Map::new();
    /// m.insert(j_a, "Paris");
    /// assert_eq!(m.get_key_value(&j_a), Some((&j_a, &"Paris")));
    /// assert_eq!(m.get_key_value(&j_b), Some((&j_a, &"Paris"))); // the notable case
    /// assert_eq!(m.get_key_value(&p), None);
    #[inline]
    #[must_use]
    pub fn get_key_value<Q>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: PartialEq + ?Sized,
    {
        let pair = self.pairs[..self.len]
            .iter()
            .find(|p| unsafe { p.assume_init_ref() }.0.borrow() == k)?;
        let (k, v) = unsafe { pair.assume_init_ref() };
        Some((k, v))
    }

    /// Attempts to get mutable references to `J` values in the map at once.
    /// Or you want to batch search keys (and get mutable references to them) in
    /// one iteration for map.
    ///
    /// Returns an array of length `J` with the results of each query.
    /// For soundness, at most one mutable reference will be returned to any value.
    /// None will be used if the key is missing.
    ///
    /// # Panics
    /// Panics if any keys are overlapping.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut libraries: Map<String, u32, 5> = Map::new();
    /// libraries.insert("Bodleian Library".to_string(), 1602);
    /// libraries.insert("Athenæum".to_string(), 1807);
    /// libraries.insert("Herzogin-Anna-Amalia-Bibliothek".to_string(), 1691);
    /// libraries.insert("Library of Congress".to_string(), 1800);
    /// // Get Athenæum and Bodleian Library
    /// let [Some(a), Some(b)] = libraries.get_disjoint_mut([
    ///     "Athenæum",
    ///     "Bodleian Library",
    /// ]) else { panic!() };
    /// *a += 10000;
    /// *b += 10000;
    /// // Assert values of Athenæum and Library of Congress
    /// let got = libraries.get_disjoint_mut([
    ///     "Athenæum",
    ///     "Library of Congress",
    /// ]);
    /// assert_eq!(
    ///     got,
    ///     [
    ///         Some(&mut 11807),
    ///         Some(&mut 1800),
    ///     ],
    /// );
    /// // Missing keys result in None
    /// let got = libraries.get_disjoint_mut([
    ///     "Athenæum",
    ///     "New York Public Library",
    /// ]);
    /// assert_eq!(
    ///     got,
    ///     [
    ///         Some(&mut 11807),
    ///         None
    ///     ]
    /// );
    /// ```
    /// Duplicate keys will panic:
    /// ```should_panic
    /// use micromap::Map;
    /// let mut libraries: Map<String, u32, 3> = Map::new();
    /// libraries.insert("Athenæum".to_string(), 1807);
    /// // panic!
    /// let _got = libraries.get_disjoint_mut([
    ///     "Athenæum",
    ///     "Athenæum",
    /// ]);
    /// ```
    #[inline]
    #[must_use]
    pub fn get_disjoint_mut<Q, const J: usize>(&mut self, ks: [&Q; J]) -> [Option<&mut V>; J]
    where
        K: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        if ks.is_empty() {
            return [const { None }; J];
        }
        // check for overlapping keys (O(n^2), but n is small)
        for (i, k) in ks[..ks.len() - 1].iter().enumerate() {
            for k_behind in &ks[i + 1..] {
                assert!(k != k_behind, "Overlapping keys");
            }
        }
        unsafe { self.get_disjoint_unchecked_mut(ks) }
    }

    /// Attempts to get mutable references to `J` values in the map at once, without validating that
    /// the values are unique. Or you want to batch search keys (and get mutable references to them)
    /// in one iteration for map.
    ///
    /// Returns an array of length `J` with the results of each query. `None` will be used if
    /// the key is missing.
    ///
    /// For a safe alternative see [`Map::get_disjoint_mut`].
    ///
    /// # Safety
    /// Calling this method with overlapping keys is *[undefined behavior]* even if the resulting
    /// references are not used.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut libraries: Map<String, u32, 5> = Map::new();
    /// libraries.insert("Bodleian Library".to_string(), 1602);
    /// libraries.insert("Athenæum".to_string(), 1807);
    /// libraries.insert("Herzogin-Anna-Amalia-Bibliothek".to_string(), 1691);
    /// libraries.insert("Library of Congress".to_string(), 1800);
    /// // SAFETY: The keys do not overlap.
    /// let [Some(a), Some(b)] = (unsafe { libraries.get_disjoint_unchecked_mut([
    ///     "Athenæum",
    ///     "Bodleian Library",
    /// ]) }) else { panic!() };
    /// *a += 10000;
    /// *b += 10000;
    /// // SAFETY: The keys do not overlap.
    /// let got = unsafe { libraries.get_disjoint_unchecked_mut([
    ///     "Athenæum",
    ///     "Library of Congress",
    /// ]) };
    /// assert_eq!(
    ///     got,
    ///     [
    ///         Some(&mut 11807),
    ///         Some(&mut 1800),
    ///     ],
    /// );
    /// // SAFETY: The keys do not overlap.
    /// let got = unsafe { libraries.get_disjoint_unchecked_mut([
    ///     "Athenæum",
    ///     "New York Public Library",
    /// ]) };
    /// // Missing keys result in None
    /// assert_eq!(got, [Some(&mut 11807), None]);
    /// ```
    #[inline]
    #[must_use]
    pub unsafe fn get_disjoint_unchecked_mut<Q, const J: usize>(
        &mut self,
        ks: [&Q; J],
    ) -> [Option<&mut V>; J]
    where
        K: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        let mut ret: [Option<&mut V>; J] = [const { None }; J];
        if ks.is_empty() {
            return ret;
        } else if ks.len() == 1 {
            ret[0] = self.get_mut(ks[0]);
            return ret;
        }
        // find the keys' index in one iteration of the map, store the result
        // into the stack.
        let mut stack = [const { None }; J];
        let mut stack_top = 0;
        for pair_i in 0..self.len {
            let p = unsafe { self.item_ref(pair_i) };
            if let Some(ks_i) = ks.iter().position(|&k| k.borrow() == p.0.borrow()) {
                stack[stack_top] = Some((pair_i, ks_i));
                stack_top += 1;
            }
        }
        stack[..stack_top].sort_unstable_by_key(|x| x.map(|(pair_i, _)| pair_i));
        // start splitting the found pairs from the back to the front
        let mut rest_head = &mut self.pairs[..self.len];
        for (pair_i, ks_i) in stack[..stack_top].iter().rev().flatten() {
            let (head, tail) = rest_head.split_at_mut(*pair_i);
            rest_head = head;
            let p = unsafe { tail[0].assume_init_mut() };
            ret[*ks_i] = Some(&mut p.1);
        }
        ret
    }

    /// Removes a key from the map, returning the stored key and value if
    /// the key was previously in the map.
    ///
    /// The key may be any borrowed form of the map’s key type, but
    /// [`PartialEq`] on the borrowed form must match those for the key
    /// type.
    ///
    /// # Examples
    /// ```
    /// use micromap::Map;
    /// let mut m: Map<_, _, 3> = Map::new();
    /// m.insert(1, "a");
    /// assert_eq!(m.remove_entry(&1), Some((1, "a")));
    /// assert_eq!(m.remove(&1), None);
    /// ```
    #[inline]
    #[must_use = "if no need the return value, use `remove()` instead."]
    pub fn remove_entry<Q>(&mut self, k: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: PartialEq + ?Sized,
    {
        let (i, _) = self.pairs[..self.len]
            .iter()
            .enumerate()
            .find(|(_, p)| unsafe { p.assume_init_ref() }.0.borrow() == k)?;
        Some(unsafe { self.remove_index_read(i) })
    }
}

mod internal {
    use super::Map;

    /// The unsafe wrapper operations for the `&[MaybeUninit]` array in [`Map`] struct.
    impl<K, V, const N: usize> Map<K, V, N> {
        /// Internal function to get mutable access via reference to the value in the internal array.
        #[inline]
        #[must_use]
        pub(crate) unsafe fn value_mut(&mut self, i: usize) -> &mut V {
            &mut self.pairs.get_unchecked_mut(i).assume_init_mut().1
        }

        /// Internal function to get access via reference to the element in the internal array.
        #[inline]
        #[must_use]
        pub(crate) unsafe fn item_ref(&self, i: usize) -> &(K, V) {
            self.pairs.get_unchecked(i).assume_init_ref()
        }

        /// Internal function to get mutable access via reference to the element in the internal array.
        #[inline]
        #[must_use]
        pub(crate) unsafe fn item_mut(&mut self, i: usize) -> &mut (K, V) {
            self.pairs.get_unchecked_mut(i).assume_init_mut()
        }

        /// Internal function to get access to the element in the internal array.
        #[inline]
        #[must_use]
        pub(crate) unsafe fn item_read(&mut self, i: usize) -> (K, V) {
            self.pairs.get_unchecked(i).assume_init_read()
        }

        /// Internal function to get access to the element in the internal array and drop it.
        #[inline]
        pub(crate) unsafe fn item_drop(&mut self, i: usize) {
            self.pairs.get_unchecked_mut(i).assume_init_drop();
        }

        /// Internal function to write key and value to the element in the internal array.
        #[inline]
        pub(crate) unsafe fn item_write(&mut self, i: usize, val: (K, V)) {
            self.pairs.get_unchecked_mut(i).write(val);
        }

        /// Remove by index and drop it (by swapping the last one here and reducing the length).
        #[inline]
        pub(crate) unsafe fn remove_index_drop(&mut self, i: usize) {
            self.item_drop(i);
            self.len -= 1;
            if i != self.len {
                let value = self.item_read(self.len);
                self.item_write(i, value);
            }
        }

        /// Remove by index and return it (by swapping the last one here and reducing the length).
        #[inline]
        #[must_use = "if no need the return value, use `remove_index_drop()` instead."]
        pub(crate) unsafe fn remove_index_read(&mut self, i: usize) -> (K, V) {
            let result = self.item_read(i);
            self.len -= 1;
            if i != self.len {
                let value = self.item_read(self.len);
                self.item_write(i, value);
            }
            result
        }
    }

    /// The insert core logic for the [`Map`] struct.
    impl<K: PartialEq, V, const N: usize> Map<K, V, N> {
        /// The core insert logic, which is used for `insert_unchecked()`, as it will
        /// disable the bound check (`debug_assert!`) in `release` mode.
        pub(crate) fn insert_i(&mut self, k: K, v: V, update_key: bool) -> (usize, Option<(K, V)>) {
            let mut target = self.len;
            let mut i = 0;
            let mut existing_pair = None;
            loop {
                if i == self.len {
                    core::debug_assert!(target < N, "No more key-value slot available in the map");
                    break;
                }
                let p = unsafe { self.item_ref(i) };
                if p.0 == k {
                    target = i;
                    existing_pair = Some(unsafe { self.item_read(i) });
                    break;
                }
                i += 1;
            }
            if target == self.len {
                self.len += 1;
            }
            if !update_key {
                if let Some((old_k, old_v)) = existing_pair {
                    unsafe { self.item_write(target, (old_k, v)) };
                    return (target, Some((k, old_v)));
                }
            }
            unsafe { self.item_write(target, (k, v)) };
            (target, existing_pair)
        }

        /// The core insert logic, which is used for `insert()`. Its name means
        /// that it uses iterators(the second `i`) instead of loops to implement the underlying
        /// insertion logic for `insert_i()`.
        pub(crate) fn insert_ii(
            &mut self,
            k: K,
            v: V,
            update_key: bool,
        ) -> (usize, Option<(K, V)>) {
            if let Some((i, pair)) = self.pairs[..self.len]
                .iter_mut()
                .map(|p| unsafe { p.assume_init_mut() })
                .enumerate()
                .find(|(_i, p)| p.0 == k)
            {
                if update_key {
                    (i, Some(core::mem::replace(pair, (k, v))))
                } else {
                    (i, Some((k, core::mem::replace(&mut pair.1, v))))
                }
            } else {
                let i = self.len;
                // just for panic msg in debug mode, not the main bound check
                core::debug_assert!(i < N, "No more key-value slot available in the map");
                self.pairs[i].write((k, v)); // main bound check here but it's ok (not hotspot)
                self.len += 1;
                (i, None)
            }
        }

        /// When the map is full, we need to check the key-value pair if existed already or not.
        /// If no place to insert the new key-value pair, we return `None`.
        pub(crate) fn insert_ii_for_full(
            &mut self,
            k: K,
            v: V,
            update_key: bool,
        ) -> Option<(usize, (K, V))> {
            if let Some((i, pair)) = self.pairs[..self.len]
                .iter_mut()
                .map(|p| unsafe { p.assume_init_mut() })
                .enumerate()
                .find(|(_, p)| p.0 == k)
            {
                let existing_pair = if update_key {
                    core::mem::replace(pair, (k, v))
                } else {
                    (k, core::mem::replace(&mut pair.1, v))
                };
                Some((i, existing_pair))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/micromap/unit/src/map/methods/tests_774.rs"));
