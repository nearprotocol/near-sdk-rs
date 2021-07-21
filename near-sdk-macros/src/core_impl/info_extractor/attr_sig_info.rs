use super::{ArgInfo, BindgenArgType, InitAttr, MethodType, SerializerAttr, SerializerType};
use quote::ToTokens;
use syn::export::Span;
use syn::spanned::Spanned;
use syn::{Attribute, Error, FnArg, Ident, Receiver, ReturnType, Signature};

/// Information extracted from method attributes and signature.
pub struct AttrSigInfo {
    /// The name of the method.
    pub ident: Ident,
    /// Attributes not related to bindgen.
    pub non_bindgen_attrs: Vec<Attribute>,
    /// All arguments of the method.
    pub args: Vec<ArgInfo>,
    /// Describes the type of the method.
    pub method_type: MethodType,
    /// Whether method accepting $NEAR.
    pub is_payable: bool,
    /// Whether method can accept calls from self (current account)
    pub is_private: bool,
    /// The serializer that we use for `env::input()`.
    pub input_serializer: SerializerType,
    /// The serializer that we use for the return type.
    pub result_serializer: SerializerType,
    /// The receiver, like `mut self`, `self`, `&mut self`, `&self`, or `None`.
    pub receiver: Option<Receiver>,
    /// What this function returns.
    pub returns: ReturnType,
    /// The original method signature.
    pub original_sig: Signature,
}

impl AttrSigInfo {
    /// Process the method and extract information important for near-sdk.
    pub fn new(
        original_attrs: &mut Vec<Attribute>,
        original_sig: &mut Signature,
    ) -> syn::Result<Self> {
        if original_sig.asyncness.is_some() {
            return Err(Error::new(
                original_sig.span(),
                "Contract API is not allowed to be async.",
            ));
        }
        if original_sig.abi.is_some() {
            return Err(Error::new(
                original_sig.span(),
                "Contract API is not allowed to have binary interface.",
            ));
        }
        if original_sig.variadic.is_some() {
            return Err(Error::new(
                original_sig.span(),
                "Contract API is not allowed to have variadic arguments.",
            ));
        }

        let ident = original_sig.ident.clone();
        let mut non_bindgen_attrs = vec![];
        let mut args = vec![];
        let mut method_type = MethodType::Regular;
        let mut is_payable = false;
        let mut is_private = false;
        // By the default we serialize the result with JSON.
        let mut result_serializer = SerializerType::JSON;

        let mut payable_attr = None;
        for attr in original_attrs.iter() {
            let attr_str = attr.path.to_token_stream().to_string();
            match attr_str.as_str() {
                "init" => {
                    let init_attr: InitAttr = syn::parse2(attr.tokens.clone())?;
                    if init_attr.ignore_state {
                        method_type = MethodType::InitIgnoreState;
                    } else {
                        method_type = MethodType::Init;
                    }
                }
                "payable" => {
                    payable_attr = Some(attr);
                    is_payable = true;
                }
                "private" => {
                    is_private = true;
                }
                "result_serializer" => {
                    let serializer: SerializerAttr = syn::parse2(attr.tokens.clone())?;
                    result_serializer = serializer.serializer_type;
                }
                _ => {
                    non_bindgen_attrs.push((*attr).clone());
                }
            }
        }

        let mut receiver = None;
        for fn_arg in &mut original_sig.inputs {
            match fn_arg {
                FnArg::Receiver(r) => receiver = Some((*r).clone()),
                FnArg::Typed(pat_typed) => {
                    args.push(ArgInfo::new(pat_typed)?);
                }
            }
        }

        if let Some(ref receiver) = receiver {
            if matches!(method_type, MethodType::Regular) {
                if receiver.mutability.is_none() {
                    method_type = MethodType::View;
                }
            } else {
                return Err(Error::new(
                    payable_attr.span(),
                    "Init methods can't have `self` attribute",
                ));
            }
        };

        if let Some(payable_attr) = payable_attr {
            if matches!(method_type, MethodType::View) {
                return Err(Error::new(
                    payable_attr.span(),
                    "Payable method must be mutable (not view)",
                ));
            }
        }

        *original_attrs = non_bindgen_attrs.clone();
        let returns = original_sig.output.clone();

        let mut result = Self {
            ident,
            non_bindgen_attrs,
            args,
            input_serializer: SerializerType::JSON,
            method_type,
            is_payable,
            is_private,
            result_serializer,
            receiver,
            returns,
            original_sig: original_sig.clone(),
        };

        let input_serializer =
            if result.input_args().all(|arg: &ArgInfo| arg.serializer_ty == SerializerType::JSON) {
                SerializerType::JSON
            } else if result.input_args().all(|arg| arg.serializer_ty == SerializerType::Borsh) {
                SerializerType::Borsh
            } else {
                return Err(Error::new(
                    Span::call_site(),
                    "Input arguments should be all of the same serialization type.",
                ));
            };
        result.input_serializer = input_serializer;
        Ok(result)
    }

    /// Only get args that correspond to `env::input()`.
    pub fn input_args(&self) -> impl Iterator<Item = &ArgInfo> {
        self.args.iter().filter(|arg| matches!(arg.bindgen_ty, BindgenArgType::Regular))
    }
}
