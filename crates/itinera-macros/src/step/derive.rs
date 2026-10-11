//! `#[derive(Step)]`: a step built from its fields, each an input or a default.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Field, Fields, Ident};

use crate::datum::{Arguments, Datum};
use crate::{described, marker};

/// Implements `FromInputs` for the struct, from its fields. On an error, it implements a stand-in
/// instead, so that the compiler reports that error alone.
pub(crate) fn expand(input: DeriveInput) -> TokenStream {
    derive(&input).unwrap_or_else(|error| failed(error, &input))
}

fn derive(input: &DeriveInput) -> syn::Result<TokenStream> {
    described::refuse_generics(&input.generics, "step")?;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "`Step` is derived only on a struct",
        ));
    };
    if let Fields::Unnamed(_) = &data.fields {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "a step deriving `Step` has named fields, each an `#[input]` or a `#[default]`",
        ));
    }
    let parts = data
        .fields
        .iter()
        .map(part)
        .collect::<syn::Result<Vec<_>>>()?;
    let names = data.fields.iter().map(|field| &field.ident);
    let got = format_ident!("got");
    let declared = parts.iter().flatten().map(declared);
    let values = parts.iter().map(|part| value(part.as_ref(), &got));
    let ident = &input.ident;
    Ok(quote! {
        impl ::itinera::step::FromInputs for #ident {
            fn needs() -> ::itinera::step::StepNeeds {
                ::itinera::step::StepNeeds::new() #(#declared)*
            }

            fn from_inputs<__ItineraMode>(
                #got: &mut ::itinera::step::Resolved<'_, __ItineraMode>,
            ) -> ::core::result::Result<Self, ::itinera::error::Error> {
                ::core::result::Result::Ok(Self { #(#names: #values),* })
            }
        }
    })
}

/// The error, and a `FromInputs` that is never called.
fn failed(error: syn::Error, input: &DeriveInput) -> TokenStream {
    let error = error.into_compile_error();
    let (generics, types, where_clause) = input.generics.split_for_impl();
    let ident = &input.ident;
    quote! {
        #error

        impl #generics ::itinera::step::FromInputs for #ident #types #where_clause {
            fn needs() -> ::itinera::step::StepNeeds {
                ::core::unreachable!()
            }

            fn from_inputs<__ItineraMode>(
                _: &mut ::itinera::step::Resolved<'_, __ItineraMode>,
            ) -> ::core::result::Result<Self, ::itinera::error::Error> {
                ::core::unreachable!()
            }
        }
    }
}

/// The input a field is, or `None` for a field made with `Default`.
fn part(field: &Field) -> syn::Result<Option<Datum>> {
    let mark = marker::single(
        marker::found(&field.attrs, &["input", "default"]),
        field,
        "every field of a step deriving `Step` is an `#[input]` or a `#[default]`",
    )?;
    if marker::name(mark) == "default" {
        mark.meta.require_path_only()?;
        return Ok(None);
    }
    Arguments::of(mark)?
        .datum(field.ident.as_ref(), &field.ty)
        .map(Some)
}

fn declared(input: &Datum) -> TokenStream {
    input.declared("input")
}

fn value(input: Option<&Datum>, got: &Ident) -> TokenStream {
    match input {
        Some(input) => input.taken(got, "input"),
        None => quote!(::core::default::Default::default()),
    }
}
