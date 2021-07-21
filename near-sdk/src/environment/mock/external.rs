use super::{Receipt, VmAction};
use crate::{
    types::{AccountId, Balance, Gas},
    PublicKey,
};
use near_vm_logic::{External, HostError, ValuePtr};
use std::{collections::HashMap, convert::TryFrom};

type Result<T> = ::core::result::Result<T, near_vm_logic::VMLogicError>;

#[derive(Default, Clone)]
/// Emulates the trie and the mock handling code for the SDK. This is a modified version of
/// `MockedExternal` from `near_vm_logic`.
pub(crate) struct SdkExternal {
    pub fake_trie: HashMap<Vec<u8>, Vec<u8>>,
    pub receipts: Vec<Receipt>,
    pub validators: HashMap<String, Balance>,
}

pub struct MockedValuePtr {
    value: Vec<u8>,
}

impl ValuePtr for MockedValuePtr {
    fn len(&self) -> u32 {
        self.value.len() as u32
    }

    fn deref(&self) -> Result<Vec<u8>> {
        Ok(self.value.clone())
    }
}

impl SdkExternal {
    pub fn new() -> Self {
        Self::default()
    }
}

impl External for SdkExternal {
    fn storage_set(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.fake_trie.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn storage_get(&self, key: &[u8]) -> Result<Option<Box<dyn ValuePtr>>> {
        Ok(self
            .fake_trie
            .get(key)
            .map(|value| Box::new(MockedValuePtr { value: value.clone() }) as Box<_>))
    }

    fn storage_remove(&mut self, key: &[u8]) -> Result<()> {
        self.fake_trie.remove(key);
        Ok(())
    }

    fn storage_remove_subtree(&mut self, prefix: &[u8]) -> Result<()> {
        self.fake_trie.retain(|key, _| !key.starts_with(prefix));
        Ok(())
    }

    fn storage_has_key(&mut self, key: &[u8]) -> Result<bool> {
        Ok(self.fake_trie.contains_key(key))
    }

    fn create_receipt(&mut self, receipt_indices: Vec<u64>, receiver_id: String) -> Result<u64> {
        if let Some(index) = receipt_indices.iter().find(|&&el| el >= self.receipts.len() as u64) {
            return Err(HostError::InvalidReceiptIndex { receipt_index: *index }.into());
        }
        let res = self.receipts.len() as u64;
        self.receipts.push(Receipt { receipt_indices, receiver_id, actions: vec![] });
        Ok(res)
    }

    fn append_action_create_account(&mut self, receipt_index: u64) -> Result<()> {
        self.receipts
            .get_mut(receipt_index as usize)
            .unwrap()
            .actions
            .push(VmAction::CreateAccount);
        Ok(())
    }

    fn append_action_deploy_contract(&mut self, receipt_index: u64, code: Vec<u8>) -> Result<()> {
        self.receipts
            .get_mut(receipt_index as usize)
            .unwrap()
            .actions
            .push(VmAction::DeployContract { code });
        Ok(())
    }

    fn append_action_function_call(
        &mut self,
        receipt_index: u64,
        method_name: Vec<u8>,
        arguments: Vec<u8>,
        attached_deposit: u128,
        prepaid_gas: u64,
    ) -> Result<()> {
        self.receipts.get_mut(receipt_index as usize).unwrap().actions.push(
            VmAction::FunctionCall {
                method_name,
                args: arguments,
                deposit: attached_deposit,
                gas: Gas(prepaid_gas),
            },
        );
        Ok(())
    }

    fn append_action_transfer(&mut self, receipt_index: u64, amount: u128) -> Result<()> {
        self.receipts
            .get_mut(receipt_index as usize)
            .unwrap()
            .actions
            .push(VmAction::Transfer { deposit: amount });
        Ok(())
    }

    fn append_action_stake(
        &mut self,
        receipt_index: u64,
        stake: u128,
        public_key: Vec<u8>,
    ) -> Result<()> {
        let public_key = PublicKey::try_from(public_key).unwrap();
        self.receipts
            .get_mut(receipt_index as usize)
            .unwrap()
            .actions
            .push(VmAction::Stake { stake, public_key });
        Ok(())
    }

    fn append_action_add_key_with_full_access(
        &mut self,
        receipt_index: u64,
        public_key: Vec<u8>,
        nonce: u64,
    ) -> Result<()> {
        let public_key = PublicKey::try_from(public_key).unwrap();
        self.receipts
            .get_mut(receipt_index as usize)
            .unwrap()
            .actions
            .push(VmAction::AddKeyWithFullAccess { public_key, nonce });
        Ok(())
    }

    fn append_action_add_key_with_function_call(
        &mut self,
        receipt_index: u64,
        public_key: Vec<u8>,
        nonce: u64,
        allowance: Option<u128>,
        receiver_id: String,
        method_names: Vec<Vec<u8>>,
    ) -> Result<()> {
        let public_key = PublicKey::try_from(public_key).unwrap();
        self.receipts.get_mut(receipt_index as usize).unwrap().actions.push(
            VmAction::AddKeyWithFunctionCall {
                public_key,
                nonce,
                allowance,
                receiver_id: AccountId::new_unchecked(receiver_id),
                method_names,
            },
        );
        Ok(())
    }

    fn append_action_delete_key(&mut self, receipt_index: u64, public_key: Vec<u8>) -> Result<()> {
        let public_key = PublicKey::try_from(public_key).unwrap();
        self.receipts
            .get_mut(receipt_index as usize)
            .unwrap()
            .actions
            .push(VmAction::DeleteKey { public_key });
        Ok(())
    }

    fn append_action_delete_account(
        &mut self,
        receipt_index: u64,
        beneficiary_id: String,
    ) -> Result<()> {
        self.receipts.get_mut(receipt_index as usize).unwrap().actions.push(
            VmAction::DeleteAccount { beneficiary_id: AccountId::new_unchecked(beneficiary_id) },
        );
        Ok(())
    }

    fn get_touched_nodes_count(&self) -> u64 {
        0
    }

    fn reset_touched_nodes_counter(&mut self) {}

    fn validator_stake(&self, account_id: &String) -> Result<Option<Balance>> {
        Ok(self.validators.get(account_id).cloned())
    }

    fn validator_total_stake(&self) -> Result<Balance> {
        Ok(self.validators.values().sum())
    }
}
