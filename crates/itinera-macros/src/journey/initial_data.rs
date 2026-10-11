//! `#[derive(InitialData)]`: the data a journey starts with, given as a struct.

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use super::Keyed;
use crate::datum::Request;

/// Implements `InitialData` for the struct, giving each field under its key.
pub(crate) fn expand(input: DeriveInput) -> TokenStream {
    match super::keyed(&input, "InitialData") {
        Ok(fields) => implementation(&input, &fields),
        Err(error) => super::failed(error, implementation(&input, &[])),
    }
}

/// `InitialData`, whose entries are the fields given, each required field always and each
/// optional one only when it is `Some`. Without fields, it is the stand-in that follows an error.
fn implementation(input: &DeriveInput, fields: &[Keyed<'_>]) -> TokenStream {
    let ident = &input.ident;
    let (generics, types, where_clause) = input.generics.split_for_impl();
    let given = fields.iter().filter_map(given);
    let optional: Vec<TokenStream> = fields.iter().filter_map(optional).collect();
    let mutability = (!optional.is_empty()).then(|| quote!(mut));
    quote! {
        impl #generics ::itinera::journey::InitialData for #ident #types #where_clause {
            fn entries(
                self,
            ) -> impl ::core::iter::IntoIterator<
                Item = (::std::string::String, ::itinera::value::AnyValue),
            > {
                let #mutability entries = ::std::vec::Vec::from([#(#given),*]);
                #(#optional)*
                entries
            }
        }
    }
}

/// The entry of a required field.
fn given(keyed: &Keyed<'_>) -> Option<TokenStream> {
    let Request::Required(ty) = keyed.datum.request() else {
        return None;
    };
    let key = keyed.datum.key();
    let field = keyed.field;
    Some(quote! {
        (
            ::std::string::String::from(#key),
            ::itinera::value::AnyValue::new::<#ty>(self.#field),
        )
    })
}

/// The statement adding the entry of an optional field when it is `Some`.
fn optional(keyed: &Keyed<'_>) -> Option<TokenStream> {
    let Request::Optional(ty) = keyed.datum.request() else {
        return None;
    };
    let key = keyed.datum.key();
    let field = keyed.field;
    Some(quote! {
        if let ::core::option::Option::Some(value) = self.#field {
            entries.push((
                ::std::string::String::from(#key),
                ::itinera::value::AnyValue::new::<#ty>(value),
            ));
        }
    })
}
