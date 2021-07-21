/// The core methods for a basic non-fungible token. Extension standards may be
/// added in addition to this macro.
#[macro_export]
macro_rules! impl_non_fungible_token_core {
    ($contract: ident, $token: ident) => {
        use std::collections::HashMap;
        use $crate::non_fungible_token::core::NonFungibleTokenCore;
        use $crate::non_fungible_token::core::NonFungibleTokenResolver;

        #[near_bindgen]
        impl NonFungibleTokenCore for $contract {
            #[payable]
            fn nft_transfer(
                &mut self,
                receiver_id: AccountId,
                token_id: TokenId,
                approval_id: Option<u64>,
                memo: Option<String>,
            ) {
                self.$token.nft_transfer(receiver_id, token_id, approval_id, memo)
            }

            #[payable]
            fn nft_transfer_call(
                &mut self,
                receiver_id: AccountId,
                token_id: TokenId,
                approval_id: Option<u64>,
                memo: Option<String>,
                msg: String,
            ) -> PromiseOrValue<bool> {
                self.$token.nft_transfer_call(receiver_id, token_id, approval_id, memo, msg)
            }

            fn nft_token(&self, token_id: TokenId) -> Option<Token> {
                self.$token.nft_token(token_id)
            }

            fn mint(
                &mut self,
                token_id: TokenId,
                token_owner_id: AccountId,
                token_metadata: Option<TokenMetadata>,
            ) -> Token {
                self.$token.mint(token_id, token_owner_id, token_metadata)
            }
        }

        #[near_bindgen]
        impl NonFungibleTokenResolver for $contract {
            #[private]
            fn nft_resolve_transfer(
                &mut self,
                previous_owner_id: AccountId,
                receiver_id: AccountId,
                token_id: TokenId,
                approved_account_ids: Option<HashMap<AccountId, u64>>,
            ) -> bool {
                self.$token.nft_resolve_transfer(
                    previous_owner_id,
                    receiver_id,
                    token_id,
                    approved_account_ids,
                )
            }
        }
    };
}

/// Non-fungible token approval management allows for an escrow system where
/// multiple approvals per token exist.
#[macro_export]
macro_rules! impl_non_fungible_token_approval {
    ($contract: ident, $token: ident) => {
        use $crate::non_fungible_token::approval::NonFungibleTokenApproval;

        #[near_bindgen]
        impl NonFungibleTokenApproval for $contract {
            #[payable]
            fn nft_approve(
                &mut self,
                token_id: TokenId,
                account_id: AccountId,
                msg: Option<String>,
            ) -> Option<Promise> {
                self.$token.nft_approve(token_id, account_id, msg)
            }

            #[payable]
            fn nft_revoke(&mut self, token_id: TokenId, account_id: AccountId) {
                self.$token.nft_revoke(token_id, account_id)
            }

            #[payable]
            fn nft_revoke_all(&mut self, token_id: TokenId) {
                self.$token.nft_revoke_all(token_id)
            }

            fn nft_is_approved(
                self,
                token_id: TokenId,
                approved_account_id: AccountId,
                approval_id: Option<u64>,
            ) -> bool {
                self.$token.nft_is_approved(token_id, approved_account_id, approval_id)
            }
        }
    };
}

/// Non-fungible enumeration adds the extension standard offering several
/// view-only methods to get token supply, tokens per owner, etc.
#[macro_export]
macro_rules! impl_non_fungible_token_enumeration {
    ($contract: ident, $token: ident) => {
        use near_sdk::json_types::U128;
        use $crate::non_fungible_token::enumeration::NonFungibleTokenEnumeration;

        #[near_bindgen]
        impl NonFungibleTokenEnumeration for $contract {
            fn nft_total_supply(&self) -> U128 {
                self.$token.nft_total_supply()
            }

            fn nft_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<Token> {
                self.$token.nft_tokens(from_index, limit)
            }

            fn nft_supply_for_owner(&self, account_id: AccountId) -> U128 {
                self.$token.nft_supply_for_owner(account_id)
            }

            fn nft_tokens_for_owner(
                &self,
                account_id: AccountId,
                from_index: Option<U128>,
                limit: Option<u64>,
            ) -> Vec<Token> {
                self.$token.nft_tokens_for_owner(account_id, from_index, limit)
            }
        }
    };
}
