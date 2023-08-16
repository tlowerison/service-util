use derive_more::*;
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::str::FromStr;
use syn::parse::Error;
use syn::parse2;

pub fn derive_id(tokens: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(tokens)?;

    let data_struct = match &ast.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => return Err(Error::new_spanned(ast, "Id can only be derived for structs")),
    };

    let field_attributes = data_struct
        .fields
        .iter()
        .filter_map(parse_field_for_attribute)
        .collect_vec();

    let id_ident = format_ident!("id");
    let (id_field_index, id_field) = field_attributes
        .iter()
        .enumerate()
        .find(|(_, x)| x.is_id())
        .map(|(i, x)| (i, x.unwrap_id()))
        .or_else(|| {
            data_struct
                .fields
                .iter()
                .enumerate()
                .find(|(_, x)| x.ident.as_ref().map(|x| *x == id_ident).unwrap_or_default())
        })
        .ok_or_else(|| Error::new_spanned(&ast, "Id requires a field to be marked with the #[id] attribute"))?;
    let id_accessor = id_field
        .ident
        .as_ref()
        .map(|x| quote!(#x))
        .unwrap_or_else(|| TokenStream::from_str(&format!("{id_field_index}")).unwrap());

    let ident = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();
    let id_ty = &id_field.ty;

    let tokens = quote!(
        impl #impl_generics ::service_util::Id for #ident #type_generics #where_clause {
            type Id = #id_ty;
            fn id(&self) -> Self::Id {
                self.#id_accessor.clone()
            }
        }
    );

    Ok(tokens)
}

#[derive(Clone, Copy, IsVariant, Unwrap)]
enum FieldAttribute<'a> {
    Id(&'a syn::Field),
}

fn parse_field_for_attribute(field: &syn::Field) -> Option<FieldAttribute<'_>> {
    for attr in &field.attrs {
        if attr.path().is_ident("id") {
            return Some(FieldAttribute::Id(field));
        }
    }
    None
}
