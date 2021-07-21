use core::borrow::Borrow;
use std::cell::RefCell;
use std::collections::BTreeMap;

pub(crate) struct StableMap<K, V> {
    map: RefCell<BTreeMap<K, Box<V>>>,
}

impl<K: Ord, V> Default for StableMap<K, V> {
    fn default() -> Self {
        Self { map: Default::default() }
    }
}

impl<K, V> StableMap<K, V> {
    pub(crate) fn get(&self, k: K) -> &V
    where
        K: Ord,
        V: Default,
    {
        let mut map = self.map.borrow_mut();
        let v: &mut Box<V> = map.entry(k).or_default();
        let v: &V = &*v;
        // SAFETY: here, we extend the lifetime of `V` from local `RefCell`
        // borrow to the `&self`. This is valid because we only append to the
        // map via `&` reference, and the values are boxed, so we have stability
        // of addresses.
        unsafe { &*(v as *const V) }
    }
    pub(crate) fn get_mut(&mut self, k: K) -> &mut V
    where
        K: Ord,
        V: Default,
    {
        &mut *self.map.get_mut().entry(k).or_default()
    }
    pub(crate) fn inner(&mut self) -> &mut BTreeMap<K, Box<V>> {
        self.map.get_mut()
    }
    pub(crate) fn with_value<Q: ?Sized, F, T>(&self, k: &Q, f: F) -> Option<T>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
        F: FnOnce(&V) -> T,
    {
        self.map.borrow().get(k).map(|s| f(s))
    }
}
