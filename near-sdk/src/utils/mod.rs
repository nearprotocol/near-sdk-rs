pub(crate) mod storage_key_impl;

#[cfg(feature = "unstable")]
mod stable_map;
#[cfg(feature = "unstable")]
pub(crate) use self::stable_map::StableMap;
#[cfg(feature = "unstable")]
mod cache_entry;
#[cfg(feature = "unstable")]
pub(crate) use cache_entry::{CacheEntry, EntryState};

use crate::{env, AccountId, PromiseResult};

/// Helper macro to log a message through [`env::log`].
/// This macro can be used similar to the [`std::format`] macro in most cases.
///
/// This differs from [`std::format`] because instead of generating a string, it will log the utf8
/// bytes as a log through the [`BlockchainInterface`].
///
/// The logged message will get persisted on chain.
///
/// # Example use
///
/// ```no_run
/// use near_sdk::log;
///
/// # fn main() {
/// log!("test");
/// let world: &str = "world";
/// log!(world);
/// log!("Hello {}", world);
/// log!("x = {}, y = {y}", 10, y = 30);
/// # }
/// ```
///
/// [`env::log`]: crate::env::log
/// [`BlockchainInterface`]: crate::BlockchainInterface
#[macro_export]
macro_rules! log {
    ($arg:expr) => {
        $crate::env::log_str($arg.as_ref())
    };
    ($($arg:tt)*) => {
        $crate::env::log_str(format!($($arg)*).as_str())
    };
}

/// Assert that predecessor_account_id == current_account_id, meaning contract called itself.
pub fn assert_self() {
    assert_eq!(env::predecessor_account_id(), env::current_account_id(), "Method is private");
}

/// Assert that 1 yoctoNEAR was attached.
pub fn assert_one_yocto() {
    assert_eq!(env::attached_deposit(), 1, "Requires attached deposit of exactly 1 yoctoNEAR")
}

/// Returns true if promise was successful.
/// Fails if called outside a callback that received 1 promise result.
pub fn is_promise_success() -> bool {
    promise_result_as_success().is_some()
}

/// Returns the result of the promise if successful. Otherwise returns None.
/// Fails if called outside a callback that received 1 promise result.
pub fn promise_result_as_success() -> Option<Vec<u8>> {
    assert_eq!(env::promise_results_count(), 1, "Contract expected a result on the callback");
    match env::promise_result(0) {
        PromiseResult::Successful(result) => Some(result),
        _ => None,
    }
}

/// Used in the simulation code generator from near_sdk.
#[derive(Debug)]
pub struct PendingContractTx {
    pub receiver_id: AccountId,
    pub method: String,
    pub args: Vec<u8>,
    pub is_view: bool,
}

impl PendingContractTx {
    pub fn new(
        receiver_id: AccountId,
        method: &str,
        args: serde_json::Value,
        is_view: bool,
    ) -> Self {
        PendingContractTx::new_from_bytes(
            receiver_id,
            method,
            args.to_string().into_bytes(),
            is_view,
        )
    }

    pub fn new_from_bytes(
        receiver_id: AccountId,
        method: &str,
        args: Vec<u8>,
        is_view: bool,
    ) -> Self {
        Self { receiver_id, method: method.to_string(), args, is_view }
    }
}

/// Boilerplate for setting up allocator used in Wasm binary.
/// Sets up the [GlobalAllocator] with [`WeeAlloc`](crate::wee_alloc::WeeAlloc).
///
/// [GlobalAllocator]: std::alloc::GlobalAlloc
#[deprecated(
    since = "4.0.0",
    note = "Allocator is already initialized with the default `wee_alloc` feature set. \
            Please make sure you don't disable default features on the SDK or set the global \
            allocator manually."
)]
#[macro_export]
macro_rules! setup_alloc {
    () => {};
}

#[cfg(test)]
mod tests {
    use crate::test_utils::{get_logs, test_env};

    #[test]
    fn test_log_simple() {
        test_env::setup();
        log!("hello");

        assert_eq!(get_logs(), vec!["hello".to_string()]);
    }

    #[test]
    fn test_log_format() {
        test_env::setup();
        log!("hello {} ({})", "user_name", 25);

        assert_eq!(get_logs(), vec!["hello user_name (25)".to_string()]);
    }
}
