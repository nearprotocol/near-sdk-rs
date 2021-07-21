use syn::ItemStruct;

pub fn generate_proxy_struct(input: &ItemStruct) -> proc_macro2::TokenStream {
    use quote::{format_ident, quote};
    let ident = &input.ident;
    let new_name = format_ident!("{}Contract", ident);
    let name = quote! {#new_name};
    quote! {
         #[cfg(not(target_arch = "wasm32"))]
         pub struct #name {
            pub account_id: near_sdk::AccountId,
          }
    }
}
