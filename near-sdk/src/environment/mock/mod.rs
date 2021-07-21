mod external;
mod mocked_blockchain;
mod receipt;

pub(crate) use self::external::SdkExternal;
pub use self::mocked_blockchain::MockedBlockchain;
pub use self::receipt::{Receipt, VmAction};

/// Perform function on a mutable reference to the [`MockedBlockchain`]. This can only be used
/// inside tests.
pub fn with_mocked_blockchain<F, R>(f: F) -> R
where
    F: FnOnce(&mut MockedBlockchain) -> R,
{
    super::env::BLOCKCHAIN_INTERFACE.with(|b| f(&mut b.borrow_mut()))
}
