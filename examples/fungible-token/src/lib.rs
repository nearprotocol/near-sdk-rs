/**
* Fungible Token implementation with JSON serialization.
* NOTES:
*  - The maximum balance value is limited by U128 (2**128 - 1).
*  - JSON calls should pass U128 as a base-10 string. E.g. "100".
*  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
*    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
*  - This contract doesn't optimize the amount of storage, since any account can create unlimited
*    amount of allowances to other accounts. It's unclear how to address this issue unless, this
*    contract limits the total number of different allowances possible at the same time.
*    And even if it limits the total number, it's still possible to transfer small amounts to
*    multiple accounts.
*/
use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::U128;
use near_sdk::{env, near_bindgen, AccountId, Balance, Promise, StorageUsage};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// Price per 1 byte of storage from mainnet genesis config.
const STORAGE_PRICE_PER_BYTE: Balance = 100000000000000000000;

/// Contains balance and allowances information for one account.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Account {
    /// Current account balance.
    pub balance: Balance,
    /// Escrow Account ID hash to the allowance amount.
    /// Allowance is the amount of tokens the Escrow Account ID can spent on behalf of the account
    /// owner.
    pub allowances: UnorderedMap<Vec<u8>, Balance>,
}

impl Account {
    /// Initializes a new Account with 0 balance and no allowances for a given `account_hash`.
    pub fn new(account_hash: Vec<u8>) -> Self {
        Self { balance: 0, allowances: UnorderedMap::new(account_hash) }
    }

    /// Sets allowance for account `escrow_account_id` to `allowance`.
    pub fn set_allowance(&mut self, escrow_account_id: &AccountId, allowance: Balance) {
        let escrow_hash = env::sha256(escrow_account_id.as_bytes());
        if allowance > 0 {
            self.allowances.insert(&escrow_hash, &allowance);
        } else {
            self.allowances.remove(&escrow_hash);
        }
    }

    /// Returns the allowance of account `escrow_account_id`.
    pub fn get_allowance(&self, escrow_account_id: &AccountId) -> Balance {
        let escrow_hash = env::sha256(escrow_account_id.as_bytes());
        self.allowances.get(&escrow_hash).unwrap_or(0)
    }
}

//
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct FungibleToken {
    /// sha256(AccountID) -> Account details.
    pub accounts: UnorderedMap<Vec<u8>, Account>,

    /// Total supply of the all token.
    pub total_supply: Balance,
}

impl Default for FungibleToken {
    fn default() -> Self {
        panic!("Fun token should be initialized before usage")
    }
}

#[near_bindgen]
impl FungibleToken {
    /// Initializes the contract with the given total supply owned by the given `owner_id`.
    #[init]
    pub fn new(owner_id: AccountId, total_supply: U128) -> Self {
        let total_supply = total_supply.into();
        assert!(!env::state_exists(), "Already initialized");
        let mut ft = Self { accounts: UnorderedMap::new(b"a".to_vec()), total_supply };
        let mut account = ft.get_account(&owner_id);
        account.balance = total_supply;
        ft.set_account(&owner_id, &account);
        ft
    }

    /// Sets the `allowance` for `escrow_account_id` on the account of the caller of this contract
    /// (`predecessor_id`) who is the balance owner.
    /// Requirements:
    /// * Caller of the method has to attach deposit enough to cover storage difference at the
    ///   fixed storage price defined in the contract.
    #[payable]
    pub fn set_allowance(&mut self, escrow_account_id: AccountId, allowance: U128) {
        let initial_storage = env::storage_usage();
        assert!(
            env::is_valid_account_id(escrow_account_id.as_bytes()),
            "Escrow account ID is invalid"
        );
        let allowance = allowance.into();
        let owner_id = env::predecessor_account_id();
        if escrow_account_id == owner_id {
            env::panic(b"Can not set allowance for yourself");
        }
        let mut account = self.get_account(&owner_id);

        account.set_allowance(&escrow_account_id, allowance);
        self.set_account(&owner_id, &account);
        self.refund_storage(initial_storage);
    }

    /// Transfers the `amount` of tokens from `owner_id` to the `new_owner_id`.
    /// Requirements:
    /// * `amount` should be a positive integer.
    /// * `owner_id` should have balance on the account greater or equal than the transfer `amount`.
    /// * If this function is called by an escrow account (`owner_id != predecessor_account_id`),
    ///   then the allowance of the caller of the function (`predecessor_account_id`) on
    ///   the account of `owner_id` should be greater or equal than the transfer `amount`.
    /// * Caller of the method has to attach deposit enough to cover storage difference at the
    ///   fixed storage price defined in the contract.
    #[payable]
    pub fn transfer_from(&mut self, owner_id: AccountId, new_owner_id: AccountId, amount: U128) {
        let initial_storage = env::storage_usage();
        assert!(
            env::is_valid_account_id(new_owner_id.as_bytes()),
            "New owner's account ID is invalid"
        );
        let amount = amount.into();
        if amount == 0 {
            env::panic(b"Can't transfer 0 tokens");
        }
        // Retrieving the account from the state.
        let mut account = self.get_account(&owner_id);

        // Checking and updating unlocked balance
        if account.balance < amount {
            env::panic(b"Not enough balance");
        }
        account.balance -= amount;

        // If transferring by escrow, need to check and update allowance.
        let escrow_account_id = env::predecessor_account_id();
        if escrow_account_id != owner_id {
            let allowance = account.get_allowance(&escrow_account_id);
            if allowance < amount {
                env::panic(b"Not enough allowance");
            }
            account.set_allowance(&escrow_account_id, allowance - amount);
        }

        // Saving the account back to the state.
        self.set_account(&owner_id, &account);

        // Deposit amount to the new owner and save the new account to the state.
        let mut new_account = self.get_account(&new_owner_id);
        new_account.balance += amount;
        self.set_account(&new_owner_id, &new_account);
        self.refund_storage(initial_storage);
    }

    /// Transfer `amount` of tokens from the caller of the contract (`predecessor_id`) to
    /// `new_owner_id`.
    /// Act the same was as `transfer_from` with `owner_id` equal to the caller of the contract
    /// (`predecessor_id`).
    /// Requirements:
    /// * Caller of the method has to attach deposit enough to cover storage difference at the
    ///   fixed storage price defined in the contract.
    #[payable]
    pub fn transfer(&mut self, new_owner_id: AccountId, amount: U128) {
        // NOTE: New owner's Account ID checked in transfer_from.
        // Storage fees are also refunded in transfer_from.
        self.transfer_from(env::predecessor_account_id(), new_owner_id, amount);
    }

    /// Returns total supply of tokens.
    pub fn get_total_supply(&self) -> U128 {
        self.total_supply.into()
    }

    /// Returns balance of the `owner_id` account.
    pub fn get_balance(&self, owner_id: AccountId) -> U128 {
        self.get_account(&owner_id).balance.into()
    }

    /// Returns current allowance of `escrow_account_id` for the account of `owner_id`.
    ///
    /// NOTE: Other contracts should not rely on this information, because by the moment a contract
    /// receives this information, the allowance may already be changed by the owner.
    /// So this method should only be used on the front-end to see the current allowance.
    pub fn get_allowance(&self, owner_id: AccountId, escrow_account_id: AccountId) -> U128 {
        assert!(
            env::is_valid_account_id(escrow_account_id.as_bytes()),
            "Escrow account ID is invalid"
        );
        self.get_account(&owner_id).get_allowance(&escrow_account_id).into()
    }
}

impl FungibleToken {
    /// Helper method to get the account details for `owner_id`.
    fn get_account(&self, owner_id: &AccountId) -> Account {
        assert!(env::is_valid_account_id(owner_id.as_bytes()), "Owner's account ID is invalid");
        let account_hash = env::sha256(owner_id.as_bytes());
        self.accounts.get(&account_hash).unwrap_or_else(|| Account::new(account_hash))
    }

    /// Helper method to set the account details for `owner_id` to the state.
    fn set_account(&mut self, owner_id: &AccountId, account: &Account) {
        let account_hash = env::sha256(owner_id.as_bytes());
        self.accounts.insert(&account_hash, &account);
    }

    fn refund_storage(&self, initial_storage: StorageUsage) {
        let current_storage = env::storage_usage();
        let attached_deposit = env::attached_deposit();
        let refund_amount = if current_storage > initial_storage {
            let required_deposit =
                Balance::from(current_storage - initial_storage) * STORAGE_PRICE_PER_BYTE;
            assert!(
                required_deposit <= attached_deposit,
                "The required attached deposit is {}, but the given attached deposit is is {}",
                required_deposit,
                attached_deposit,
            );
            attached_deposit - required_deposit
        } else {
            attached_deposit
                + Balance::from(initial_storage - current_storage) * STORAGE_PRICE_PER_BYTE
        };
        if refund_amount > 0 {
            env::log(format!("Refunding {} tokens for storage", refund_amount).as_bytes());
            Promise::new(env::predecessor_account_id()).transfer(refund_amount);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    use super::*;

    fn alice() -> AccountId {
        "alice.near".to_string()
    }
    fn bob() -> AccountId {
        "bob.near".to_string()
    }
    fn carol() -> AccountId {
        "carol.near".to_string()
    }

    fn get_context(predecessor_account_id: AccountId) -> VMContext {
        VMContext {
            current_account_id: alice(),
            signer_account_id: bob(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id,
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 1_000_000_000_000_000_000_000_000_000u128,
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
            epoch_height: 0,
        }
    }

    #[test]
    fn test_new() {
        let context = get_context(carol());
        testing_env!(context);
        let total_supply = 1_000_000_000_000_000u128;
        let contract = FungibleToken::new(bob(), total_supply.into());
        assert_eq!(contract.get_total_supply().0, total_supply);
        assert_eq!(contract.get_balance(bob()).0, total_supply);
    }

    #[test]
    #[should_panic]
    fn test_new_twice_fails() {
        let context = get_context(carol());
        testing_env!(context);
        let total_supply = 1_000_000_000_000_000u128;
        {
            let _contract = FungibleToken::new(bob(), total_supply.into());
        }
        FungibleToken::new(bob(), total_supply.into());
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.storage_usage = env::storage_usage();

        context.attached_deposit = 1000 * STORAGE_PRICE_PER_BYTE;
        testing_env!(context.clone());
        let transfer_amount = total_supply / 3;
        contract.transfer(bob(), transfer_amount.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(contract.get_balance(carol()).0, (total_supply - transfer_amount));
        assert_eq!(contract.get_balance(bob()).0, transfer_amount);
    }

    #[test]
    #[should_panic(expected = "Can not set allowance for yourself")]
    fn test_self_allowance_fail() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.set_allowance(carol(), (total_supply / 2).into());
    }

    #[test]
    #[should_panic(
        expected = "The required attached deposit is 33100000000000000000000, but the given attached deposit is is 0"
    )]
    fn test_self_allowance_fail_no_deposit() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.attached_deposit = 0;
        testing_env!(context.clone());
        contract.set_allowance(bob(), (total_supply / 2).into());
    }

    #[test]
    fn test_carol_escrows_to_bob_transfers_to_alice() {
        // Acting as carol
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.storage_usage = env::storage_usage();

        context.is_view = true;
        testing_env!(context.clone());
        assert_eq!(contract.get_total_supply().0, total_supply);

        let allowance = total_supply / 3;
        let transfer_amount = allowance / 3;
        context.is_view = false;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.set_allowance(bob(), allowance.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(contract.get_allowance(carol(), bob()).0, allowance);

        // Acting as bob now
        context.is_view = false;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        context.predecessor_account_id = bob();
        testing_env!(context.clone());
        contract.transfer_from(carol(), alice(), transfer_amount.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(contract.get_balance(carol()).0, total_supply - transfer_amount);
        assert_eq!(contract.get_balance(alice()).0, transfer_amount);
        assert_eq!(contract.get_allowance(carol(), bob()).0, allowance - transfer_amount);
    }

    #[test]
    fn test_carol_escrows_to_bob_locks_and_transfers_to_alice() {
        // Acting as carol
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.storage_usage = env::storage_usage();

        context.is_view = true;
        testing_env!(context.clone());
        assert_eq!(contract.get_total_supply().0, total_supply);

        let allowance = total_supply / 3;
        let transfer_amount = allowance / 3;
        context.is_view = false;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.set_allowance(bob(), allowance.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(contract.get_allowance(carol(), bob()).0, allowance);
        assert_eq!(contract.get_balance(carol()).0, total_supply);

        // Acting as bob now
        context.is_view = false;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        context.predecessor_account_id = bob();
        testing_env!(context.clone());
        contract.transfer_from(carol(), alice(), transfer_amount.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(contract.get_balance(carol()).0, (total_supply - transfer_amount));
        assert_eq!(contract.get_balance(alice()).0, transfer_amount);
        assert_eq!(contract.get_allowance(carol(), bob()).0, allowance - transfer_amount);
    }

    #[test]
    fn test_self_allowance_set_for_refund() {
        let mut context = get_context(carol());
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(carol(), total_supply.into());
        context.storage_usage = env::storage_usage();

        let initial_balance = context.account_balance;
        let initial_storage = context.storage_usage;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.set_allowance(bob(), (total_supply / 2).into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();
        assert_eq!(
            context.account_balance,
            initial_balance
                + Balance::from(context.storage_usage - initial_storage) * STORAGE_PRICE_PER_BYTE
        );

        let initial_balance = context.account_balance;
        let initial_storage = context.storage_usage;
        testing_env!(context.clone());
        context.attached_deposit = 0;
        testing_env!(context.clone());
        contract.set_allowance(bob(), 0.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();
        assert!(context.storage_usage < initial_storage);
        assert!(context.account_balance < initial_balance);
        assert_eq!(
            context.account_balance,
            initial_balance
                - Balance::from(initial_storage - context.storage_usage) * STORAGE_PRICE_PER_BYTE
        );
    }
}
