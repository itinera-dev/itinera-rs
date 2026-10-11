//! `#[derive(InitialData)]` and `#[derive(Output)]`: journey data given or read as a struct,
//! each field one key of the data bag.

mod initial_data;
mod output;

pub(crate) use initial_data::expand as expand_initial_data;
pub(crate) use output::expand as expand_output;

use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned as _;
use syn::{Attribute, Data, DeriveInput, Field, Fields, Ident, LitStr};

use crate::datum::{Arguments, Datum};
use crate::marker;

/// A field of the struct, and the datum it is.
struct Keyed<'a> {
    field: &'a Ident,
    datum: Datum,
}

/// The struct's fields, each with its key, or an error when the type is generic or is not a
/// struct with named fields or none, or when two fields give the same key.
fn keyed<'a>(input: &'a DeriveInput, derived: &str) -> syn::Result<Vec<Keyed<'a>>> {
    if let Some(parameter) = input.generics.params.first() {
        return Err(syn::Error::new_spanned(
            parameter,
            format!("a generic type implements `{derived}` by hand, not with the derive"),
        ));
    }
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!("`{derived}` is derived only on a struct"),
        ));
    };
    match &data.fields {
        Fields::Named(fields) => {
            let keyed = fields
                .named
                .iter()
                .map(key)
                .collect::<syn::Result<Vec<_>>>()?;
            let mut given = HashSet::new();
            keyed
                .iter()
                .try_for_each(|field| field.given_once(&mut given))?;
            Ok(keyed)
        }
        Fields::Unit => Ok(Vec::new()),
        Fields::Unnamed(_) => Err(syn::Error::new_spanned(
            &input.ident,
            format!("a struct deriving `{derived}` has named fields, each naming its key, or none"),
        )),
    }
}

/// The datum a field is: its key is its name unless `#[key("...")]` names it, and its type says
/// whether it is optional, unless `#[required]` makes an `Option` required.
fn key(field: &Field) -> syn::Result<Keyed<'_>> {
    let Some(name) = &field.ident else {
        return Err(syn::Error::new_spanned(
            field,
            "a field of a struct with named fields has a name",
        ));
    };
    let named = marker::at_most_one(
        marker::found(&field.attrs, &["key"]),
        "a field is given one key",
    )?
    .map(named_key)
    .transpose()?;
    let required = marker::at_most_one(
        marker::found(&field.attrs, &["required"]),
        "a field is marked `#[required]` once",
    )?
    .map(marked_required)
    .transpose()?;
    let datum = Arguments::new(named, required).datum(Some(name), &field.ty)?;
    Ok(Keyed { field: name, datum })
}

impl Keyed<'_> {
    /// Refuses the field's key when an earlier field gives it already, and notes it otherwise.
    fn given_once(&self, given: &mut HashSet<String>) -> syn::Result<()> {
        let key = self.datum.key();
        if given.insert(key.value()) {
            return Ok(());
        }
        Err(syn::Error::new_spanned(
            key,
            format!(
                "key given twice: `{}`, which an earlier field gives already",
                key.value()
            ),
        ))
    }
}

fn named_key(mark: &Attribute) -> syn::Result<LitStr> {
    mark.parse_args().map_err(|error| {
        syn::Error::new(
            error.span(),
            "`#[key]` names the key, such as `#[key(\"note\")]`",
        )
    })
}

fn marked_required(mark: &Attribute) -> syn::Result<Span> {
    mark.meta
        .require_path_only()
        .map_err(|error| syn::Error::new(error.span(), "`#[required]` takes no arguments"))?;
    Ok(mark.path().span())
}

/// The derive's error, followed by a stand-in implementation of the trait, so that the compiler
/// reports that error alone.
fn failed(error: syn::Error, stand_in: TokenStream) -> TokenStream {
    let error = error.into_compile_error();
    quote! {
        #error
        #stand_in
    }
}
