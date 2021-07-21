use crate::{BindgenArgType, ImplItemMethodInfo, InputStructType, MethodType, SerializerType};

use quote::quote;
use syn::export::TokenStream2;
use syn::ReturnType;

impl ImplItemMethodInfo {
    /// Generates metadata struct for this method.
    ///
    /// # Example:
    /// The following method:
    /// ```ignore
    /// fn f3(&mut self, arg0: FancyStruct, arg1: u64) -> Result<IsOk, Error> { }
    /// ```
    /// will produce this struct:
    /// ```ignore
    /// near_sdk::MethodMetadata {
    ///     name: "f3".to_string(),
    ///     is_view: false,
    ///     is_init: false,
    ///     args: {
    ///         #[derive(borsh::BorshSchema)]
    ///         #[derive(serde :: Deserialize, serde :: Serialize)]
    ///         struct Input {
    ///             arg0: FancyStruct,
    ///             arg1: u64,
    ///         }
    ///         Some(Input::schema_container())
    ///     },
    ///     callbacks: vec![],
    ///     callbacks_vec: None,
    ///     result: Some(Result < IsOk, Error > ::schema_container())
    /// }
    /// ```
    /// If args are serialized with Borsh it will not include `#[derive(borsh::BorshSchema)]`.
    pub fn metadata_struct(&self) -> TokenStream2 {
        let method_name_str = self.attr_signature_info.ident.to_string();
        let is_view = matches!(&self.attr_signature_info.method_type, &MethodType::View);
        let is_init = matches!(
            &self.attr_signature_info.method_type,
            &MethodType::Init | &MethodType::InitIgnoreState
        );
        let args = if self.attr_signature_info.input_args().next().is_some() {
            let input_struct =
                self.attr_signature_info.input_struct(InputStructType::Deserialization);
            // If input args are JSON then we need to additionally specify schema for them.
            let additional_schema = match &self.attr_signature_info.input_serializer {
                SerializerType::Borsh => TokenStream2::new(),
                SerializerType::JSON => quote! {
                    #[derive(borsh::BorshSchema)]
                },
            };
            quote! {
                {
                    #additional_schema
                    #input_struct
                    Some(Input::schema_container())
                }
            }
        } else {
            quote! {
                 None
            }
        };
        let callbacks: Vec<_> = self
            .attr_signature_info
            .args
            .iter()
            .filter(|arg| matches!(arg.bindgen_ty, BindgenArgType::CallbackArg))
            .map(|arg| {
                let ty = &arg.ty;
                quote! {
                    #ty::schema_container()
                }
            })
            .collect();
        let callbacks_vec = match self
            .attr_signature_info
            .args
            .iter()
            .filter(|arg| matches!(arg.bindgen_ty, BindgenArgType::CallbackArgVec))
            .last()
        {
            None => {
                quote! {
                    None
                }
            }
            Some(arg) => {
                let ty = &arg.ty;
                quote! {
                    Some(#ty::schema_container())
                }
            }
        };
        let result = match &self.attr_signature_info.returns {
            ReturnType::Default => {
                quote! {
                    None
                }
            }
            ReturnType::Type(_, ty) => {
                quote! {
                    Some(#ty::schema_container())
                }
            }
        };

        quote! {
             near_sdk::MethodMetadata {
                 name: #method_name_str.to_string(),
                 is_view: #is_view,
                 is_init: #is_init,
                 args: #args,
                 callbacks: vec![#(#callbacks),*],
                 callbacks_vec: #callbacks_vec,
                 result: #result
             }
        }
    }
}
