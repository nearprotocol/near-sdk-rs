#![recursion_limit = "128"]
extern crate proc_macro;

mod core_impl;

use proc_macro::TokenStream;

use self::core_impl::*;
use proc_macro2::Span;
use quote::quote;
use syn::visit::Visit;
use syn::{File, ItemEnum, ItemImpl, ItemStruct, ItemTrait};

#[proc_macro_attribute]
pub fn near_bindgen(_attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(input) = syn::parse::<ItemStruct>(item.clone()) {
        let struct_proxy = generate_proxy_struct(&input);
        TokenStream::from(quote! {
            #input
            #struct_proxy
        })
    } else if let Ok(mut input) = syn::parse::<ItemImpl>(item) {
        let item_impl_info = match ItemImplInfo::new(&mut input) {
            Ok(x) => x,
            Err(err) => {
                return err.to_compile_error().into();
            }
        };
        let generated_code = item_impl_info.wrapper_code();
        // Add helper type for simulation testing only if not wasm32
        let marshalled_code = item_impl_info.marshall_code();
        TokenStream::from(quote! {
            #marshalled_code
            #input
            #generated_code
        })
    } else {
        TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "near_bindgen can only be used on type declarations and impl sections.",
            )
            .to_compile_error(),
        )
    }
}

#[proc_macro_attribute]
pub fn ext_contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(mut input) = syn::parse::<ItemTrait>(item) {
        let mod_name: Option<proc_macro2::Ident> = if attr.is_empty() {
            None
        } else {
            match syn::parse(attr) {
                Ok(x) => x,
                Err(err) => {
                    return TokenStream::from(
                        syn::Error::new(
                            Span::call_site(),
                            format!("Failed to parse mod name for ext_contract: {}", err),
                        )
                        .to_compile_error(),
                    )
                }
            }
        };
        let item_trait_info = match ItemTraitInfo::new(&mut input, mod_name) {
            Ok(x) => x,
            Err(err) => return TokenStream::from(err.to_compile_error()),
        };
        item_trait_info.wrapped_module().into()
    } else {
        TokenStream::from(
            syn::Error::new(Span::call_site(), "ext_contract can only be used on traits")
                .to_compile_error(),
        )
    }
}

// The below attributes a marker-attributes and therefore they are no-op.

/// `callback` is a marker attribute it does not generate code by itself.
#[proc_macro_attribute]
pub fn callback(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `callback_args_vec` is a marker attribute it does not generate code by itself.
#[proc_macro_attribute]
pub fn callback_vec(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `serializer` is a marker attribute it does not generate code by itself.
#[proc_macro_attribute]
pub fn serializer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `result_serializer` is a marker attribute it does not generate code by itself.
#[proc_macro_attribute]
pub fn result_serializer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `init` is a marker attribute it does not generate code by itself.
#[proc_macro_attribute]
pub fn init(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `metadata` generates the metadata method and should be placed at the very end of the `lib.rs` file.
// TODO: Once Rust allows inner attributes and custom procedural macros for modules we should switch this
// to be `#![metadata]` attribute at the top of the contract file instead. https://github.com/rust-lang/rust/issues/54727
#[proc_macro]
pub fn metadata(item: TokenStream) -> TokenStream {
    if let Ok(input) = syn::parse::<File>(item) {
        let mut visitor = MetadataVisitor::new();
        visitor.visit_file(&input);
        let generated = match visitor.generate_metadata_method() {
            Ok(x) => x,
            Err(err) => return TokenStream::from(err.to_compile_error()),
        };
        TokenStream::from(quote! {
            #input
            #generated
        })
    } else {
        TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "Failed to parse code decorated with `metadata!{}` macro. Only valid Rust is supported.",
            )
            .to_compile_error(),
        )
    }
}

/// `PanicOnDefault` generates implementation for `Default` trait that panics with the following
/// message `The contract is not initialized` when `default()` is called.
/// This is a helpful macro in case the contract is required to be initialized with either `init` or
/// `init(ignore_state)`.
#[proc_macro_derive(PanicOnDefault)]
pub fn derive_no_default(item: TokenStream) -> TokenStream {
    if let Ok(input) = syn::parse::<ItemStruct>(item) {
        let name = &input.ident;
        TokenStream::from(quote! {
            impl Default for #name {
                fn default() -> Self {
                    near_sdk::env::panic(b"The contract is not initialized");
                }
            }
        })
    } else {
        TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "PanicOnDefault can only be used on type declarations sections.",
            )
            .to_compile_error(),
        )
    }
}

/// `BorshStorageKey` generates implementation for `BorshIntoStorageKey` trait.
/// It allows the type to be passed as a unique prefix for persistent collections.
/// The type should also implement or derive `BorshSerialize` trait.
#[proc_macro_derive(BorshStorageKey)]
pub fn borsh_storage_key(item: TokenStream) -> TokenStream {
    let name = if let Ok(input) = syn::parse::<ItemEnum>(item.clone()) {
        input.ident
    } else if let Ok(input) = syn::parse::<ItemStruct>(item) {
        input.ident
    } else {
        return TokenStream::from(
            syn::Error::new(
                Span::call_site(),
                "BorshStorageKey can only be used as a derive on enums or structs.",
            )
            .to_compile_error(),
        );
    };
    TokenStream::from(quote! {
        impl near_sdk::BorshIntoStorageKey for #name {}
    })
}
