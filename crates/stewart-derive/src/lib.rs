use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive `Protocol` implementation for common message cases.
///
/// Supports:
/// - Owned message types, as `Type`
/// - Borrowed message types with one lifetime, as `Type<'static>`
#[proc_macro_derive(Protocol)]
pub fn derive_protocol(input: TokenStream) -> TokenStream {
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
        impl Protocol for #ident #generics_impl {
            type Message<'a> = #ident #generics_gat;
        }
    };
    output.into()
}
