use super::SerializerType;
use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::{parenthesized, Error};

pub struct SerializerAttr {
    #[allow(dead_code)]
    paren_token: syn::token::Paren,
    pub serializer_type: SerializerType,
}

impl Parse for SerializerAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let paren_token = parenthesized!(content in input);
        let ident: Ident = content.parse()?;
        let serializer_type = match ident.to_string().as_str() {
            "borsh" => SerializerType::Borsh,
            "json" => SerializerType::JSON,
            _ => return Err(Error::new(input.span(), "Unsupported serializer type.")),
        };
        Ok(Self { paren_token, serializer_type })
    }
}
