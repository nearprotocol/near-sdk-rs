mod mocked_blockchain;

pub use self::mocked_blockchain::MockedBlockchain;

/// Perform function on a mutable reference to the [`MockedBlockchain`]. This can only be used
/// inside tests.
pub fn with_mocked_blockchain<F, R>(f: F) -> R
where
    F: FnOnce(&mut MockedBlockchain) -> R,
{
    super::env::BLOCKCHAIN_INTERFACE.with(|b| f(&mut b.borrow_mut()))
}
