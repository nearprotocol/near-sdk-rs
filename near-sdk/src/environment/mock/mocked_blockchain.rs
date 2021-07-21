use super::{Receipt, SdkExternal};
use crate::test_utils::VMContextBuilder;
use crate::types::{Balance, PromiseResult};
use crate::RuntimeFeesConfig;
use near_vm_logic::mocks::mock_memory::MockedMemory;
use near_vm_logic::types::PromiseResult as VmPromiseResult;
use near_vm_logic::{External, MemoryLike, VMConfig, VMContext, VMLogic, VMOutcome};
use std::cell::RefCell;
use std::collections::HashMap;

/// Mocked blockchain that can be used in the tests for the smart contracts.
/// It implements `BlockchainInterface` by redirecting calls to `VMLogic`. It unwraps errors of
/// `VMLogic` to cause panic during the unit tests similarly to how errors of `VMLogic` would cause
/// the termination of guest program execution. Unit tests can even assert the expected error
/// message.
pub struct MockedBlockchain {
    logic: RefCell<VMLogic<'static>>,
    // We keep ownership over logic fixture so that references in `VMLogic` are valid.
    #[allow(dead_code)]
    logic_fixture: LogicFixture,
}

impl Default for MockedBlockchain {
    fn default() -> Self {
        MockedBlockchain::new(
            VMContextBuilder::new().build(),
            Default::default(),
            Default::default(),
            vec![],
            Default::default(),
            Default::default(),
            None,
        )
    }
}

struct LogicFixture {
    ext: Box<SdkExternal>,
    memory: Box<dyn MemoryLike>,
    #[allow(clippy::box_vec)]
    promise_results: Box<Vec<VmPromiseResult>>,
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
        validators: HashMap<String, Balance>,
        memory_opt: Option<Box<dyn MemoryLike>>,
    ) -> Self {
        let mut ext = Box::new(SdkExternal::new());
        ext.fake_trie = storage;
        ext.validators = validators;
        let memory = memory_opt.unwrap_or_else(|| Box::new(MockedMemory {}));
        let promise_results = Box::new(promise_results.into_iter().map(From::from).collect());
        let config = Box::new(config);
        let fees_config = Box::new(fees_config);

        let mut logic_fixture = LogicFixture { ext, memory, promise_results, config, fees_config };

        let logic = unsafe {
            VMLogic::new_with_protocol_version(
                &mut *(logic_fixture.ext.as_mut() as *mut dyn External),
                context,
                &*(logic_fixture.config.as_mut() as *const VMConfig),
                &*(logic_fixture.fees_config.as_mut() as *const RuntimeFeesConfig),
                &*(logic_fixture.promise_results.as_ref().as_slice() as *const [VmPromiseResult]),
                &mut *(logic_fixture.memory.as_mut() as *mut dyn MemoryLike),
                Default::default(),
                u32::MAX,
            )
        };

        let logic = RefCell::new(logic);
        Self { logic, logic_fixture }
    }

    pub fn take_storage(&mut self) -> HashMap<Vec<u8>, Vec<u8>> {
        std::mem::take(&mut self.logic_fixture.ext.fake_trie)
    }

    pub fn created_receipts(&self) -> &Vec<Receipt> {
        &self.logic_fixture.ext.receipts
    }
    pub fn outcome(&self) -> VMOutcome {
        self.logic.borrow().clone_outcome()
    }

    pub fn gas(&mut self, gas_amount: u32) {
        self.logic.borrow_mut().gas(gas_amount).unwrap()
    }

    pub fn logs(&self) -> Vec<String> {
        self.logic.borrow().clone_outcome().logs
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod mock_chain {
    use near_vm_logic::{VMLogic, VMLogicError};

    use crate::env::BLOCKCHAIN_INTERFACE;

    fn with_mock_interface<F, R>(f: F) -> R
    where
        F: FnOnce(&mut VMLogic) -> Result<R, VMLogicError>,
    {
        BLOCKCHAIN_INTERFACE.with(|b| f(&mut b.borrow().logic.borrow_mut()).unwrap())
    }

    #[no_mangle]
    extern "C" fn read_register(register_id: u64, ptr: u64) {
        with_mock_interface(|b| b.read_register(register_id, ptr))
    }
    #[no_mangle]
    extern "C" fn register_len(register_id: u64) -> u64 {
        with_mock_interface(|b| b.register_len(register_id))
    }
    #[no_mangle]
    extern "C" fn current_account_id(register_id: u64) {
        with_mock_interface(|b| b.current_account_id(register_id))
    }
    #[no_mangle]
    extern "C" fn signer_account_id(register_id: u64) {
        with_mock_interface(|b| b.signer_account_id(register_id))
    }
    #[no_mangle]
    extern "C" fn signer_account_pk(register_id: u64) {
        with_mock_interface(|b| b.signer_account_pk(register_id))
    }
    #[no_mangle]
    extern "C" fn predecessor_account_id(register_id: u64) {
        with_mock_interface(|b| b.predecessor_account_id(register_id))
    }
    #[no_mangle]
    extern "C" fn input(register_id: u64) {
        with_mock_interface(|b| b.input(register_id))
    }
    #[no_mangle]
    extern "C" fn block_index() -> u64 {
        with_mock_interface(|b| b.block_index())
    }
    #[no_mangle]
    extern "C" fn block_timestamp() -> u64 {
        with_mock_interface(|b| b.block_timestamp())
    }
    #[no_mangle]
    extern "C" fn epoch_height() -> u64 {
        with_mock_interface(|b| b.epoch_height())
    }
    #[no_mangle]
    extern "C" fn storage_usage() -> u64 {
        with_mock_interface(|b| b.storage_usage())
    }
    #[no_mangle]
    extern "C" fn account_balance(balance_ptr: u64) {
        with_mock_interface(|b| b.account_balance(balance_ptr))
    }
    #[no_mangle]
    extern "C" fn account_locked_balance(balance_ptr: u64) {
        with_mock_interface(|b| b.account_locked_balance(balance_ptr))
    }
    #[no_mangle]
    extern "C" fn attached_deposit(balance_ptr: u64) {
        with_mock_interface(|b| b.attached_deposit(balance_ptr))
    }
    #[no_mangle]
    extern "C" fn prepaid_gas() -> u64 {
        with_mock_interface(|b| b.prepaid_gas())
    }
    #[no_mangle]
    extern "C" fn used_gas() -> u64 {
        with_mock_interface(|b| b.used_gas())
    }
    #[no_mangle]
    extern "C" fn random_seed(register_id: u64) {
        with_mock_interface(|b| b.random_seed(register_id))
    }
    #[no_mangle]
    extern "C" fn sha256(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.sha256(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn keccak256(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.keccak256(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn keccak512(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.keccak512(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn value_return(value_len: u64, value_ptr: u64) {
        with_mock_interface(|b| b.value_return(value_len, value_ptr))
    }
    #[no_mangle]
    extern "C" fn panic() {
        with_mock_interface(|b| b.panic())
    }
    #[no_mangle]
    extern "C" fn panic_utf8(len: u64, ptr: u64) {
        with_mock_interface(|b| b.panic_utf8(len, ptr))
    }
    #[no_mangle]
    extern "C" fn log_utf8(len: u64, ptr: u64) {
        with_mock_interface(|b| b.log_utf8(len, ptr))
    }
    #[no_mangle]
    extern "C" fn log_utf16(len: u64, ptr: u64) {
        with_mock_interface(|b| b.log_utf16(len, ptr))
    }
    #[no_mangle]
    extern "C" fn promise_create(
        account_id_len: u64,
        account_id_ptr: u64,
        method_name_len: u64,
        method_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: u64,
    ) -> u64 {
        with_mock_interface(|b| {
            b.promise_create(
                account_id_len,
                account_id_ptr,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
        })
    }
    #[no_mangle]
    extern "C" fn promise_then(
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
        with_mock_interface(|b| {
            b.promise_then(
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
        })
    }
    #[no_mangle]
    extern "C" fn promise_and(promise_idx_ptr: u64, promise_idx_count: u64) -> u64 {
        with_mock_interface(|b| b.promise_and(promise_idx_ptr, promise_idx_count))
    }
    #[no_mangle]
    extern "C" fn promise_batch_create(account_id_len: u64, account_id_ptr: u64) -> u64 {
        with_mock_interface(|b| b.promise_batch_create(account_id_len, account_id_ptr))
    }
    #[no_mangle]
    extern "C" fn promise_batch_then(
        promise_index: u64,
        account_id_len: u64,
        account_id_ptr: u64,
    ) -> u64 {
        with_mock_interface(|b| b.promise_batch_then(promise_index, account_id_len, account_id_ptr))
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_create_account(promise_index: u64) {
        with_mock_interface(|b| b.promise_batch_action_create_account(promise_index))
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_deploy_contract(
        promise_index: u64,
        code_len: u64,
        code_ptr: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_deploy_contract(promise_index, code_len, code_ptr)
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_function_call(
        promise_index: u64,
        method_name_len: u64,
        method_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_function_call(
                promise_index,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_transfer(promise_index: u64, amount_ptr: u64) {
        with_mock_interface(|b| b.promise_batch_action_transfer(promise_index, amount_ptr))
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_stake(
        promise_index: u64,
        amount_ptr: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_stake(promise_index, amount_ptr, public_key_len, public_key_ptr)
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_add_key_with_full_access(
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
        nonce: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_add_key_with_full_access(
                promise_index,
                public_key_len,
                public_key_ptr,
                nonce,
            )
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_add_key_with_function_call(
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
        with_mock_interface(|b| {
            b.promise_batch_action_add_key_with_function_call(
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
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_delete_key(
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_delete_key(promise_index, public_key_len, public_key_ptr)
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_delete_account(
        promise_index: u64,
        beneficiary_id_len: u64,
        beneficiary_id_ptr: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_delete_account(
                promise_index,
                beneficiary_id_len,
                beneficiary_id_ptr,
            )
        })
    }
    #[no_mangle]
    extern "C" fn promise_results_count() -> u64 {
        with_mock_interface(|b| b.promise_results_count())
    }
    #[no_mangle]
    extern "C" fn promise_result(result_idx: u64, register_id: u64) -> u64 {
        with_mock_interface(|b| b.promise_result(result_idx, register_id))
    }
    #[no_mangle]
    extern "C" fn promise_return(promise_id: u64) {
        with_mock_interface(|b| b.promise_return(promise_id))
    }
    #[no_mangle]
    extern "C" fn storage_write(
        key_len: u64,
        key_ptr: u64,
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        with_mock_interface(|b| {
            b.storage_write(key_len, key_ptr, value_len, value_ptr, register_id)
        })
    }
    #[no_mangle]
    extern "C" fn storage_read(key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        with_mock_interface(|b| b.storage_read(key_len, key_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn storage_remove(key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        with_mock_interface(|b| b.storage_remove(key_len, key_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn storage_has_key(key_len: u64, key_ptr: u64) -> u64 {
        with_mock_interface(|b| b.storage_has_key(key_len, key_ptr))
    }
    #[no_mangle]
    extern "C" fn validator_stake(account_id_len: u64, account_id_ptr: u64, stake_ptr: u64) {
        with_mock_interface(|b| b.validator_stake(account_id_len, account_id_ptr, stake_ptr))
    }
    #[no_mangle]
    extern "C" fn validator_total_stake(stake_ptr: u64) {
        with_mock_interface(|b| b.validator_total_stake(stake_ptr))
    }
}
