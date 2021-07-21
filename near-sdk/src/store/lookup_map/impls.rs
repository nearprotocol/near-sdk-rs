use core::borrow::Borrow;

use borsh::{BorshDeserialize, BorshSerialize};

use super::{LookupMap, ERR_NOT_EXIST};
use crate::{env, hash::CryptoHash};

impl<K, V, H> Drop for LookupMap<K, V, H>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
    H: CryptoHash<Digest = [u8; 32]>,
{
    fn drop(&mut self) {
        self.flush()
    }
}

impl<K, V, H> Extend<(K, V)> for LookupMap<K, V, H>
where
    K: BorshSerialize + Ord,
    V: BorshSerialize,
    H: CryptoHash<Digest = [u8; 32]>,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (K, V)>,
    {
        for (key, value) in iter {
            self.set(key, Some(value))
        }
    }
}

impl<K, V, H, Q: ?Sized> core::ops::Index<&Q> for LookupMap<K, V, H>
where
    K: BorshSerialize + Ord + Clone + Borrow<Q>,
    V: BorshSerialize + BorshDeserialize,
    H: CryptoHash<Digest = [u8; 32]>,
    Q: BorshSerialize + ToOwned<Owned = K>,
{
    type Output = V;

    /// Returns reference to value corresponding to key.
    ///
    /// # Panics
    ///
    /// Panics if the key does not exist in the map
    fn index(&self, index: &Q) -> &Self::Output {
        self.get(index).unwrap_or_else(|| env::panic(ERR_NOT_EXIST))
    }
}
