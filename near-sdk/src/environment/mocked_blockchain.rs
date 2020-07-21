use crate::environment::blockchain_interface::BlockchainInterface;
use near_runtime_fees::RuntimeFeesConfig;
use near_vm_logic::mocks::mock_external::{MockedExternal, Receipt};
use near_vm_logic::mocks::mock_memory::MockedMemory;
use near_vm_logic::types::{PromiseResult, AccountId, Balance};
use near_vm_logic::{External, MemoryLike, VMConfig, VMContext, VMLogic};
use std::cell::RefCell;
use std::collections::HashMap;

/// Mocked blockchain that can be used in the tests for the smart contracts.
/// It implements `BlockchainInterface` by redirecting calls to `VMLogic`. It unwraps errors of
/// `VMLogic` to cause panic during the unit test similarly to how errors of `VMLogic` would cause
/// the termination of guest program execution. Unit tests can even assert the expected error
/// message.
pub struct MockedBlockchain {
    logic: RefCell<VMLogic<'static>>,
    // We keep ownership over logic fixture so that references in `VMLogic` are valid.
    #[allow(dead_code)]
    logic_fixture: LogicFixture,
}

struct LogicFixture {
    ext: Box<MockedExternal>,
    memory: Box<dyn MemoryLike>,
    #[allow(clippy::box_vec)]
    promise_results: Box<Vec<PromiseResult>>,
    config: Box<VMConfig>,
    fees_config: Box<RuntimeFeesConfig>,
}

impl MockedBlockchain {
    pub fn new(
        context: VMContext,
        config: VMConfig,
        fees_config: RuntimeFeesConfig,
        promise_results: Vec<PromiseResult>,
        storage: HashMap<Vec<u8>, Vec<u8>>,
        validators: HashMap<AccountId, Balance>,
    ) -> Self {
        let mut ext = Box::new(MockedExternal::new());
        ext.fake_trie = storage;
        ext.validators = validators;
        let memory = Box::new(MockedMemory {});
        let promise_results = Box::new(promise_results);
        let config = Box::new(config);
        let fees_config = Box::new(fees_config);

        let mut logic_fixture = LogicFixture { ext, memory, config, fees_config, promise_results };

        let logic = unsafe {
            VMLogic::new(
                &mut *(logic_fixture.ext.as_mut() as *mut dyn External),
                context,
                &*(logic_fixture.config.as_mut() as *const VMConfig),
                &*(logic_fixture.fees_config.as_mut() as *const RuntimeFeesConfig),
                &*(logic_fixture.promise_results.as_ref().as_slice() as *const [PromiseResult]),
                &mut *(logic_fixture.memory.as_mut() as *mut dyn MemoryLike),
            )
        };

        let logic = RefCell::new(logic);
        Self { logic, logic_fixture }
    }

    pub fn take_storage(&mut self) -> HashMap<Vec<u8>, Vec<u8>> {
        std::mem::take(&mut self.logic_fixture.ext.fake_trie)
    }

    pub fn created_receipts(&self) -> &Vec<Receipt> {
        self.logic_fixture.ext.get_receipt_create_calls()
    }
}

impl BlockchainInterface for MockedBlockchain {
    unsafe fn read_register(&self, register_id: u64, ptr: u64) {
        self.logic.borrow_mut().read_register(register_id, ptr).unwrap()
    }

    unsafe fn register_len(&self, register_id: u64) -> u64 {
        self.logic.borrow_mut().register_len(register_id).unwrap()
    }

    unsafe fn current_account_id(&self, register_id: u64) {
        self.logic.borrow_mut().current_account_id(register_id).unwrap()
    }

    unsafe fn signer_account_id(&self, register_id: u64) {
        self.logic.borrow_mut().signer_account_id(register_id).unwrap()
    }

    unsafe fn signer_account_pk(&self, register_id: u64) {
        self.logic.borrow_mut().signer_account_pk(register_id).unwrap()
    }

    unsafe fn predecessor_account_id(&self, register_id: u64) {
        self.logic.borrow_mut().predecessor_account_id(register_id).unwrap()
    }

    unsafe fn input(&self, register_id: u64) {
        self.logic.borrow_mut().input(register_id).unwrap()
    }

    unsafe fn block_index(&self) -> u64 {
        self.logic.borrow_mut().block_index().unwrap()
    }

    unsafe fn block_timestamp(&self) -> u64 {
        self.logic.borrow_mut().block_timestamp().unwrap()
    }

    unsafe fn epoch_height(&self) -> u64 {
        self.logic.borrow_mut().epoch_height().unwrap()
    }

    unsafe fn storage_usage(&self) -> u64 {
        self.logic.borrow_mut().storage_usage().unwrap()
    }

    unsafe fn account_balance(&self, balance_ptr: u64) {
        self.logic.borrow_mut().account_balance(balance_ptr).unwrap()
    }

    unsafe fn account_locked_balance(&self, balance_ptr: u64) {
        self.logic.borrow_mut().account_locked_balance(balance_ptr).unwrap()
    }

    unsafe fn attached_deposit(&self, balance_ptr: u64) {
        self.logic.borrow_mut().attached_deposit(balance_ptr).unwrap()
    }

    unsafe fn prepaid_gas(&self) -> u64 {
        self.logic.borrow_mut().prepaid_gas().unwrap()
    }

    unsafe fn used_gas(&self) -> u64 {
        self.logic.borrow_mut().used_gas().unwrap()
    }

    unsafe fn random_seed(&self, register_id: u64) {
        self.logic.borrow_mut().random_seed(register_id).unwrap()
    }

    unsafe fn sha256(&self, value_len: u64, value_ptr: u64, register_id: u64) {
        self.logic.borrow_mut().sha256(value_len, value_ptr, register_id).unwrap()
    }

    unsafe fn keccak256(&self, value_len: u64, value_ptr: u64, register_id: u64) {
        self.logic.borrow_mut().keccak256(value_len, value_ptr, register_id).unwrap()
    }

    unsafe fn keccak512(&self, value_len: u64, value_ptr: u64, register_id: u64) {
        self.logic.borrow_mut().keccak512(value_len, value_ptr, register_id).unwrap()
    }

    unsafe fn alt_bn128_g1_multiexp(&self, value_len: u64, value_ptr: u64, register_id: u64) {
        self.logic.borrow_mut().alt_bn128_g1_multiexp(value_len, value_ptr, register_id).unwrap()
    }

    unsafe fn alt_bn128_g1_sum(&self, value_len: u64, value_ptr: u64, register_id: u64) {
        self.logic.borrow_mut().alt_bn128_g1_sum(value_len, value_ptr, register_id).unwrap()
    }

    unsafe fn alt_bn128_pairing_check(&self, value_len: u64, value_ptr: u64) -> u64 {
        self.logic.borrow_mut().alt_bn128_pairing_check(value_len, value_ptr).unwrap()
    }

    unsafe fn value_return(&self, value_len: u64, value_ptr: u64) {
        self.logic.borrow_mut().value_return(value_len, value_ptr).unwrap()
    }

    unsafe fn panic(&self) {
        self.logic.borrow_mut().panic().unwrap()
    }

    unsafe fn panic_utf8(&self, len: u64, ptr: u64) {
        self.logic.borrow_mut().panic_utf8(len, ptr).unwrap()
    }

    unsafe fn log_utf8(&self, len: u64, ptr: u64) {
        self.logic.borrow_mut().log_utf8(len, ptr).unwrap()
    }

    unsafe fn log_utf16(&self, len: u64, ptr: u64) {
        self.logic.borrow_mut().log_utf16(len, ptr).unwrap()
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
        self.logic
            .borrow_mut()
            .promise_create(
                account_id_len,
                account_id_ptr,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
            .unwrap()
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
        self.logic
            .borrow_mut()
            .promise_then(
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
            .unwrap()
    }

    unsafe fn promise_and(&self, promise_idx_ptr: u64, promise_idx_count: u64) -> u64 {
        self.logic.borrow_mut().promise_and(promise_idx_ptr, promise_idx_count).unwrap()
    }

    unsafe fn promise_batch_create(&self, account_id_len: u64, account_id_ptr: u64) -> u64 {
        self.logic.borrow_mut().promise_batch_create(account_id_len, account_id_ptr).unwrap()
    }
    unsafe fn promise_batch_then(
        &self,
        promise_index: u64,
        account_id_len: u64,
        account_id_ptr: u64,
    ) -> u64 {
        self.logic
            .borrow_mut()
            .promise_batch_then(promise_index, account_id_len, account_id_ptr)
            .unwrap()
    }
    unsafe fn promise_batch_action_create_account(&self, promise_index: u64) {
        self.logic.borrow_mut().promise_batch_action_create_account(promise_index).unwrap()
    }
    unsafe fn promise_batch_action_deploy_contract(
        &self,
        promise_index: u64,
        code_len: u64,
        code_ptr: u64,
    ) {
        self.logic
            .borrow_mut()
            .promise_batch_action_deploy_contract(promise_index, code_len, code_ptr)
            .unwrap()
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
        self.logic
            .borrow_mut()
            .promise_batch_action_function_call(
                promise_index,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
            .unwrap()
    }
    unsafe fn promise_batch_action_transfer(&self, promise_index: u64, amount_ptr: u64) {
        self.logic.borrow_mut().promise_batch_action_transfer(promise_index, amount_ptr).unwrap()
    }
    unsafe fn promise_batch_action_stake(
        &self,
        promise_index: u64,
        amount_ptr: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) {
        self.logic
            .borrow_mut()
            .promise_batch_action_stake(promise_index, amount_ptr, public_key_len, public_key_ptr)
            .unwrap()
    }
    unsafe fn promise_batch_action_add_key_with_full_access(
        &self,
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
        nonce: u64,
    ) {
        self.logic
            .borrow_mut()
            .promise_batch_action_add_key_with_full_access(
                promise_index,
                public_key_len,
                public_key_ptr,
                nonce,
            )
            .unwrap()
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
        self.logic
            .borrow_mut()
            .promise_batch_action_add_key_with_function_call(
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
            .unwrap()
    }
    unsafe fn promise_batch_action_delete_key(
        &self,
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) {
        self.logic
            .borrow_mut()
            .promise_batch_action_delete_key(promise_index, public_key_len, public_key_ptr)
            .unwrap()
    }
    unsafe fn promise_batch_action_delete_account(
        &self,
        promise_index: u64,
        beneficiary_id_len: u64,
        beneficiary_id_ptr: u64,
    ) {
        self.logic
            .borrow_mut()
            .promise_batch_action_delete_account(
                promise_index,
                beneficiary_id_len,
                beneficiary_id_ptr,
            )
            .unwrap()
    }
    unsafe fn promise_results_count(&self) -> u64 {
        self.logic.borrow_mut().promise_results_count().unwrap()
    }

    unsafe fn promise_result(&self, result_idx: u64, register_id: u64) -> u64 {
        self.logic.borrow_mut().promise_result(result_idx, register_id).unwrap()
    }

    unsafe fn promise_return(&self, promise_id: u64) {
        self.logic.borrow_mut().promise_return(promise_id).unwrap()
    }

    unsafe fn storage_write(
        &self,
        key_len: u64,
        key_ptr: u64,
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        self.logic
            .borrow_mut()
            .storage_write(key_len, key_ptr, value_len, value_ptr, register_id)
            .unwrap()
    }

    unsafe fn storage_read(&self, key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        self.logic.borrow_mut().storage_read(key_len, key_ptr, register_id).unwrap()
    }

    unsafe fn storage_remove(&self, key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        self.logic.borrow_mut().storage_remove(key_len, key_ptr, register_id).unwrap()
    }

    unsafe fn storage_has_key(&self, key_len: u64, key_ptr: u64) -> u64 {
        self.logic.borrow_mut().storage_has_key(key_len, key_ptr).unwrap()
    }

    unsafe fn validator_stake(&self, account_id_len: u64, account_id_ptr: u64, stake_ptr: u64) {
        self.logic.borrow_mut().validator_stake(account_id_len, account_id_ptr, stake_ptr).unwrap();
    }

    unsafe fn validator_total_stake(&self, stake_ptr: u64) {
        self.logic.borrow_mut().validator_total_stake(stake_ptr).unwrap();
    }

    fn as_mut_mocked_blockchain(&mut self) -> Option<&mut MockedBlockchain> {
        Some(self)
    }

    fn as_mocked_blockchain(&self) -> Option<&MockedBlockchain> {
        Some(self)
    }
}
