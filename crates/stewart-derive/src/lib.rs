use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive `Family` implementation for common cases.
///
/// Supports:
/// - Owned types, as `Type`
/// - Borrowed types with one lifetime, as `Type<'static>`
#[proc_macro_derive(Family)]
pub fn derive_family(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let DeriveInput {
        ident, generics, ..
    } = input;

    let lifetimes = generics.lifetimes().count();
    let (generics_impl, generics_gat) = match lifetimes {
        0 => (quote! {}, quote! {}),
        1 => (quote! { <'static> }, quote! { <'a> }),
        _ => panic!("derive macro only supports 0 or 1 lifetimes"),
    };

    let output = quote! {
        impl Family for #ident #generics_impl {
            type Member<'a> = #ident #generics_gat;
        }
    };
    output.into()
}
