mod serializer_attr;
pub use serializer_attr::SerializerAttr;

mod arg_info;
pub use arg_info::{ArgInfo, BindgenArgType};

mod attr_sig_info;
pub use attr_sig_info::AttrSigInfo;

mod impl_item_method_info;
pub use impl_item_method_info::ImplItemMethodInfo;

mod trait_item_method_info;
pub use trait_item_method_info::*;

mod item_trait_info;
pub use item_trait_info::ItemTraitInfo;

mod item_impl_info;

mod init_attr;
pub use init_attr::InitAttr;

pub use item_impl_info::ItemImplInfo;

/// Type of serialization we use.
#[derive(PartialEq, Eq)]
pub enum SerializerType {
    JSON,
    Borsh,
}

/// Type of the method.
#[derive(PartialEq, Eq)]
pub enum MethodType {
    Regular,
    View,
    Init,
    InitIgnoreState,
}

/// Whether the input struct is used for serialization or deserialization.
#[derive(PartialEq, Eq)]
pub enum InputStructType {
    Serialization,
    Deserialization,
}
