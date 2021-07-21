use super::TraitItemMethodInfo;
use inflector::Inflector;
use syn::export::Span;
use syn::spanned::Spanned;
use syn::{Error, Ident, ItemTrait, TraitItem};

/// Information extracted from `ItemTrait`.
pub struct ItemTraitInfo {
    /// The name of the module that will be used to generate the module.
    pub mod_name: Ident,
    /// Information extracted from the methods.
    pub methods: Vec<TraitItemMethodInfo>,
    /// The original AST.
    pub original: ItemTrait,
}

impl ItemTraitInfo {
    pub fn new(original: &mut ItemTrait, mod_name_override: Option<Ident>) -> syn::Result<Self> {
        let mod_name = mod_name_override.unwrap_or({
            let res = original.ident.to_string().to_snake_case();
            Ident::new(&res, Span::call_site())
        });

        let mut methods = vec![];
        for item in &mut original.items {
            match item {
                TraitItem::Type(_) => {
                    return Err(Error::new(
                        item.span(),
                        "Traits for external contracts do not support associated trait types yet.",
                    ))
                }
                TraitItem::Method(method) => {
                    methods.push(TraitItemMethodInfo::new(method)?);
                    if method.default.is_some() {
                        return Err(Error::new(
                            method.span(),
                            "Traits that are used to describe external contract should not include
                             default implementations because this is not a valid use case of traits
                             to describe external contracts.",
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(Self { original: original.clone(), mod_name, methods })
    }
}
