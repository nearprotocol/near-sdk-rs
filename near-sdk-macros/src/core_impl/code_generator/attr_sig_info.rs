use syn::export::TokenStream2;

use crate::core_impl::info_extractor::{
    ArgInfo, AttrSigInfo, BindgenArgType, InputStructType, SerializerType,
};
use quote::quote;

impl AttrSigInfo {
    /// Create struct representing input arguments.
    /// * input_struct_type represents whether the input structure will be used for serialization
    ///     (e.g. for a promise input) or deserialization (e.g. for a method input).
    /// Each argument is getting converted to a field in a struct. Specifically argument:
    /// `ATTRIBUTES ref mut binding @ SUBPATTERN : TYPE` is getting converted to:
    /// `binding: SUBTYPE,` where `TYPE` is one of the following: `& SUBTYPE`, `&mut SUBTYPE`, `SUBTYPE`,
    /// and `SUBTYPE` is one of the following: `[T; n]`, path like
    /// `std::collections::HashMap<SUBTYPE, SUBTYPE>`, or tuple `(SUBTYPE0, SUBTYPE1, ...)`.
    /// # Example
    /// ```ignore
    /// struct Input {
    ///   arg0: Vec<String>,
    ///   arg1: [u64; 10],
    ///   arg2: (u64, Vec<String>),
    /// }
    /// ```
    pub fn input_struct(&self, input_struct_type: InputStructType) -> TokenStream2 {
        let args: Vec<_> = self.input_args().collect();
        assert!(
            !args.is_empty(),
            "Can only generate input struct for when input args are specified"
        );
        let attribute = match input_struct_type {
            InputStructType::Serialization => match &self.input_serializer {
                SerializerType::JSON => quote! {
                    #[derive(near_sdk::serde::Serialize)]
                    #[serde(crate = "near_sdk::serde")]
                },
                SerializerType::Borsh => {
                    quote! {
                        #[derive(near_sdk::borsh::BorshSerialize)]
                    }
                }
            },
            InputStructType::Deserialization => match &self.input_serializer {
                SerializerType::JSON => quote! {
                    #[derive(near_sdk::serde::Deserialize)]
                    #[serde(crate = "near_sdk::serde")]
                },
                SerializerType::Borsh => {
                    quote! {
                        #[derive(near_sdk::borsh::BorshDeserialize)]
                    }
                }
            },
        };
        let mut fields = TokenStream2::new();
        for arg in args {
            let ArgInfo { ty, ident, .. } = &arg;
            fields.extend(quote! {
                #ident: #ty,
            });
        }
        quote! {
            #attribute
            struct Input {
                #fields
            }
        }
    }

    /// Create pattern that decomposes input struct using correct mutability modifiers.
    /// # Example:
    /// ```ignore
    /// Input {
    ///     arg0,
    ///     mut arg1,
    ///     arg2
    /// }
    /// ```
    pub fn decomposition_pattern(&self) -> TokenStream2 {
        let args: Vec<_> = self.input_args().collect();
        assert!(
            !args.is_empty(),
            "Can only generate decomposition pattern for when input args are specified."
        );
        let mut fields = TokenStream2::new();
        for arg in args {
            let ArgInfo { mutability, ident, .. } = &arg;
            fields.extend(quote! {
            #mutability #ident,
            });
        }
        quote! {
            Input {
                #fields
            }
        }
    }

    /// Create expression that constructs the struct.
    /// # Example:
    /// ```ignore
    /// Input {
    ///     arg0,
    ///     arg1,
    ///     arg2,
    /// }
    /// ```
    pub fn constructor_expr(&self) -> TokenStream2 {
        let args: Vec<_> = self.input_args().collect();
        assert!(
            !args.is_empty(),
            "Can only generate constructor expression for when input args are specified."
        );
        let mut fields = TokenStream2::new();
        for arg in args {
            let ArgInfo { ident, .. } = &arg;
            fields.extend(quote! {
            #ident,
            });
        }
        quote! {
            Input {
                #fields
            }
        }
    }

    /// Create a sequence of arguments that can be used to call the method or the function
    /// of the smart contract.
    ///
    /// # Example:
    /// ```ignore
    /// a, &b, &mut c,
    /// ```
    pub fn arg_list(&self) -> TokenStream2 {
        let mut result = TokenStream2::new();
        for arg in &self.args {
            let ArgInfo { reference, mutability, ident, .. } = &arg;
            result.extend(quote! {
                #reference #mutability #ident,
            });
        }
        result
    }

    /// Create a sequence of patterns and types to be used in the method signature.
    ///
    /// # Example:
    /// ```ignore
    /// a: u64, b: &mut T, ref mut c: Vec<String>,
    /// ```
    pub fn pat_type_list(&self) -> TokenStream2 {
        let mut result = TokenStream2::new();
        for arg in self.input_args() {
            let ArgInfo { original, .. } = &arg;
            result.extend(quote! {
                #original,
            });
        }
        result
    }

    /// Create code that deserializes arguments that were decorated with `#[callback]`
    pub fn callback_deserialization(&self) -> TokenStream2 {
        self
            .args
            .iter()
            .filter(|arg| matches!(arg.bindgen_ty, BindgenArgType::CallbackArg))
            .enumerate()
            .fold(TokenStream2::new(), |acc, (idx, arg)| {
                let idx = idx as u64;
                let ArgInfo { mutability, ident, ty, .. } = arg;
                let read_data = quote! {
                let data: Vec<u8> = match near_sdk::env::promise_result(#idx) {
                    near_sdk::PromiseResult::Successful(x) => x,
                    _ => panic!("Callback computation {} was not successful", #idx)
                };
            };
                let invocation = match arg.serializer_ty {
                    SerializerType::JSON => quote! {
                        near_sdk::serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON")
                    },
                    SerializerType::Borsh => quote! {
                        near_sdk::borsh::BorshDeserialize::try_from_slice(&data).expect("Failed to deserialize callback using Borsh")
                    },
                };
                quote! {
                #acc
                #read_data
                let #mutability #ident: #ty = #invocation;
            }
            })
    }

    /// Create code that deserializes arguments that were decorated with `#[callback_vec]`.
    pub fn callback_vec_deserialization(&self) -> TokenStream2 {
        self
            .args
            .iter()
            .filter(|arg| matches!(arg.bindgen_ty, BindgenArgType::CallbackArgVec))
            .fold(TokenStream2::new(), |acc, arg| {
                let ArgInfo { mutability, ident, ty, .. } = arg;
                let invocation = match arg.serializer_ty {
                    SerializerType::JSON => quote! {
                        near_sdk::serde_json::from_slice(&data).expect("Failed to deserialize callback using JSON")
                    },
                    SerializerType::Borsh => quote! {
                        near_sdk::borsh::BorshDeserialize::try_from_slice(&data).expect("Failed to deserialize callback using Borsh")
                    },
                };
                quote! {
                #acc
                let #mutability #ident: #ty = (0..near_sdk::env::promise_results_count())
                .map(|i| {
                    let data: Vec<u8> = match near_sdk::env::promise_result(i) {
                        near_sdk::PromiseResult::Successful(x) => x,
                        _ => panic!("Callback computation {} was not successful", i)
                    };
                    #invocation
                }).collect();
            }
            })
    }
}
