mod impls;

use core::borrow::Borrow;
use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};
use once_cell::unsync::OnceCell;

use crate::hash::{CryptoHash, Sha256};
use crate::utils::{EntryState, StableMap};
use crate::{env, CacheEntry, IntoStorageKey};

const ERR_ELEMENT_DESERIALIZATION: &[u8] = b"Cannot deserialize element";
const ERR_ELEMENT_SERIALIZATION: &[u8] = b"Cannot serialize element";
const ERR_NOT_EXIST: &[u8] = b"Key does not exist in map";

type LookupKey = [u8; 32];

/// A non-iterable, lazily loaded storage map that stores its content directly on the storage trie.
///
/// This map stores the values under a hash of the map's `prefix` and [`BorshSerialize`] of the key
/// using the map's [`CryptoHash`] implementation.
///
/// The default hash function for [`LookupMap`] is [`Sha256`] which uses a syscall to hash the
/// key. To use a custom function, use [`new_with_hasher`]
///
/// # Examples
/// ```
/// use near_sdk::store::LookupMap;
///
/// // Initializes a map, the generic types can be inferred to `LookupMap<String, u8, Sha256>`
/// let mut map = LookupMap::new(b"a");
///
/// map.set("test".to_string(), Some(7u8));
/// assert!(map.contains_key("test"));
/// assert_eq!(map.get("test"), Some(&7u8));
///
/// let prev = map.insert("test".to_string(), 5u8);
/// assert_eq!(prev, Some(7u8));
/// assert_eq!(map["test"], 5u8);
/// ```
///
/// [`new_with_hasher`]: Self::new_with_hasher
#[derive(BorshSerialize, BorshDeserialize)]
pub struct LookupMap<K, V, H = Sha256>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
    H: CryptoHash<Digest = [u8; 32]>,
{
    prefix: Box<[u8]>,
    #[borsh_skip]
    /// Cache for loads and intermediate changes to the underlying vector.
    /// The cached entries are wrapped in a [`Box`] to avoid existing pointers from being
    /// invalidated.
    cache: StableMap<K, OnceCell<CacheEntry<V>>>,

    #[borsh_skip]
    hasher: PhantomData<H>,
}

impl<K, V> LookupMap<K, V, Sha256>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
{
    #[inline]
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self::new_with_hasher(prefix)
    }
}

impl<K, V, H> LookupMap<K, V, H>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
    H: CryptoHash<Digest = [u8; 32]>,
{
    /// Initialize a [`LookupMap`] with a custom hash function.
    ///
    /// # Example
    /// ```
    /// use near_sdk::hash::Keccak256;
    /// use near_sdk::store::LookupMap;
    ///
    /// let map = LookupMap::<String, String, Keccak256>::new_with_hasher(b"m");
    /// ```
    pub fn new_with_hasher<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self {
            prefix: prefix.into_storage_key().into_boxed_slice(),
            cache: Default::default(),
            hasher: Default::default(),
        }
    }

    /// Overwrites the current value for the given key.
    ///
    /// This function will not load the existing value from storage and return the value in storage.
    /// Use [`LookupMap::insert`] if you need the previous value.
    ///
    /// Calling `set` with a `None` value will delete the entry from storage.
    pub fn set(&mut self, key: K, value: Option<V>) {
        let entry = self.cache.get_mut(key);
        match entry.get_mut() {
            Some(entry) => *entry.value_mut() = value,
            None => {
                let _ = entry.set(CacheEntry::new_modified(value));
            }
        }
    }

    fn lookup_key<Q: ?Sized>(prefix: &[u8], key: &Q) -> LookupKey
    where
        Q: BorshSerialize,
        K: Borrow<Q>,
    {
        // Concat the prefix with serialized key and hash the bytes for the lookup key.
        let mut buffer = prefix.to_vec();
        key.serialize(&mut buffer).unwrap_or_else(|_| env::panic(ERR_ELEMENT_SERIALIZATION));

        H::hash(&buffer)
    }
}

impl<K, V, H> LookupMap<K, V, H>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHash<Digest = [u8; 32]>,
{
    fn deserialize_element(bytes: &[u8]) -> V {
        V::try_from_slice(bytes).unwrap_or_else(|_| env::panic(ERR_ELEMENT_DESERIALIZATION))
    }

    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: BorshSerialize + ToOwned<Owned = K>,
    {
        //* ToOwned bound, which forces a clone, is required to be able to keep the key in the cache
        let entry = self.cache.get(k.to_owned()).get_or_init(|| {
            let storage_bytes = env::storage_read(&Self::lookup_key(&self.prefix, k));
            let value = storage_bytes.as_deref().map(Self::deserialize_element);
            CacheEntry::new_cached(value)
        });
        entry.value().as_ref()
    }

    fn get_mut_inner<Q: ?Sized>(&mut self, k: &Q) -> &mut CacheEntry<V>
    where
        K: Borrow<Q>,
        Q: BorshSerialize + ToOwned<Owned = K>,
    {
        let prefix = &self.prefix;
        //* ToOwned bound, which forces a clone, is required to be able to keep the key in the cache
        let entry = self.cache.get_mut(k.to_owned());
        entry.get_or_init(|| {
            let storage_bytes = env::storage_read(&Self::lookup_key(&prefix, k));
            let value = storage_bytes.as_deref().map(Self::deserialize_element);
            CacheEntry::new_cached(value)
        });
        let entry = entry.get_mut().unwrap();
        entry
    }

    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: BorshSerialize + ToOwned<Owned = K>,
    {
        self.get_mut_inner(k).value_mut().as_mut()
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V>
    where
        K: Clone,
    {
        self.get_mut_inner(&k).replace(Some(v))
    }

    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: BorshSerialize + ToOwned<Owned = K> + Ord,
    {
        // Check cache before checking storage
        if self
            .cache
            .with_value(&k, |v| v.get().and_then(|s| s.value().as_ref()).is_some())
            .unwrap_or(false)
        {
            return true;
        }
        let storage_key = Self::lookup_key(&self.prefix, k);
        let contains = env::storage_has_key(&storage_key);

        if !contains {
            // If value not in cache and not in storage, can set a cached `None`
            let _ = self.cache.get(k.to_owned()).set(CacheEntry::new_cached(None));
        }
        contains
    }

    pub fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: BorshSerialize + ToOwned<Owned = K>,
    {
        self.get_mut_inner(k).replace(None)
    }
}

impl<K, V, H> LookupMap<K, V, H>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
    H: CryptoHash<Digest = [u8; 32]>,
{
    pub fn flush(&mut self) {
        let mut buf = Vec::new();
        for (k, v) in self.cache.inner().iter_mut() {
            if let Some(v) = v.get_mut() {
                if v.is_modified() {
                    let key = Self::lookup_key(&self.prefix, k);
                    match v.value().as_ref() {
                        Some(modified) => {
                            buf.clear();
                            BorshSerialize::serialize(modified, &mut buf)
                                .unwrap_or_else(|_| env::panic(ERR_ELEMENT_SERIALIZATION));
                            env::storage_write(&key, &buf);
                        }
                        None => {
                            // Element was removed, clear the storage for the value
                            env::storage_remove(&key);
                        }
                    }

                    // Update state of flushed state as cached, to avoid duplicate writes/removes
                    // while also keeping the cached values in memory.
                    v.replace_state(EntryState::Cached);
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::LookupMap;
    use crate::env;
    use crate::hash::Keccak256;
    use rand::seq::SliceRandom;
    use rand::{Rng, SeedableRng};
    use std::collections::HashMap;

    #[test]
    fn test_insert() {
        let mut map = LookupMap::new(b"m");
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        for _ in 0..500 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            map.insert(key, value);
        }
    }

    #[test]
    fn test_insert_has_key() {
        let mut map = LookupMap::new(b"m");
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        let mut key_to_value = HashMap::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            map.insert(key, value);
            key_to_value.insert(key, value);
        }
        // Non existing
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            assert_eq!(map.contains_key(&key), key_to_value.contains_key(&key));
        }
        // Existing
        for (key, _) in key_to_value.iter() {
            assert!(map.contains_key(&key));
        }
    }

    #[test]
    fn test_insert_remove() {
        let mut map = LookupMap::new(b"m");
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(1);
        let mut keys = vec![];
        let mut key_to_value = HashMap::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            keys.push(key);
            key_to_value.insert(key, value);
            map.insert(key, value);
        }
        keys.shuffle(&mut rng);
        for key in keys {
            let actual = map.remove(&key).unwrap();
            assert_eq!(actual, key_to_value[&key]);
        }
    }

    #[test]
    fn test_remove_last_reinsert() {
        let mut map = LookupMap::new(b"m");
        let key1 = 1u64;
        let value1 = 2u64;
        map.insert(key1, value1);
        let key2 = 3u64;
        let value2 = 4u64;
        map.insert(key2, value2);

        let actual_value2 = map.remove(&key2).unwrap();
        assert_eq!(actual_value2, value2);

        let actual_insert_value2 = map.insert(key2, value2);
        assert_eq!(actual_insert_value2, None);
    }

    #[test]
    fn test_insert_override_remove() {
        let mut map = LookupMap::new(b"m");
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(2);
        let mut keys = vec![];
        let mut key_to_value = HashMap::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            keys.push(key);
            key_to_value.insert(key, value);
            map.insert(key, value);
        }
        keys.shuffle(&mut rng);
        for key in &keys {
            let value = rng.gen::<u64>();
            let actual = map.insert(*key, value).unwrap();
            assert_eq!(actual, key_to_value[key]);
            key_to_value.insert(*key, value);
        }
        keys.shuffle(&mut rng);
        for key in keys {
            let actual = map.remove(&key).unwrap();
            assert_eq!(actual, key_to_value[&key]);
        }
    }

    #[test]
    fn test_get_non_existent() {
        let mut map = LookupMap::new(b"m");
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(3);
        let mut key_to_value = HashMap::new();
        for _ in 0..500 {
            let key = rng.gen::<u64>() % 20_000;
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(key, value);
        }
        for _ in 0..500 {
            let key = rng.gen::<u64>() % 20_000;
            assert_eq!(map.get(&key), key_to_value.get(&key));
        }
    }

    #[test]
    fn test_extend() {
        let mut map = LookupMap::new(b"m");
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let mut key_to_value = HashMap::new();
        for _ in 0..100 {
            let key = rng.gen::<u64>();
            let value = rng.gen::<u64>();
            key_to_value.insert(key, value);
            map.insert(key, value);
        }
        for _ in 0..10 {
            let mut tmp = vec![];
            for _ in 0..=(rng.gen::<u64>() % 20 + 1) {
                let key = rng.gen::<u64>();
                let value = rng.gen::<u64>();
                tmp.push((key, value));
            }
            key_to_value.extend(tmp.iter().cloned());
            map.extend(tmp.iter().cloned());
        }

        for (key, value) in key_to_value {
            assert_eq!(*map.get(&key).unwrap(), value);
        }
    }

    #[test]
    fn flush_on_drop() {
        let mut map = LookupMap::<_, _, Keccak256>::new_with_hasher(b"m");

        // Set a value, which does not write to storage yet
        map.set(5u8, Some(8u8));

        // Create duplicate which references same data
        assert_eq!(map[&5], 8);

        let storage_key = LookupMap::<u8, u8, Keccak256>::lookup_key(b"m", &5);
        assert!(!env::storage_has_key(&storage_key));

        drop(map);

        let dup_map = LookupMap::<u8, u8, Keccak256>::new_with_hasher(b"m");

        // New map can now load the value
        assert_eq!(dup_map[&5], 8);
    }
}
