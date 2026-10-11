//! `#[derive(Output)]`: a journey's output, read as a struct.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

use super::Keyed;
use crate::datum::Request;

/// Implements `Output` for the struct, reading each field under its key.
pub(crate) fn expand(input: DeriveInput) -> TokenStream {
    match super::keyed(&input, "Output") {
        Ok(fields) => implementation(&input, &fields),
        Err(error) => super::failed(error, stand_in(&input)),
    }
}

fn implementation(input: &DeriveInput, fields: &[Keyed<'_>]) -> TokenStream {
    let ident = &input.ident;
    let data = format_ident!("data");
    let parameter = if fields.is_empty() {
        quote!(_)
    } else {
        quote!(#data)
    };
    let names = fields.iter().map(|keyed| keyed.field);
    let values = fields.iter().map(|keyed| read(keyed, &data));
    quote! {
        impl ::itinera::journey::Output for #ident {
            fn from_data(
                #parameter: &::itinera::journey::DataBag,
            ) -> ::core::result::Result<Self, ::itinera::journey::OutputError> {
                ::core::result::Result::Ok(Self { #(#names: #values),* })
            }
        }
    }
}

/// The expression reading a field: `data.required::<T>("key")?` or `data.optional::<T>("key")?`.
fn read(keyed: &Keyed<'_>, data: &syn::Ident) -> TokenStream {
    let key = keyed.datum.key();
    match keyed.datum.request() {
        Request::Required(ty) => quote!(#data.required::<#ty>(#key)?),
        Request::Optional(ty) => quote!(#data.optional::<#ty>(#key)?),
    }
}

/// An `Output` that is never called.
fn stand_in(input: &DeriveInput) -> TokenStream {
    let ident = &input.ident;
    let (generics, types, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #generics ::itinera::journey::Output for #ident #types #where_clause {
            fn from_data(
                _: &::itinera::journey::DataBag,
            ) -> ::core::result::Result<Self, ::itinera::journey::OutputError> {
                ::core::unreachable!()
            }
        }
    }
}
