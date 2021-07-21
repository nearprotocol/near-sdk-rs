use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::token::Paren;
use syn::Error;

pub struct InitAttr {
    pub ignore_state: bool,
}

impl Parse for InitAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ignore_state = if input.peek(Paren) {
            let content;
            let _paren_token = syn::parenthesized!(content in input);
            let ident: Ident = content.parse()?;
            match ident.to_string().as_str() {
                "ignore_state" => true,
                _ => return Err(Error::new(input.span(), "Unsupported init attribute.")),
            }
        } else {
            false
        };
        Ok(Self { ignore_state })
    }
}
