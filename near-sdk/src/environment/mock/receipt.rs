use crate::{AccountId, Balance, Gas, PublicKey};

#[derive(Clone, Debug)]
pub struct Receipt {
    pub receipt_indices: Vec<u64>,
    pub receiver_id: String,
    pub actions: Vec<VmAction>,
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum VmAction {
    CreateAccount,
    DeployContract {
        code: Vec<u8>,
    },
    FunctionCall {
        method_name: Vec<u8>,
        /// Most function calls still take JSON as input, so we'll keep it there as a string.
        /// Once we switch to borsh, we'll have to switch to base64 encoding.
        /// Right now, it is only used with standalone runtime when passing in Receipts or expecting
        /// receipts. The workaround for input is to use a VMContext input.
        args: Vec<u8>,
        gas: Gas,
        deposit: Balance,
    },
    Transfer {
        deposit: Balance,
    },
    Stake {
        stake: Balance,
        public_key: PublicKey,
    },
    AddKeyWithFullAccess {
        public_key: PublicKey,
        nonce: u64,
    },
    AddKeyWithFunctionCall {
        public_key: PublicKey,
        nonce: u64,
        allowance: Option<Balance>,
        receiver_id: AccountId,
        method_names: Vec<Vec<u8>>,
    },
    DeleteKey {
        public_key: PublicKey,
    },
    DeleteAccount {
        beneficiary_id: AccountId,
    },
}
