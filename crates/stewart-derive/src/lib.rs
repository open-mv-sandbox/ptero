use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, token::Colon2, Attribute, DeriveInput, Path};

/// Derive `Factory` implementation from typed target actor start function.
#[proc_macro_derive(Factory, attributes(factory))]
pub fn derive_factory(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let DeriveInput { ident, attrs, .. } = input;

    // Find the target function
    let factory_fn = find_attr(attrs);

    // Infer target actor type
    if factory_fn.segments.len() < 2 {
        panic!("factory function path must contain actor type when inferring actor");
    }

    let mut actor_type: Punctuated<_, Colon2> = Punctuated::new();
    for (i, segment) in factory_fn.segments.clone().into_iter().enumerate() {
        // Skip last
        if i == factory_fn.segments.len() - 1 {
            continue;
        }

        actor_type.push(segment);
    }

    // Build the final output
    let output = quote! {
        impl stewart::Factory for #ident {
            fn start(
                self: Box<Self>,
                id: stewart::ActorId,
            ) -> Result<Box<dyn stewart::AnyActor>, stewart::Error> {
                let addr = stewart::ActorAddr::<<#actor_type as stewart::Actor>::Protocol>::from_id(id);
                let actor = #factory_fn(addr, *self);
                Ok(Box::new(actor))
            }
        }
    };
    output.into()
}

fn find_attr(attrs: Vec<Attribute>) -> Path {
    for attr in attrs {
        if !attr.path.is_ident("factory") {
            continue;
        }

        let path: Path = attr
            .parse_args()
            .expect("wrong format of \"factory\" attribute");
        return path;
    }

    panic!("unable to find \"factory\" attribute")
}

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
