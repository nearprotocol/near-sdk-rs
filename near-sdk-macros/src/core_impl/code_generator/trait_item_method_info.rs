use crate::core_impl::{
    info_extractor::{InputStructType, SerializerType, TraitItemMethodInfo},
    AttrSigInfo,
};
use quote::quote;
use syn::export::TokenStream2;

impl TraitItemMethodInfo {
    /// Generate code that wraps the method.
    pub fn method_wrapper(&self) -> TokenStream2 {
        let ident = &self.attr_sig_info.ident;
        let ident_byte_str = &self.ident_byte_str;
        let pat_type_list = self.attr_sig_info.pat_type_list();
        let serialize = TraitItemMethodInfo::generate_serialier(
            &self.attr_sig_info,
            &self.attr_sig_info.result_serializer,
        );
        quote! {
            pub fn #ident<T: ToString>(#pat_type_list __account_id: &T, __balance: near_sdk::Balance, __gas: near_sdk::Gas) -> near_sdk::Promise {
                #serialize
                near_sdk::Promise::new(AccountId::new_unchecked(__account_id.to_string()))
                .function_call(
                    #ident_byte_str.to_vec(),
                    args,
                    __balance,
                    __gas,
                )
            }
        }
    }

    pub fn generate_serialier(
        attr_sig_info: &AttrSigInfo,
        serializer: &SerializerType,
    ) -> TokenStream2 {
        let has_input_args = attr_sig_info.input_args().next().is_some();
        if !has_input_args {
            return quote! { let args = vec![]; };
        }
        let struct_decl = attr_sig_info.input_struct(InputStructType::Serialization);
        let constructor_call = attr_sig_info.constructor_expr();
        let constructor = quote! { let args = #constructor_call; };
        let value_ser = match serializer {
            SerializerType::JSON => quote! {
                let args = near_sdk::serde_json::to_vec(&args).expect("Failed to serialize the cross contract args using JSON.");
            },
            SerializerType::Borsh => quote! {
                let args = near_sdk::borsh::BorshSerialize::try_to_vec(&args).expect("Failed to serialize the cross contract args using Borsh.");
            },
        };

        quote! {
          #struct_decl
          #constructor
          #value_ser
        }
    }
}
