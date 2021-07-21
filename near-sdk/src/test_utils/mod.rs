#[allow(dead_code)]
pub mod test_env;

mod context;
use crate::mock::Receipt;
pub use context::{accounts, testing_env_with_promise_results, VMContextBuilder};

/// Initializes a testing environment to mock interactions which would otherwise go through a
/// validator node. This macro will initialize or overwrite the [`MockedBlockchain`]
/// instance for interactions from a smart contract.
///
/// There are five parameters that can be accepted to configure the interface with a
/// [`MockedBlockchain`], in this order:
/// - `context`: [`VMContext`] which contains some core information about
/// the blockchain and message data which can be used from the smart contract.
/// - `config` (optional): [`VMConfig`] which contains some additional information
/// about the VM to configure parameters not directly related to the transaction being executed.
/// - `fee_config`(optional): [`RuntimeFeesConfig`] which configures the
/// fees for execution and storage of transactions.
/// - `validators`(optional): a [`HashMap`]<[`AccountId`], [`Balance`]> mocking the
/// current validators of the blockchain.
/// - `promise_results`(optional): a [`Vec`] of [`PromiseResult`] which mocks the results
/// of callback calls during the execution.
///
/// Any argument not included will use the default implementation of each.
///
/// # Example use
///
/// ```
/// use near_sdk::testing_env;
/// use near_sdk::test_utils::{accounts, VMContextBuilder};
/// use near_sdk::{VMConfig, RuntimeFeesConfig};
/// use std::collections::HashMap;
///
/// # fn main() {
/// // Initializing some context is required
/// let context = VMContextBuilder::new().signer_account_id(accounts(0)).build();
///
/// // Build with just the base context
/// testing_env!(context.clone());
///
/// // Or include arguments up to the five optional
/// testing_env!(
///     context,
///     VMConfig::default(),
///     RuntimeFeesConfig::default(),
///     HashMap::default(),
///     Vec::default(),
/// );
/// # }
/// ```
///
/// [`MockedBlockchain`]: crate::mock::MockedBlockchain
/// [`VMContext`]: crate::VMContext
/// [`VMConfig`]: crate::VMConfig
/// [`RuntimeFeesConfig`]: crate::RuntimeFeesConfig
/// [`AccountId`]: crate::AccountId
/// [`Balance`]: crate::Balance
/// [`PromiseResult`]: crate::PromiseResult
/// [`HashMap`]: std::collections::HashMap
#[macro_export]
macro_rules! testing_env {
    ($context:expr, $config:expr, $fee_config:expr, $validators:expr, $promise_results:expr $(,)?) => {
        $crate::env::set_blockchain_interface($crate::MockedBlockchain::new(
            $context,
            $config,
            $fee_config,
            $promise_results,
            $crate::mock::with_mocked_blockchain(|b| b.take_storage()),
            $validators,
            None,
        ));
    };
    ($context:expr, $config:expr, $fee_config:expr, $validators:expr $(,)?) => {
        $crate::testing_env!($context, $config, $fee_config, $validators, Default::default());
    };

    ($context:expr, $config:expr, $fee_config:expr $(,)?) => {
        $crate::testing_env!($context, $config, $fee_config, Default::default());
    };
    ($context:expr) => {
        $crate::testing_env!($context, Default::default(), Default::default());
    };
}

#[allow(dead_code)]
/// Returns a copy of logs from VMLogic. Only available in unit tests.
pub fn get_logs() -> Vec<String> {
    crate::env::BLOCKCHAIN_INTERFACE.with(|b| b.borrow().logs())
}

/// Accessing receipts created by the contract. Only available in unit tests.
#[allow(dead_code)]
pub fn get_created_receipts() -> Vec<Receipt> {
    crate::env::BLOCKCHAIN_INTERFACE.with(|b| b.borrow().created_receipts().clone())
}

/// Objects stored on the trie directly should have identifiers. If identifier is not provided
/// explicitly than `Default` trait would use this index to generate an id.
#[allow(dead_code)]
pub(crate) static mut NEXT_TRIE_OBJECT_INDEX: u64 = 0;
/// Get next id of the object stored on trie.
#[allow(dead_code)]
pub(crate) fn next_trie_id() -> Vec<u8> {
    unsafe {
        let id = NEXT_TRIE_OBJECT_INDEX;
        NEXT_TRIE_OBJECT_INDEX += 1;
        id.to_le_bytes().to_vec()
    }
}
