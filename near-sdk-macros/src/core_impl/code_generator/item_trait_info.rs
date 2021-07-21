use crate::core_impl::info_extractor::ItemTraitInfo;
use quote::quote;
use syn::export::TokenStream2;

impl ItemTraitInfo {
    /// Generate code that wrapps external calls.
    pub fn wrapped_module(&self) -> TokenStream2 {
        let mut result = TokenStream2::new();
        for method in &self.methods {
            result.extend(method.method_wrapper());
        }
        let mod_name = &self.mod_name;
        quote! {
           pub mod #mod_name {
                use super::*;
                use near_sdk::{Gas, Balance, AccountId, Promise};
                use std::string::ToString;
                #result
            }
        }
    }
}

// Rustfmt removes comas.
#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use syn::ItemTrait;
    use quote::quote;
    use crate::core_impl::info_extractor::ItemTraitInfo;

    #[test]
    fn standard() {
        let mut t: ItemTrait = syn::parse2(
            quote!{
                    pub trait ExternalCrossContract {
                        fn merge_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>>;
                        fn merge(
                            &self,
                            #[callback]
                            #[serializer(borsh)]
                            data0: Vec<u8>,
                            #[callback]
                            #[serializer(borsh)]
                            data1: Vec<u8>,
                        ) -> Vec<u8>;
                    }
            }
        ).unwrap();
        let info = ItemTraitInfo::new(&mut t, None).unwrap();
        let actual = info.wrapped_module();

        let expected = quote! {
            pub mod external_cross_contract {
                use super::*;
                use near_sdk::{Gas, Balance, AccountId, Promise};
                use std::string::ToString;
                pub fn merge_sort<T: ToString>(
                    arr: Vec<u8>,
                    __account_id: &T,
                    __balance: near_sdk::Balance,
                    __gas: near_sdk::Gas
                ) -> near_sdk::Promise {
                    #[derive(near_sdk :: serde :: Serialize)]
                    #[serde(crate = "near_sdk::serde")]
                    struct Input {
                        arr: Vec<u8>,
                    }
                    let args = Input { arr, };
                    let args = near_sdk::serde_json::to_vec(&args)
                        .expect("Failed to serialize the cross contract args using JSON.");
                    near_sdk::Promise::new(AccountId::new_unchecked(__account_id.to_string())).function_call(
                        b"merge_sort".to_vec(),
                        args,
                        __balance,
                        __gas,
                    )
                }
                pub fn merge<T: ToString>(__account_id: &T, __balance: near_sdk::Balance, __gas: near_sdk::Gas) -> near_sdk::Promise {
                    let args = vec![];
                    near_sdk::Promise::new(AccountId::new_unchecked(__account_id.to_string())).function_call(
                        b"merge".to_vec(),
                        args,
                        __balance,
                        __gas,
                    )
                }
            }
        };
        assert_eq!(actual.to_string(), expected.to_string());
    }

    #[test]
    fn serialize_with_borsh() {
        let mut t: ItemTrait = syn::parse2(
            quote!{
              trait TestExt {
                #[result_serializer(borsh)]
                fn test(#[serializer(borsh)] v: Vec<String>) -> Vec<String>;
              }
            }
        ).unwrap();
        let info = ItemTraitInfo::new(&mut t, None).unwrap();
        let actual = info.wrapped_module();

        let expected = quote! {
          pub mod test_ext {
            use super::*;
            use near_sdk::{Gas, Balance, AccountId, Promise};
            use std::string::ToString;
            pub fn test<T: ToString>(
                v: Vec<String>,
                __account_id: &T,
                __balance: near_sdk::Balance,
                __gas: near_sdk::Gas
            ) -> near_sdk::Promise {
                #[derive(near_sdk :: borsh :: BorshSerialize)]
                struct Input {
                    v: Vec<String>,
                }
                let args = Input { v, };
                let args = near_sdk::borsh::BorshSerialize::try_to_vec(&args)
                    .expect("Failed to serialize the cross contract args using Borsh.");
                near_sdk::Promise::new(AccountId::new_unchecked(__account_id.to_string())).function_call(
                    b"test".to_vec(),
                    args,
                    __balance,
                    __gas,
                )
            }
        }
        };
        assert_eq!(actual.to_string(), expected.to_string());
    }
}
