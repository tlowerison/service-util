#![feature(proc_macro_span)]

use proc_macro::TokenStream;

#[proc_macro_derive(Id, attributes(id))]
pub fn derive_id(tokens: TokenStream) -> TokenStream {
    match core::derive_id(tokens.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(Split, attributes(split))]
pub fn derive_split(tokens: TokenStream) -> TokenStream {
    match core::derive_split(tokens.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(WithVariantUpdate, attributes(with_variant_update))]
pub fn derive_with_variant_update(tokens: TokenStream) -> TokenStream {
    match core::derive_with_variant_update(tokens.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
