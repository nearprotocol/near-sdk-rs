//! Blockchain-specific methods available to the smart contract. This is a wrapper around a
//! low-level `BlockchainInterface`. Unless you know what you are doing prefer using `env::*`
//! whenever possible. In case of cross-contract calls prefer using even higher-level API available
//! through `callback_args`, `callback_args_vec`, `ext_contract`, `Promise`, and `PromiseOrValue`.

use std::borrow::Borrow;
#[cfg(not(target_arch = "wasm32"))]
use std::cell::RefCell;
use std::convert::TryFrom;
use std::mem::size_of;
use std::panic as std_panic;

use super::sys;
#[cfg(not(target_arch = "wasm32"))]
use crate::mock::MockedBlockchain;
use crate::types::{
    AccountId, Balance, BlockHeight, Gas, PromiseIndex, PromiseResult, PublicKey, StorageUsage,
};

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
/// Low-level blockchain interface wrapped by the environment. Prefer using `env::*` and `testing_env`
/// for interacting with the real and fake blockchains.
    pub(crate) static BLOCKCHAIN_INTERFACE: RefCell<MockedBlockchain>
         = RefCell::new(MockedBlockchain::default());
}

const REGISTER_EXPECTED_ERR: &str =
    "Register was expected to have data because we just wrote it into it.";

/// Register used internally for atomic operations. This register is safe to use by the user,
/// since it only needs to be untouched while methods of `Environment` execute, which is guaranteed
/// guest code is not parallel.
const ATOMIC_OP_REGISTER: u64 = 0;
/// Register used to record evicted values from the storage.
const EVICTED_REGISTER: u64 = std::u64::MAX - 1;

/// Key used to store the state of the contract.
const STATE_KEY: &[u8] = b"STATE";

/// The minimum length of a valid account ID.
const MIN_ACCOUNT_ID_LEN: u64 = 2;
/// The maximum length of a valid account ID.
const MAX_ACCOUNT_ID_LEN: u64 = 64;

/// A simple macro helper to read blob value coming from host's method.
macro_rules! try_method_into_register {
    ( $method:ident ) => {{
        unsafe { sys::$method(ATOMIC_OP_REGISTER) };
        read_register(ATOMIC_OP_REGISTER)
    }};
}

/// Same as `try_method_into_register` but expects the data.
macro_rules! method_into_register {
    ( $method:ident ) => {{
        try_method_into_register!($method).expect(REGISTER_EXPECTED_ERR)
    }};
}

/// Replaces the current low-level blockchain interface accessible through `env::*` with another
/// low-level blockchain interfacr that implements `BlockchainInterface` trait. In most cases you
/// want to use `testing_env!` macro to set it.
///
/// ```ignore
/// # let context = Default::default();
/// # let vm_config = Default::default();
/// # let fees_config = Default::default();
/// # let storage = Default::default();
/// let mocked_blockchain = near_sdk::MockedBlockchain::new(
///           context,
///           vm_config,
///           fees_config,
///           vec![],
///           storage,
///           None,
///       );
/// near_sdk::env::set_blockchain_interface(mocked_blockchain);
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn set_blockchain_interface(blockchain_interface: MockedBlockchain) {
    BLOCKCHAIN_INTERFACE.with(|b| {
        *b.borrow_mut() = blockchain_interface;
    })
}

/// Implements panic hook that converts `PanicInfo` into a string and provides it through the
/// blockchain interface.
fn panic_hook_impl(info: &std_panic::PanicInfo) {
    panic(info.to_string().as_bytes());
}

/// Setups panic hook to expose error info to the blockchain.
pub fn setup_panic_hook() {
    std_panic::set_hook(Box::new(panic_hook_impl));
}

/// Reads the content of the `register_id`. If register is not used returns `None`.
pub fn read_register(register_id: u64) -> Option<Vec<u8>> {
    let len = register_len(register_id)?;
    let res = vec![0u8; len as usize];
    unsafe { sys::read_register(register_id, res.as_ptr() as _) };
    Some(res)
}

/// Returns the size of the register. If register is not used returns `None`.
pub fn register_len(register_id: u64) -> Option<u64> {
    let len = unsafe { sys::register_len(register_id) };
    if len == std::u64::MAX {
        None
    } else {
        Some(len)
    }
}

// ###############
// # Context API #
// ###############
/// The id of the account that owns the current contract.
pub fn current_account_id() -> AccountId {
    assert_valid_account_id(method_into_register!(current_account_id))
}

/// The id of the account that either signed the original transaction or issued the initial
/// cross-contract call.
pub fn signer_account_id() -> AccountId {
    assert_valid_account_id(method_into_register!(signer_account_id))
}

/// The public key of the account that did the signing.
pub fn signer_account_pk() -> PublicKey {
    PublicKey::try_from(method_into_register!(signer_account_pk)).unwrap_or_else(|_| unreachable!())
}

/// The id of the account that was the previous contract in the chain of cross-contract calls.
/// If this is the first contract, it is equal to `signer_account_id`.
pub fn predecessor_account_id() -> AccountId {
    assert_valid_account_id(method_into_register!(predecessor_account_id))
}

/// Helper function to convert and check the account ID from bytes from the runtime.
fn assert_valid_account_id(bytes: Vec<u8>) -> AccountId {
    String::from_utf8(bytes)
        .ok()
        .and_then(|s| AccountId::try_from(s).ok())
        .unwrap_or_else(|| unreachable!())
}

/// The input to the contract call serialized as bytes. If input is not provided returns `None`.
pub fn input() -> Option<Vec<u8>> {
    try_method_into_register!(input)
}

/// Current block index.
#[deprecated(since = "4.0.0", note = "Use block_height instead")]
pub fn block_index() -> BlockHeight {
    block_height()
}

/// Returns the height of the block the transaction is being executed in.
pub fn block_height() -> BlockHeight {
    unsafe { sys::block_height() }
}

/// Current block timestamp, i.e, number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
pub fn block_timestamp() -> u64 {
    unsafe { sys::block_timestamp() }
}

/// Current epoch height.
pub fn epoch_height() -> u64 {
    unsafe { sys::epoch_height() }
}

/// Current total storage usage of this smart contract that this account would be paying for.
pub fn storage_usage() -> StorageUsage {
    unsafe { sys::storage_usage() }
}

// #################
// # Economics API #
// #################
/// The balance attached to the given account. This includes the attached_deposit that was
/// attached to the transaction
pub fn account_balance() -> Balance {
    let data = [0u8; size_of::<Balance>()];
    unsafe { sys::account_balance(data.as_ptr() as u64) };
    Balance::from_le_bytes(data)
}

/// The balance locked for potential validator staking.
pub fn account_locked_balance() -> Balance {
    let data = [0u8; size_of::<Balance>()];
    unsafe { sys::account_locked_balance(data.as_ptr() as u64) };
    Balance::from_le_bytes(data)
}

/// The balance that was attached to the call that will be immediately deposited before the
/// contract execution starts
pub fn attached_deposit() -> Balance {
    let data = [0u8; size_of::<Balance>()];
    unsafe { sys::attached_deposit(data.as_ptr() as u64) };
    Balance::from_le_bytes(data)
}

/// The amount of gas attached to the call that can be used to pay for the gas fees.
pub fn prepaid_gas() -> Gas {
    Gas(unsafe { sys::prepaid_gas() })
}

/// The gas that was already burnt during the contract execution (cannot exceed `prepaid_gas`)
pub fn used_gas() -> Gas {
    Gas(unsafe { sys::used_gas() })
}

// ############
// # Math API #
// ############
/// Get random seed from the register.
pub fn random_seed() -> Vec<u8> {
    method_into_register!(random_seed)
}

/// Hashes the random sequence of bytes using sha256.
pub fn sha256(value: &[u8]) -> Vec<u8> {
    unsafe { sys::sha256(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER) };
    read_register(ATOMIC_OP_REGISTER).expect(REGISTER_EXPECTED_ERR)
}

/// Hashes the random sequence of bytes using keccak256.
pub fn keccak256(value: &[u8]) -> Vec<u8> {
    unsafe { sys::keccak256(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER) };
    read_register(ATOMIC_OP_REGISTER).expect(REGISTER_EXPECTED_ERR)
}

/// Hashes the random sequence of bytes using keccak512.
pub fn keccak512(value: &[u8]) -> Vec<u8> {
    unsafe { sys::keccak512(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER) };
    read_register(ATOMIC_OP_REGISTER).expect(REGISTER_EXPECTED_ERR)
}

// ################
// # Promises API #
// ################
/// Creates a promise that will execute a method on account with given arguments and attaches
/// the given amount and gas.
pub fn promise_create(
    account_id: AccountId,
    method_name: &[u8],
    arguments: &[u8],
    amount: Balance,
    gas: Gas,
) -> PromiseIndex {
    let account_id = account_id.as_bytes();
    unsafe {
        sys::promise_create(
            account_id.len() as _,
            account_id.as_ptr() as _,
            method_name.len() as _,
            method_name.as_ptr() as _,
            arguments.len() as _,
            arguments.as_ptr() as _,
            &amount as *const Balance as _,
            gas.0,
        )
    }
}

/// Attaches the callback that is executed after promise pointed by `promise_idx` is complete.
pub fn promise_then(
    promise_idx: PromiseIndex,
    account_id: AccountId,
    method_name: &[u8],
    arguments: &[u8],
    amount: Balance,
    gas: Gas,
) -> PromiseIndex {
    let account_id = account_id.as_bytes();
    unsafe {
        sys::promise_then(
            promise_idx,
            account_id.len() as _,
            account_id.as_ptr() as _,
            method_name.len() as _,
            method_name.as_ptr() as _,
            arguments.len() as _,
            arguments.as_ptr() as _,
            &amount as *const Balance as _,
            gas.0,
        )
    }
}

/// Creates a new promise which completes when time all promises passed as arguments complete.
pub fn promise_and(promise_indices: &[PromiseIndex]) -> PromiseIndex {
    let mut data = vec![0u8; promise_indices.len() * size_of::<PromiseIndex>()];
    for i in 0..promise_indices.len() {
        data[i * size_of::<PromiseIndex>()..(i + 1) * size_of::<PromiseIndex>()]
            .copy_from_slice(&promise_indices[i].to_le_bytes());
    }
    unsafe { sys::promise_and(data.as_ptr() as _, promise_indices.len() as _) }
}

pub fn promise_batch_create(account_id: &AccountId) -> PromiseIndex {
    let account_id = account_id.as_ref();
    unsafe { sys::promise_batch_create(account_id.len() as _, account_id.as_ptr() as _) }
}

pub fn promise_batch_then(promise_index: PromiseIndex, account_id: &AccountId) -> PromiseIndex {
    let account_id: &str = account_id.as_ref();
    unsafe {
        sys::promise_batch_then(promise_index, account_id.len() as _, account_id.as_ptr() as _)
    }
}

pub fn promise_batch_action_create_account(promise_index: PromiseIndex) {
    unsafe { sys::promise_batch_action_create_account(promise_index) }
}

pub fn promise_batch_action_deploy_contract(promise_index: u64, code: &[u8]) {
    unsafe {
        sys::promise_batch_action_deploy_contract(
            promise_index,
            code.len() as _,
            code.as_ptr() as _,
        )
    }
}

pub fn promise_batch_action_function_call(
    promise_index: PromiseIndex,
    method_name: &[u8],
    arguments: &[u8],
    amount: Balance,
    gas: Gas,
) {
    unsafe {
        sys::promise_batch_action_function_call(
            promise_index,
            method_name.len() as _,
            method_name.as_ptr() as _,
            arguments.len() as _,
            arguments.as_ptr() as _,
            &amount as *const Balance as _,
            gas.0,
        )
    }
}

pub fn promise_batch_action_transfer(promise_index: PromiseIndex, amount: Balance) {
    unsafe { sys::promise_batch_action_transfer(promise_index, &amount as *const Balance as _) }
}

pub fn promise_batch_action_stake<P: Borrow<PublicKey>>(
    promise_index: PromiseIndex,
    amount: Balance,
    public_key: P,
) {
    let public_key = public_key.borrow();
    unsafe {
        sys::promise_batch_action_stake(
            promise_index,
            &amount as *const Balance as _,
            public_key.as_bytes().len() as _,
            public_key.as_bytes().as_ptr() as _,
        )
    }
}
pub fn promise_batch_action_add_key_with_full_access<P: Borrow<PublicKey>>(
    promise_index: PromiseIndex,
    public_key: P,
    nonce: u64,
) {
    let public_key = public_key.borrow();
    unsafe {
        sys::promise_batch_action_add_key_with_full_access(
            promise_index,
            public_key.as_bytes().len() as _,
            public_key.as_bytes().as_ptr() as _,
            nonce,
        )
    }
}
pub fn promise_batch_action_add_key_with_function_call<P: Borrow<PublicKey>>(
    promise_index: PromiseIndex,
    public_key: P,
    nonce: u64,
    allowance: Balance,
    receiver_id: &AccountId,
    method_names: &[u8],
) {
    let public_key = public_key.borrow();
    let receiver_id: &str = receiver_id.as_ref();
    unsafe {
        sys::promise_batch_action_add_key_with_function_call(
            promise_index,
            public_key.as_bytes().len() as _,
            public_key.as_bytes().as_ptr() as _,
            nonce,
            &allowance as *const Balance as _,
            receiver_id.len() as _,
            receiver_id.as_ptr() as _,
            method_names.len() as _,
            method_names.as_ptr() as _,
        )
    }
}
pub fn promise_batch_action_delete_key<P: Borrow<PublicKey>>(
    promise_index: PromiseIndex,
    public_key: P,
) {
    let public_key = public_key.borrow();
    unsafe {
        sys::promise_batch_action_delete_key(
            promise_index,
            public_key.as_bytes().len() as _,
            public_key.as_bytes().as_ptr() as _,
        )
    }
}

pub fn promise_batch_action_delete_account(
    promise_index: PromiseIndex,
    beneficiary_id: &AccountId,
) {
    let beneficiary_id: &str = beneficiary_id.as_ref();
    unsafe {
        sys::promise_batch_action_delete_account(
            promise_index,
            beneficiary_id.len() as _,
            beneficiary_id.as_ptr() as _,
        )
    }
}

/// If the current function is invoked by a callback we can access the execution results of the
/// promises that caused the callback. This function returns the number of complete and
/// incomplete callbacks.
pub fn promise_results_count() -> u64 {
    unsafe { sys::promise_results_count() }
}
/// If the current function is invoked by a callback we can access the execution results of the
/// promises that caused the callback.
pub fn promise_result(result_idx: u64) -> PromiseResult {
    match unsafe { sys::promise_result(result_idx, ATOMIC_OP_REGISTER) } {
        0 => PromiseResult::NotReady,
        1 => {
            let data = read_register(ATOMIC_OP_REGISTER)
                .expect("Promise result should've returned into register.");
            PromiseResult::Successful(data)
        }
        2 => PromiseResult::Failed,
        _ => unreachable!(),
    }
}
/// Consider the execution result of promise under `promise_idx` as execution result of this
/// function.
pub fn promise_return(promise_idx: PromiseIndex) {
    unsafe { sys::promise_return(promise_idx) }
}

// ###############
// # Validator API #
// ###############

/// For a given account return its current stake. If the account is not a validator, returns 0.
pub fn validator_stake(account_id: &AccountId) -> Balance {
    let account_id: &str = account_id.as_ref();
    let data = [0u8; size_of::<Balance>()];
    unsafe {
        sys::validator_stake(account_id.len() as _, account_id.as_ptr() as _, data.as_ptr() as u64)
    };
    Balance::from_le_bytes(data)
}

/// Returns the total stake of validators in the current epoch.
pub fn validator_total_stake() -> Balance {
    let data = [0u8; size_of::<Balance>()];
    unsafe { sys::validator_total_stake(data.as_ptr() as u64) };
    Balance::from_le_bytes(data)
}

// #####################
// # Miscellaneous API #
// #####################
/// Sets the blob of data as the return value of the contract.
pub fn value_return(value: &[u8]) {
    unsafe { sys::value_return(value.len() as _, value.as_ptr() as _) }
}
/// Terminates the execution of the program with the UTF-8 encoded message.
pub fn panic(message: &[u8]) -> ! {
    unsafe { sys::panic_utf8(message.len() as _, message.as_ptr() as _) }
}
/// Logs the string message message. This message is stored on chain.
pub fn log_str(message: &str) {
    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
    eprintln!("{}", message);

    unsafe { sys::log_utf8(message.len() as _, message.as_ptr() as _) }
}

/// Log the UTF-8 encodable message.
#[deprecated(since = "4.0.0", note = "Use env::log_str for logging messages.")]
pub fn log(message: &[u8]) {
    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
    eprintln!("{}", String::from_utf8_lossy(message));

    unsafe { sys::log_utf8(message.len() as _, message.as_ptr() as _) }
}

// ###############
// # Storage API #
// ###############
/// Writes key-value into storage.
/// If another key-value existed in the storage with the same key it returns `true`, otherwise `false`.
pub fn storage_write(key: &[u8], value: &[u8]) -> bool {
    match unsafe {
        sys::storage_write(
            key.len() as _,
            key.as_ptr() as _,
            value.len() as _,
            value.as_ptr() as _,
            EVICTED_REGISTER,
        )
    } {
        0 => false,
        1 => true,
        _ => unreachable!(),
    }
}
/// Reads the value stored under the given key.
pub fn storage_read(key: &[u8]) -> Option<Vec<u8>> {
    match unsafe { sys::storage_read(key.len() as _, key.as_ptr() as _, ATOMIC_OP_REGISTER) } {
        0 => None,
        1 => Some(read_register(ATOMIC_OP_REGISTER).expect(REGISTER_EXPECTED_ERR)),
        _ => unreachable!(),
    }
}
/// Removes the value stored under the given key.
/// If key-value existed returns `true`, otherwise `false`.
pub fn storage_remove(key: &[u8]) -> bool {
    match unsafe { sys::storage_remove(key.len() as _, key.as_ptr() as _, EVICTED_REGISTER) } {
        0 => false,
        1 => true,
        _ => unreachable!(),
    }
}
/// Reads the most recent value that was evicted with `storage_write` or `storage_remove` command.
pub fn storage_get_evicted() -> Option<Vec<u8>> {
    read_register(EVICTED_REGISTER)
}
/// Checks if there is a key-value in the storage.
pub fn storage_has_key(key: &[u8]) -> bool {
    match unsafe { sys::storage_has_key(key.len() as _, key.as_ptr() as _) } {
        0 => false,
        1 => true,
        _ => unreachable!(),
    }
}

// ############################################
// # Saving and loading of the contract state #
// ############################################
/// Load the state of the given object.
pub fn state_read<T: borsh::BorshDeserialize>() -> Option<T> {
    storage_read(STATE_KEY)
        .map(|data| T::try_from_slice(&data).expect("Cannot deserialize the contract state."))
}

pub fn state_write<T: borsh::BorshSerialize>(state: &T) {
    let data = state.try_to_vec().expect("Cannot serialize the contract state.");
    storage_write(STATE_KEY, &data);
}

/// Returns `true` if the contract state exists and `false` otherwise.
pub fn state_exists() -> bool {
    storage_has_key(STATE_KEY)
}

// #####################################
// # Parameters exposed by the runtime #
// #####################################

/// Price per 1 byte of storage from mainnet genesis config.
/// TODO: will be using the host function when it will be available.
pub const STORAGE_PRICE_PER_BYTE: Balance = 10_000_000_000_000_000_000;

pub fn storage_byte_cost() -> Balance {
    STORAGE_PRICE_PER_BYTE
}

// ##################
// # Helper methods #
// ##################

/// Returns `true` if the given account ID is valid and `false` otherwise.
pub fn is_valid_account_id(account_id: &[u8]) -> bool {
    if (account_id.len() as u64) < MIN_ACCOUNT_ID_LEN
        || (account_id.len() as u64) > MAX_ACCOUNT_ID_LEN
    {
        return false;
    }

    // NOTE: We don't want to use Regex here, because it requires extra time to compile it.
    // The valid account ID regex is /^(([a-z\d]+[-_])*[a-z\d]+\.)*([a-z\d]+[-_])*[a-z\d]+$/
    // Instead the implementation is based on the previous character checks.

    // We can safely assume that last char was a separator.
    let mut last_char_is_separator = true;

    for c in account_id {
        let current_char_is_separator = match *c {
            b'a'..=b'z' | b'0'..=b'9' => false,
            b'-' | b'_' | b'.' => true,
            _ => return false,
        };
        if current_char_is_separator && last_char_is_separator {
            return false;
        }
        last_char_is_separator = current_char_is_separator;
    }
    // The account can't end as separator.
    !last_char_is_separator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_account_id_strings() {
        // Valid
        for account_id in &[
            "aa",
            "a-a",
            "a-aa",
            "100",
            "0o",
            "com",
            "near",
            "bowen",
            "b-o_w_e-n",
            "b.owen",
            "bro.wen",
            "a.ha",
            "a.b-a.ra",
            "system",
            "over.9000",
            "google.com",
            "illia.cheapaccounts.near",
            "0o0ooo00oo00o",
            "alex-skidanov",
            "10-4.8-2",
            "b-o_w_e-n",
            "no_lols",
            "0123456789012345678901234567890123456789012345678901234567890123",
            // Valid, but can't be created
            "near.a",
            "a.a",
        ] {
            assert!(
                is_valid_account_id(account_id.as_ref()),
                "Valid account id {:?} marked invalid",
                account_id
            );
        }

        // Invalid
        for account_id in &[
            "",
            "a",
            "A",
            "Abc",
            "-near",
            "near-",
            "-near-",
            "near.",
            ".near",
            "near@",
            "@near",
            "неар",
            "@@@@@",
            "0__0",
            "0_-_0",
            "0_-_0",
            "..",
            "a..near",
            "nEar",
            "_bowen",
            "hello world",
            "abcdefghijklmnopqrstuvwxyz.abcdefghijklmnopqrstuvwxyz.abcdefghijklmnopqrstuvwxyz",
            "01234567890123456789012345678901234567890123456789012345678901234",
            // `@` separators are banned now
            "some-complex-address@gmail.com",
            "sub.buy_d1gitz@atata@b0-rg.c_0_m",
        ] {
            assert!(
                !is_valid_account_id(account_id.as_ref()),
                "Invalid account id {:?} marked valid",
                account_id
            );
        }
    }

    #[test]
    fn test_is_valid_account_id_binary() {
        assert!(!is_valid_account_id(&[]));
        assert!(!is_valid_account_id(&[0]));
        assert!(!is_valid_account_id(&[0, 1]));
        assert!(!is_valid_account_id(&[0, 1, 2]));
        assert!(is_valid_account_id(b"near"));
    }
}
