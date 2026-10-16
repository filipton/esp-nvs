//! A small ordered map on a sorted `Vec`.
//!
//! The driver only ever holds a handful of entries per map (namespaces, blob versions, duplicate
//! keys), where `BTreeMap`'s per-type node code costs far more flash than a binary search over a
//! `Vec`. Iteration stays in key order, like `BTreeMap`.

use alloc::vec::Vec;

pub(crate) struct VecMap<K, V>(Vec<(K, V)>);

impl<K, V> Default for VecMap<K, V> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<K: Ord, V> VecMap<K, V> {
    pub(crate) const fn new() -> Self {
        Self(Vec::new())
    }

    fn find(&self, key: &K) -> Result<usize, usize> {
        self.0.binary_search_by(|(k, _)| k.cmp(key))
    }

    pub(crate) fn get(&self, key: &K) -> Option<&V> {
        self.find(key).ok().map(|i| &self.0[i].1)
    }

    pub(crate) fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.find(key).ok().map(|i| &mut self.0[i].1)
    }

    /// Inserts `value`, returning the previous value of `key` if there was one.
    pub(crate) fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.find(&key) {
            Ok(i) => Some(core::mem::replace(&mut self.0[i].1, value)),
            Err(i) => {
                self.0.insert(i, (key, value));
                None
            }
        }
    }

    /// Like `BTreeMap::entry(key).or_default()`.
    pub(crate) fn entry_or_default(&mut self, key: K) -> &mut V
    where
        V: Default,
    {
        let i = match self.find(&key) {
            Ok(i) => i,
            Err(i) => {
                self.0.insert(i, (key, V::default()));
                i
            }
        };
        &mut self.0[i].1
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter().map(|(k, v)| (k, v))
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.iter().map(|(k, _)| k)
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &V> {
        self.0.iter().map(|(_, v)| v)
    }
}

impl<K, V> IntoIterator for VecMap<K, V> {
    type Item = (K, V);
    type IntoIter = alloc::vec::IntoIter<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Stable in-place sort for the short lists the driver sorts (versions of one blob, entries of
/// one key); far less code than `slice::sort_by_key`.
pub(crate) fn sort_by_key<T, K: Ord>(v: &mut [T], mut key: impl FnMut(&T) -> K) {
    for i in 1..v.len() {
        let mut j = i;
        while j > 0 && key(&v[j - 1]) > key(&v[j]) {
            v.swap(j - 1, j);
            j -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behaves_like_btreemap() {
        let mut m = VecMap::new();
        assert_eq!(m.insert(3, 'c'), None);
        assert_eq!(m.insert(1, 'a'), None);
        assert_eq!(m.insert(2, 'x'), None);
        assert_eq!(m.insert(2, 'b'), Some('x'));
        *m.entry_or_default(4) = 'd';
        assert_eq!(m.get(&2), Some(&'b'));
        assert_eq!(m.get(&5), None);
        assert_eq!(m.keys().copied().collect::<Vec<_>>(), [1, 2, 3, 4]);
        assert_eq!(m.into_iter().map(|(_, v)| v).collect::<Vec<_>>(), ['a', 'b', 'c', 'd']);
    }

    #[test]
    fn sort_is_stable() {
        let mut v = [(2, 'a'), (1, 'b'), (2, 'c'), (0, 'd'), (1, 'e')];
        sort_by_key(&mut v, |e| e.0);
        assert_eq!(v, [(0, 'd'), (1, 'b'), (1, 'e'), (2, 'a'), (2, 'c')]);
    }
}
