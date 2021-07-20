mod impls;
mod iter;

use std::fmt;

use borsh::{BorshDeserialize, BorshSerialize};
use once_cell::unsync::OnceCell;

pub use self::iter::{Iter, IterMut};
use crate::collections::append_slice;
use crate::utils::StableMap;
use crate::{env, CacheEntry, EntryState, IntoStorageKey};

const ERR_INCONSISTENT_STATE: &[u8] = b"The collection is an inconsistent state. Did previous smart contract execution terminate unexpectedly?";
const ERR_ELEMENT_DESERIALIZATION: &[u8] = b"Cannot deserialize element";
const ERR_ELEMENT_SERIALIZATION: &[u8] = b"Cannot serialize element";
const ERR_INDEX_OUT_OF_BOUNDS: &[u8] = b"Index out of bounds";

fn expect_consistent_state<T>(val: Option<T>) -> T {
    val.unwrap_or_else(|| env::panic(ERR_INCONSISTENT_STATE))
}

/// An iterable implementation of vector that stores its content on the trie. This implementation
/// will load and store values in the underlying storage lazily.
///
/// Uses the following map: index -> element. Because the data is sharded to avoid reading/writing
/// large chunks of data, the values cannot be accessed as a contiguous piece of memory.
///
/// This implementation will cache all changes and loads and only updates values that are changed
/// in storage after it's dropped through it's [`Drop`] implementation. These changes can be updated
/// in storage before the variable is dropped by using [`Vector::flush`]. During the lifetime of
/// this type, storage will only be read a maximum of one time per index and only written once per
/// index unless specifically flushed.
///
/// This type should be a drop in replacement for [`Vec`] in most cases and will provide contracts
/// a vector structure which scales much better as the contract data grows.
///
/// # Examples
/// ```
/// use near_sdk::store::Vector;
///
///# near_sdk::test_utils::test_env::setup();
/// let mut vec = Vector::new(b"a");
/// assert!(vec.is_empty());
///
/// vec.push(1);
/// vec.push(2);
///
/// assert_eq!(vec.len(), 2);
/// assert_eq!(vec[0], 1);
///
/// assert_eq!(vec.pop(), Some(2));
/// assert_eq!(vec.len(), 1);
///
/// vec[0] = 7;
/// assert_eq!(vec[0], 7);
///
/// vec.extend([1, 2, 3].iter().copied());
/// assert!(Iterator::eq(vec.into_iter(), [7, 1, 2, 3].iter()));
/// ```
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Vector<T>
where
    T: BorshSerialize,
{
    len: u32,
    prefix: Box<[u8]>,
    #[borsh_skip]
    /// Cache for loads and intermediate changes to the underlying vector.
    /// The cached entries are wrapped in a [`Box`] to avoid existing pointers from being
    /// invalidated.
    cache: StableMap<u32, OnceCell<CacheEntry<T>>>,
}

impl<T> Vector<T>
where
    T: BorshSerialize,
{
    /// Returns the number of elements in the vector, also referred to as its size.
    /// This function returns a `u32` rather than the [`Vec`] equivalent of `usize` to have
    /// consistency between targets.
    pub fn len(&self) -> u32 {
        self.len
    }

    /// Returns `true` if the vector contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Create new vector with zero elements. Use `id` as a unique identifier on the trie.
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        Self {
            len: 0,
            prefix: prefix.into_storage_key().into_boxed_slice(),
            cache: Default::default(),
        }
    }

    fn index_to_lookup_key(&self, index: u32) -> Vec<u8> {
        append_slice(&self.prefix, &index.to_le_bytes()[..])
    }

    /// Removes all elements from the collection. This will remove all storage values for the
    /// length of the [`Vector`].
    pub fn clear(&mut self) {
        for i in 0..self.len {
            let lookup_key = self.index_to_lookup_key(i);
            env::storage_remove(&lookup_key);
        }
        self.len = 0;
        self.cache.inner().clear();
    }

    /// Flushes the cache and writes all modified values to storage.
    pub fn flush(&mut self) {
        let mut buf = Vec::new();
        for (k, v) in self.cache.inner().iter_mut() {
            if let Some(v) = v.get_mut() {
                if v.is_modified() {
                    let key = append_slice(&self.prefix, &k.to_le_bytes()[..]);
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

    /// Sets a value at a given index to the value provided. This does not shift values after the
    /// index to the right.
    pub fn set(&mut self, index: u32, value: T) {
        if index >= self.len() {
            env::panic(ERR_INDEX_OUT_OF_BOUNDS);
        }

        let entry = self.cache.get_mut(index);
        match entry.get_mut() {
            Some(entry) => *entry.value_mut() = Some(value),
            None => {
                let _ = entry.set(CacheEntry::new_modified(Some(value)));
            }
        }
    }

    /// Appends an element to the back of the collection.
    pub fn push(&mut self, element: T) {
        let last_idx = self.len();
        self.len = self.len.checked_add(1).unwrap_or_else(|| env::panic(ERR_INDEX_OUT_OF_BOUNDS));
        self.set(last_idx, element)
    }
}

impl<T> Vector<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn deserialize_element(raw_element: &[u8]) -> T {
        T::try_from_slice(&raw_element).unwrap_or_else(|_| env::panic(ERR_ELEMENT_DESERIALIZATION))
    }

    /// Returns the element by index or `None` if it is not present.
    pub fn get(&self, index: u32) -> Option<&T> {
        let entry = self.cache.get(index).get_or_init(|| {
            let storage_bytes = env::storage_read(&self.index_to_lookup_key(index));
            let value = storage_bytes.as_deref().map(Self::deserialize_element);
            CacheEntry::new_cached(value)
        });
        entry.value().as_ref()
    }

    /// Returns a mutable reference to the element at the `index` provided.
    fn get_mut_inner(&mut self, index: u32) -> Option<&mut CacheEntry<T>> {
        if index >= self.len {
            return None;
        }
        let index_to_lookup_key = self.index_to_lookup_key(index);
        let entry = self.cache.get_mut(index);
        entry.get_or_init(|| {
            let storage_bytes = env::storage_read(&index_to_lookup_key);
            let value = storage_bytes.as_deref().map(Self::deserialize_element);
            CacheEntry::new_cached(value)
        });
        let entry = entry.get_mut().unwrap();
        Some(entry)
    }

    /// Returns a mutable reference to the element at the `index` provided.
    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        let entry = self.get_mut_inner(index)?;
        entry.value_mut().as_mut()
    }

    fn swap(&mut self, a: u32, b: u32) {
        if a >= self.len() || b >= self.len() {
            env::panic(ERR_INDEX_OUT_OF_BOUNDS);
        }

        if a == b {
            // Short circuit if indices are the same, also guarantees uniqueness below
            return;
        }

        let val_a = self.get_mut_inner(a).unwrap().replace(None);
        let val_b = self.get_mut_inner(b).unwrap().replace(val_a);
        self.get_mut_inner(a).unwrap().replace(val_b);
    }

    /// Removes an element from the vector and returns it.
    /// The removed element is replaced by the last element of the vector.
    /// Does not preserve ordering, but is `O(1)`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    pub fn swap_remove(&mut self, index: u32) -> T {
        if self.is_empty() {
            env::panic(ERR_INDEX_OUT_OF_BOUNDS);
        }

        self.swap(index, self.len() - 1);
        expect_consistent_state(self.pop())
    }

    /// Removes the last element from a vector and returns it, or `None` if it is empty.
    pub fn pop(&mut self) -> Option<T> {
        let new_idx = self.len.checked_sub(1)?;
        let prev = self.get_mut_inner(new_idx)?.replace(None);
        self.len = new_idx;
        prev
    }

    /// Inserts a element at `index`, returns an evicted element.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn replace(&mut self, index: u32, element: T) -> T {
        self.get_mut_inner(index)
            .unwrap_or_else(|| env::panic(ERR_INDEX_OUT_OF_BOUNDS))
            .replace(Some(element))
            .unwrap()
    }

    /// Returns an iterator over the vector. This iterator will lazily load any values iterated
    /// over from storage.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(self)
    }

    /// Returns an iterator over the [`Vector`] that allows modifying each value. This iterator
    /// will lazily load any values iterated over from storage.
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut::new(self)
    }
}

impl<T> fmt::Debug for Vector<T>
where
    T: BorshSerialize + BorshDeserialize + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if cfg!(feature = "expensive-debug") {
            fmt::Debug::fmt(&self.iter().collect::<Vec<_>>(), f)
        } else {
            f.debug_struct("Vector").field("len", &self.len).field("prefix", &self.prefix).finish()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use rand::{Rng, SeedableRng};

    use super::Vector;
    use crate::test_utils::test_env;

    #[test]
    fn test_push_pop() {
        test_env::setup();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        let mut vec = Vector::new(b"v".to_vec());
        let mut baseline = vec![];
        for _ in 0..500 {
            let value = rng.gen::<u64>();
            vec.push(value);
            baseline.push(value);
        }
        let actual: Vec<u64> = vec.iter().cloned().collect();
        assert_eq!(actual, baseline);
        for _ in 0..501 {
            assert_eq!(baseline.pop(), vec.pop());
        }
    }

    #[test]
    pub fn test_replace() {
        test_env::setup();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(1);
        let mut vec = Vector::new(b"v".to_vec());
        let mut baseline = vec![];
        for _ in 0..500 {
            let value = rng.gen::<u64>();
            vec.push(value);
            baseline.push(value);
        }
        for _ in 0..500 {
            let index = rng.gen::<u32>() % vec.len();
            let value = rng.gen::<u64>();
            let old_value0 = vec[index];
            let old_value1 = core::mem::replace(vec.get_mut(index).unwrap(), value);
            let old_value2 = baseline[index as usize];
            assert_eq!(old_value0, old_value1);
            assert_eq!(old_value0, old_value2);
            *baseline.get_mut(index as usize).unwrap() = value;
        }
        let actual: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(actual, baseline);
    }

    #[test]
    pub fn test_swap_remove() {
        test_env::setup();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(2);
        let mut vec = Vector::new(b"v".to_vec());
        let mut baseline = vec![];
        for _ in 0..500 {
            let value = rng.gen::<u64>();
            vec.push(value);
            baseline.push(value);
        }
        for _ in 0..500 {
            let index = rng.gen::<u32>() % vec.len();
            let old_value0 = vec[index];
            let old_value1 = vec.swap_remove(index);
            let old_value2 = baseline[index as usize];
            let last_index = baseline.len() - 1;
            baseline.swap(index as usize, last_index);
            baseline.pop();
            assert_eq!(old_value0, old_value1);
            assert_eq!(old_value0, old_value2);
        }
        let actual: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(actual, baseline);
    }

    #[test]
    pub fn test_clear() {
        test_env::setup();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(3);
        let mut vec = Vector::new(b"v".to_vec());
        for _ in 0..100 {
            for _ in 0..(rng.gen::<u64>() % 20 + 1) {
                let value = rng.gen::<u64>();
                vec.push(value);
            }
            assert!(!vec.is_empty());
            vec.clear();
            assert!(vec.is_empty());
        }
    }

    #[test]
    pub fn test_extend() {
        test_env::setup();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(0);
        let mut vec = Vector::new(b"v".to_vec());
        let mut baseline = vec![];
        for _ in 0..100 {
            let value = rng.gen::<u64>();
            vec.push(value);
            baseline.push(value);
        }

        for _ in 0..100 {
            let mut tmp = vec![];
            for _ in 0..=(rng.gen::<u64>() % 20 + 1) {
                let value = rng.gen::<u64>();
                tmp.push(value);
            }
            baseline.extend(tmp.clone());
            vec.extend(tmp.clone());
        }
        let actual: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(actual, baseline);
    }

    #[test]
    fn test_debug() {
        test_env::setup();
        let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(4);
        let prefix = b"v".to_vec();
        let mut vec = Vector::new(prefix.clone());
        let mut baseline = vec![];
        for _ in 0..10 {
            let value = rng.gen::<u64>();
            vec.push(value);
            baseline.push(value);
        }
        let actual: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(actual, baseline);
        for _ in 0..5 {
            assert_eq!(baseline.pop(), vec.pop());
        }
        if cfg!(feature = "expensive-debug") {
            assert_eq!(format!("{:#?}", vec), format!("{:#?}", baseline));
        } else {
            assert_eq!(
                format!("{:?}", vec),
                format!("Vector {{ len: 5, prefix: {:?} }}", vec.prefix)
            );
        }

        // * The storage is reused in the second part of this test, need to flush
        vec.flush();

        use borsh::{BorshDeserialize, BorshSerialize};
        #[derive(Debug, BorshSerialize, BorshDeserialize)]
        struct TestType(u64);

        let deserialize_only_vec = Vector::<TestType> {
            len: vec.len(),
            prefix: prefix.into_boxed_slice(),
            cache: Default::default(),
        };
        let baseline: Vec<_> = baseline.into_iter().map(|x| TestType(x)).collect();
        if cfg!(feature = "expensive-debug") {
            assert_eq!(format!("{:#?}", deserialize_only_vec), format!("{:#?}", baseline));
        } else {
            assert_eq!(
                format!("{:?}", deserialize_only_vec),
                format!("Vector {{ len: 5, prefix: {:?} }}", deserialize_only_vec.prefix)
            );
        }
    }

    #[test]
    pub fn iterator_checks() {
        test_env::setup();
        let mut vec = Vector::new(b"v");
        let mut baseline = vec![];
        for i in 0..10 {
            vec.push(i);
            baseline.push(i);
        }

        let mut vec_iter = vec.iter();
        let mut bl_iter = baseline.iter();
        assert_eq!(vec_iter.next(), bl_iter.next());
        assert_eq!(vec_iter.next_back(), bl_iter.next_back());
        assert_eq!(vec_iter.nth(3), bl_iter.nth(3));
        assert_eq!(vec_iter.nth_back(2), bl_iter.nth_back(2));

        // Check to make sure indexing overflow is handled correctly
        assert!(vec_iter.nth(5).is_none());
        assert!(bl_iter.nth(5).is_none());

        assert!(vec_iter.next().is_none());
        assert!(bl_iter.next().is_none());

        // Count check
        assert_eq!(vec.iter().count(), baseline.iter().count());
    }
}
