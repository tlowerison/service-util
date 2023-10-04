use itertools::*;
use proc_macro2::TokenStream;
use quote::*;
use syn::parse::Error;
use syn::parse2;

pub fn derive_split(tokens: TokenStream) -> Result<TokenStream, Error> {
    let ast: syn::DeriveInput = parse2(tokens)?;

    let data_enum = match &ast.data {
        syn::Data::Enum(data_enum) => data_enum,
        _ => return Err(Error::new_spanned(ast, "Split can only be derived for enums")),
    };

    let variant_sizes = data_enum
        .variants
        .iter()
        .map(|variant| match &variant.fields {
            syn::Fields::Named(fields_named) => fields_named.named.len(),
            syn::Fields::Unit => 0,
            syn::Fields::Unnamed(fields_unnamed) => fields_unnamed.unnamed.len(),
        })
        .collect_vec();

    let split_variants = data_enum
        .variants
        .iter()
        .enumerate()
        .map(|(variant_index, variant)| {
            let variant_ident = &variant.ident;

            let prefix_size = variant_sizes[..variant_index].iter().sum();
            let suffix_size = if variant_index + 1 < variant_sizes.len() {
                variant_sizes[variant_index + 1..].iter().sum()
            } else {
                0
            };

            let prefix_nones = (0..prefix_size).map(|_| quote!(None)).collect_vec();
            let suffix_nones = (0..suffix_size).map(|_| quote!(None)).collect_vec();

            let (variant_pattern, variant_fields) = match &variant.fields {
                syn::Fields::Unit => (quote!(), quote!()),
                syn::Fields::Named(fields_named) => {
                    let field_idents = fields_named.named.iter().map(|x| &x.ident).collect_vec();
                    (quote!({ #(#field_idents),* }), quote!(#(Some(#field_idents),)*))
                }
                syn::Fields::Unnamed(fields_unnamed) => {
                    let field_idents = fields_unnamed
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format_ident!("_{i}"))
                        .collect_vec();
                    (quote!(( #(#field_idents),* )), quote!(#(Some(#field_idents),)*))
                }
            };

            quote!(Self::#variant_ident #variant_pattern => (#(#prefix_nones,)* #variant_fields #(#suffix_nones,)* ))
        })
        .collect_vec();

    let component_tys = data_enum
        .variants
        .iter()
        .flat_map(|variant| {
            let fields = match &variant.fields {
                syn::Fields::Named(fields_named) => fields_named.named.iter().collect_vec(),
                syn::Fields::Unit => vec![],
                syn::Fields::Unnamed(fields_unnamed) => fields_unnamed.unnamed.iter().collect_vec(),
            };
            fields.into_iter().map(|field| &field.ty)
        })
        .collect_vec();

    let lt: syn::Lifetime = parse2(quote!('split))?;

    let components_ty = quote!((#(Option<#component_tys>,)*));
    let ref_components_ty = quote!((#(Option<&#lt #component_tys>,)*));
    let mut_components_ty = quote!((#(Option<&#lt mut #component_tys>,)*));

    let ident = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();

    // ref
    let mut ref_generics = ast.generics.clone();
    ref_generics.params.insert(0, parse2(quote!(#lt))?);
    let (ref_impl_generics, _, _) = ref_generics.split_for_impl();

    let tokens = quote!(
        impl #impl_generics ::service_util::Split for #ident #type_generics #where_clause {
            type Components = #components_ty;
            fn split(self) -> Self::Components {
                match self { #(#split_variants,)* }
            }
        }

        impl #ref_impl_generics ::service_util::Split for &#lt #ident #type_generics #where_clause {
            type Components = #ref_components_ty;
            fn split(self) -> Self::Components {
                match self { #(#split_variants,)* }
            }
        }
        impl #ref_impl_generics ::service_util::Split for &#lt mut #ident #type_generics #where_clause {
            type Components = #mut_components_ty;
            fn split(self) -> Self::Components {
                match self { #(#split_variants,)* }
            }
        }

        impl #impl_generics From<#ident #type_generics> for #components_ty #where_clause {
            fn from(value: #ident #type_generics) -> Self {
                ::service_util::Split::split(value)
            }
        }
    );

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_unnamed() {
        let tokens = quote!(
            #[derive(Split)]
            pub enum TestEnum {
                A(i32),
                B(u32, u64),
                C(String),
            }
        );

        let output = derive_split(tokens).unwrap();

        let ty = format_ident!("TestEnum");
        let expected = quote!(
            impl ::service_util::Split for #ty {
                type Components = (Option<i32>, Option<u32>, Option<u64>, Option<String>,);
                fn split(self) -> Self::Components {
                    match self {
                        Self::A(_0) => (Some(_0), None, None, None,),
                        Self::B(_0, _1) => (None, Some(_0), Some(_1), None,),
                        Self::C(_0) => (None, None, None, Some(_0),),
                    }
                }
            }

            impl<'split> ::service_util::Split for &'split #ty {
                type Components = (Option<&'split i32>, Option<&'split u32>, Option<&'split u64>, Option<&'split String>,);
                fn split(self) -> Self::Components {
                    match self {
                        Self::A(_0) => (Some(_0), None, None, None,),
                        Self::B(_0, _1) => (None, Some(_0), Some(_1), None,),
                        Self::C(_0) => (None, None, None, Some(_0),),
                    }
                }
            }

            impl<'split> ::service_util::Split for &'split mut #ty {
                type Components = (Option<&'split mut i32>, Option<&'split mut u32>, Option<&'split mut u64>, Option<&'split mut String>,);
                fn split(self) -> Self::Components {
                    match self {
                        Self::A(_0) => (Some(_0), None, None, None,),
                        Self::B(_0, _1) => (None, Some(_0), Some(_1), None,),
                        Self::C(_0) => (None, None, None, Some(_0),),
                    }
                }
            }

            impl From<#ty> for (Option<i32>, Option<u32>, Option<u64>, Option<String>,) {
                fn from(value: #ty) -> Self {
                    ::service_util::Split::split(value)
                }
            }
        );

        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn test_split_named() {
        let tokens = quote!(
            #[derive(Split)]
            pub enum TestEnum {
                A { a: i32 },
                B { b: u32, B: u64 },
                C { c: String },
            }
        );

        let output = derive_split(tokens).unwrap();

        let ty = format_ident!("TestEnum");
        let expected = quote!(
            impl ::service_util::Split for #ty {
                type Components = (Option<i32>, Option<u32>, Option<u64>, Option<String>,);
                fn split(self) -> Self::Components {
                    match self {
                        Self::A { a } => (Some(a), None, None, None,),
                        Self::B { b, B } => (None, Some(b), Some(B), None,),
                        Self::C { c } => (None, None, None, Some(c),),
                    }
                }
            }

            impl<'split> ::service_util::Split for &'split #ty {
                type Components = (Option<&'split i32>, Option<&'split u32>, Option<&'split u64>, Option<&'split String>,);
                fn split(self) -> Self::Components {
                    match self {
                        Self::A { a } => (Some(a), None, None, None,),
                        Self::B { b, B } => (None, Some(b), Some(B), None,),
                        Self::C { c } => (None, None, None, Some(c),),
                    }
                }
            }

            impl<'split> ::service_util::Split for &'split mut #ty {
                type Components = (Option<&'split mut i32>, Option<&'split mut u32>, Option<&'split mut u64>, Option<&'split mut String>,);
                fn split(self) -> Self::Components {
                    match self {
                        Self::A { a } => (Some(a), None, None, None,),
                        Self::B { b, B } => (None, Some(b), Some(B), None,),
                        Self::C { c } => (None, None, None, Some(c),),
                    }
                }
            }

            impl From<#ty> for (Option<i32>, Option<u32>, Option<u64>, Option<String>,) {
                fn from(value: #ty) -> Self {
                    ::service_util::Split::split(value)
                }
            }
        );

        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn test_split_mixed() {
        let tokens = quote!(
            #[derive(Split)]
            pub enum TestEnum {
                A,
                B(u8),
                C(u16, u32),
                D { d: u64 },
                E { e: i8, E: i16 },
            }
        );

        let output = derive_split(tokens).unwrap();

        let ty = format_ident!("TestEnum");
        let expected = quote!(
            impl ::service_util::Split for #ty {
                type Components = (Option<u8>, Option<u16>, Option<u32>, Option<u64>, Option<i8>, Option<i16>,);
                fn split(self) -> Self::Components {
                    match self {
                        Self::A => (None, None, None, None, None, None,),
                        Self::B(_0) => (Some(_0), None, None, None, None, None,),
                        Self::C(_0, _1) => (None, Some(_0), Some(_1), None, None, None,),
                        Self::D { d } => (None, None, None, Some(d), None, None,),
                        Self::E { e, E } => (None, None, None, None, Some(e), Some(E),),
                    }
                }
            }

            impl<'split> ::service_util::Split for &'split #ty {
                type Components = (Option<&'split u8>, Option<&'split u16>, Option<&'split u32>, Option<&'split u64>, Option<&'split i8>, Option<&'split i16>,);
                fn split(self) -> Self::Components {
                    match self {
                        Self::A => (None, None, None, None, None, None,),
                        Self::B(_0) => (Some(_0), None, None, None, None, None,),
                        Self::C(_0, _1) => (None, Some(_0), Some(_1), None, None, None,),
                        Self::D { d } => (None, None, None, Some(d), None, None,),
                        Self::E { e, E } => (None, None, None, None, Some(e), Some(E),),
                    }
                }
            }

            impl<'split> ::service_util::Split for &'split mut #ty {
                type Components = (Option<&'split mut u8>, Option<&'split mut u16>, Option<&'split mut u32>, Option<&'split mut u64>, Option<&'split mut i8>, Option<&'split mut i16>,);
                fn split(self) -> Self::Components {
                    match self {
                        Self::A => (None, None, None, None, None, None,),
                        Self::B(_0) => (Some(_0), None, None, None, None, None,),
                        Self::C(_0, _1) => (None, Some(_0), Some(_1), None, None, None,),
                        Self::D { d } => (None, None, None, Some(d), None, None,),
                        Self::E { e, E } => (None, None, None, None, Some(e), Some(E),),
                    }
                }
            }

            impl From<#ty> for (Option<u8>, Option<u16>, Option<u32>, Option<u64>, Option<i8>, Option<i16>,) {
                fn from(value: #ty) -> Self {
                    ::service_util::Split::split(value)
                }
            }
        );

        assert_eq!(output.to_string(), expected.to_string());
    }
}
