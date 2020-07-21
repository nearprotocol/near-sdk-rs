#[cfg(target_arch = "wasm32")]
pub mod near_blockchain {
    use super::sys;
    use near_sdk::BlockchainInterface;
    /// Implementation of the blockchain interface that contracts actually use during the execution
    /// of the contract.
    pub struct NearBlockchain {}

    impl BlockchainInterface for NearBlockchain {
        unsafe fn read_register(&self, register_id: u64, ptr: u64) {
            sys::read_register(register_id, ptr)
        }

        unsafe fn register_len(&self, register_id: u64) -> u64 {
            sys::register_len(register_id)
        }

        unsafe fn current_account_id(&self, register_id: u64) {
            sys::current_account_id(register_id)
        }

        unsafe fn signer_account_id(&self, register_id: u64) {
            sys::signer_account_id(register_id)
        }

        unsafe fn signer_account_pk(&self, register_id: u64) {
            sys::signer_account_pk(register_id)
        }

        unsafe fn predecessor_account_id(&self, register_id: u64) {
            sys::predecessor_account_id(register_id)
        }

        unsafe fn input(&self, register_id: u64) {
            sys::input(register_id)
        }

        unsafe fn block_index(&self) -> u64 {
            sys::block_index()
        }

        unsafe fn block_timestamp(&self) -> u64 {
            sys::block_timestamp()
        }

        unsafe  fn epoch_height(&self) -> u64 { sys::epoch_height() }

        unsafe fn storage_usage(&self) -> u64 {
            sys::storage_usage()
        }

        unsafe fn account_balance(&self, balance_ptr: u64) {
            sys::account_balance(balance_ptr)
        }

        unsafe fn account_locked_balance(&self, balance_ptr: u64) {
            sys::account_locked_balance(balance_ptr)
        }

        unsafe fn attached_deposit(&self, balance_ptr: u64) {
            sys::attached_deposit(balance_ptr)
        }

        unsafe fn prepaid_gas(&self) -> u64 {
            sys::prepaid_gas()
        }

        unsafe fn used_gas(&self) -> u64 {
            sys::used_gas()
        }

        unsafe fn random_seed(&self, register_id: u64) {
            sys::random_seed(register_id)
        }

        unsafe fn sha256(&self, value_len: u64, value_ptr: u64, register_id: u64) {
            sys::sha256(value_len, value_ptr, register_id)
        }

        unsafe fn keccak256(&self, value_len: u64, value_ptr: u64, register_id: u64) {
            sys::keccak256(value_len, value_ptr, register_id)
        }

        unsafe fn keccak512(&self, value_len: u64, value_ptr: u64, register_id: u64) {
            sys::keccak512(value_len, value_ptr, register_id)
        }

        unsafe fn value_return(&self, value_len: u64, value_ptr: u64) {
            sys::value_return(value_len, value_ptr)
        }

        unsafe fn panic(&self) {
            sys::panic()
        }

        unsafe fn panic_utf8(&self, len: u64, ptr: u64) {
            sys::panic_utf8(len, ptr)
        }

        unsafe fn log_utf8(&self, len: u64, ptr: u64) {
            sys::log_utf8(len, ptr)
        }

        unsafe fn log_utf16(&self, len: u64, ptr: u64) {
            sys::log_utf16(len, ptr)
        }

        unsafe fn promise_create(
            &self,
            account_id_len: u64,
            account_id_ptr: u64,
            method_name_len: u64,
            method_name_ptr: u64,
            arguments_len: u64,
            arguments_ptr: u64,
            amount_ptr: u64,
            gas: u64,
        ) -> u64 {
            sys::promise_create(
                account_id_len,
                account_id_ptr,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
        }

        unsafe fn promise_then(
            &self,
            promise_index: u64,
            account_id_len: u64,
            account_id_ptr: u64,
            method_name_len: u64,
            method_name_ptr: u64,
            arguments_len: u64,
            arguments_ptr: u64,
            amount_ptr: u64,
            gas: u64,
        ) -> u64 {
            sys::promise_then(
                promise_index,
                account_id_len,
                account_id_ptr,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
        }

        unsafe fn promise_and(&self, promise_idx_ptr: u64, promise_idx_count: u64) -> u64 {
            sys::promise_and(promise_idx_ptr, promise_idx_count)
        }

        unsafe fn promise_batch_create(&self, account_id_len: u64, account_id_ptr: u64) -> u64 {
            sys::promise_batch_create(account_id_len, account_id_ptr)
        }

        unsafe fn promise_batch_then(
            &self,
            promise_index: u64,
            account_id_len: u64,
            account_id_ptr: u64,
        ) -> u64 {
            sys::promise_batch_then(promise_index, account_id_len, account_id_ptr)
        }

        unsafe fn promise_batch_action_create_account(&self, promise_index: u64) {
            sys::promise_batch_action_create_account(promise_index)
        }

        unsafe fn promise_batch_action_deploy_contract(
            &self,
            promise_index: u64,
            code_len: u64,
            code_ptr: u64,
        ) {
            sys::promise_batch_action_deploy_contract(promise_index, code_len, code_ptr)
        }

        unsafe fn promise_batch_action_function_call(
            &self,
            promise_index: u64,
            method_name_len: u64,
            method_name_ptr: u64,
            arguments_len: u64,
            arguments_ptr: u64,
            amount_ptr: u64,
            gas: u64,
        ) {
            sys::promise_batch_action_function_call(
                promise_index,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
        }

        unsafe fn promise_batch_action_transfer(&self, promise_index: u64, amount_ptr: u64) {
            sys::promise_batch_action_transfer(promise_index, amount_ptr)
        }

        unsafe fn promise_batch_action_stake(
            &self,
            promise_index: u64,
            amount_ptr: u64,
            public_key_len: u64,
            public_key_ptr: u64,
        ) {
            sys::promise_batch_action_stake(
                promise_index,
                amount_ptr,
                public_key_len,
                public_key_ptr,
            )
        }

        unsafe fn promise_batch_action_add_key_with_full_access(
            &self,
            promise_index: u64,
            public_key_len: u64,
            public_key_ptr: u64,
            nonce: u64,
        ) {
            sys::promise_batch_action_add_key_with_full_access(
                promise_index,
                public_key_len,
                public_key_ptr,
                nonce,
            )
        }

        unsafe fn promise_batch_action_add_key_with_function_call(
            &self,
            promise_index: u64,
            public_key_len: u64,
            public_key_ptr: u64,
            nonce: u64,
            allowance_ptr: u64,
            receiver_id_len: u64,
            receiver_id_ptr: u64,
            method_names_len: u64,
            method_names_ptr: u64,
        ) {
            sys::promise_batch_action_add_key_with_function_call(
                promise_index,
                public_key_len,
                public_key_ptr,
                nonce,
                allowance_ptr,
                receiver_id_len,
                receiver_id_ptr,
                method_names_len,
                method_names_ptr,
            )
        }

        unsafe fn promise_batch_action_delete_key(
            &self,
            promise_index: u64,
            public_key_len: u64,
            public_key_ptr: u64,
        ) {
            sys::promise_batch_action_delete_key(promise_index, public_key_len, public_key_ptr)
        }

        unsafe fn promise_batch_action_delete_account(
            &self,
            promise_index: u64,
            beneficiary_id_len: u64,
            beneficiary_id_ptr: u64,
        ) {
            sys::promise_batch_action_delete_account(
                promise_index,
                beneficiary_id_len,
                beneficiary_id_ptr,
            )
        }

        unsafe fn promise_results_count(&self) -> u64 {
            sys::promise_results_count()
        }

        unsafe fn promise_result(&self, result_idx: u64, register_id: u64) -> u64 {
            sys::promise_result(result_idx, register_id)
        }

        unsafe fn promise_return(&self, promise_id: u64) {
            sys::promise_return(promise_id)
        }

        unsafe fn storage_write(
            &self,
            key_len: u64,
            key_ptr: u64,
            value_len: u64,
            value_ptr: u64,
            register_id: u64,
        ) -> u64 {
            sys::storage_write(key_len, key_ptr, value_len, value_ptr, register_id)
        }

        unsafe fn storage_read(&self, key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
            sys::storage_read(key_len, key_ptr, register_id)
        }

        unsafe fn storage_remove(&self, key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
            sys::storage_remove(key_len, key_ptr, register_id)
        }

        unsafe fn storage_has_key(&self, key_len: u64, key_ptr: u64) -> u64 {
            sys::storage_has_key(key_len, key_ptr)
        }

        unsafe fn validator_stake(&self, account_id_len: u64, account_id_ptr: u64, stake_ptr: u64) {
            sys::validator_stake(account_id_len, account_id_ptr, stake_ptr)
        }

        unsafe fn validator_total_stake(&self, stake_ptr: u64) {
            sys::validator_total_stake(stake_ptr)
        }
        
        unsafe fn alt_bn128_pairing_check(&self, value_len: u64, value_ptr: u64) -> u64 {
            sys::alt_bn128_pairing_check(value_len, value_ptr)
        }

        unsafe fn alt_bn128_g1_multiexp(&self, value_len: u64, value_ptr: u64, register_id: u64) {
            sys::alt_bn128_g1_multiexp(value_len, value_ptr, register_id)
        }

        unsafe fn alt_bn128_g1_sum(&self, value_len: u64, value_ptr: u64, register_id: u64) {
            sys::alt_bn128_g1_sum(value_len, value_ptr, register_id)
        }
    }
}
