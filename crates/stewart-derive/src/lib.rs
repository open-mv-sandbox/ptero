use heck::ToKebabCase;
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, Attribute, DeriveInput, Path};

/// Derive `Factory` implementation from typed target actor start function.
#[proc_macro_derive(Factory, attributes(factory))]
pub fn derive_factory(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let DeriveInput { ident, attrs, .. } = input;

    // Find the target
    let factory_fn = find_attr(attrs);
    let actor_type = infer_actor_type(&factory_fn);

    // Infer the logging name from the actor type
    let actor_ident = actor_type.get_ident().unwrap();
    let converted_type_name = actor_ident.to_string().to_kebab_case();
    let actor_logging_name = converted_type_name.trim_end_matches("-actor");

    // Build the final output
    let output = quote! {
        impl stewart::Factory for #ident {
            fn create_span(&self) -> stewart::tracing::Span {
                stewart::tracing::span!(stewart::tracing::Level::INFO, #actor_logging_name)
            }

            fn start(
                self: Box<Self>,
                system: &mut stewart::System,
                id: stewart::ActorId,
            ) -> Result<Box<dyn stewart::AnyActor>, stewart::Error> {
                let addr = stewart::ActorAddr::<<#actor_type as stewart::Actor>::Protocol>::from_id(id);
                let actor = #factory_fn(system, addr, *self)?;
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

fn infer_actor_type(factory_fn: &Path) -> Path {
    if factory_fn.segments.len() < 2 {
        panic!("factory function path must contain actor type when inferring actor");
    }

    let mut segments = Punctuated::new();
    for (i, segment) in factory_fn.segments.clone().into_iter().enumerate() {
        // Skip last
        if i == factory_fn.segments.len() - 1 {
            continue;
        }

        segments.push(segment);
    }

    Path {
        leading_colon: None,
        segments,
    }
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
