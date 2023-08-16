use derive_more::*;
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Error;
use syn::parse2;

pub fn derive_with_variant_update(tokens: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(tokens)?;

    let data_struct = match &ast.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => {
            return Err(Error::new_spanned(
                ast,
                "WithVariantUpdate can only be derived for structs",
            ))
        }
    };

    let field_attributes = data_struct
        .fields
        .iter()
        .filter_map(parse_field_for_attribute)
        .collect_vec();

    let variant_field = field_attributes
        .iter()
        .find(|x| x.is_variant())
        .ok_or_else(|| {
            Error::new_spanned(
                &ast,
                "WithVariantUpdate requires a field to be marked with the #[variant] attribute",
            )
        })?
        .unwrap_variant();

    let ident = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();

    let variant_ty = &variant_field.ty;

    let tokens = quote!(
        impl #impl_generics ::service_util::WithVariantUpdate for #ident #type_generics #where_clause {
            type Variant = #variant_ty;
            type Split = ();

            fn split(self, existing: &HashMap<Self::Id, Self>) -> Self::Split;
        }
    );

    Ok(tokens)
}

#[derive(Clone, Copy, IsVariant, Unwrap)]
enum FieldAttribute<'a> {
    Variant(&'a syn::Field),
}

fn parse_field_for_attribute(field: &syn::Field) -> Option<FieldAttribute<'_>> {
    for attr in &field.attrs {
        if attr.path().is_ident("variant") {
            return Some(FieldAttribute::Variant(field));
        }
    }
    None
}
