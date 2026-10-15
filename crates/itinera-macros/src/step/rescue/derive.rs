//! `#[derive(RescueStep)]`: a rescue step built from its fields, each a request: an input, data
//! from the step it rescues, the failure's reason or the error.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Attribute, Data, DeriveInput, Field, Fields, Ident, Type};

use crate::datum::{Arguments, Datum};
use crate::handle::Handle;
use crate::{described, marker};

/// Implements `FromRescueRequests` for the struct, from its fields. On an error, it implements a
/// stand-in instead, so that the compiler reports that error alone.
pub(crate) fn expand(input: DeriveInput) -> TokenStream {
    derive(&input).unwrap_or_else(|error| failed(error, &input))
}

fn derive(input: &DeriveInput) -> syn::Result<TokenStream> {
    described::refuse_generics(&input.generics, "rescue step")?;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "`RescueStep` is derived only on a struct",
        ));
    };
    if let Fields::Unnamed(_) = &data.fields {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "a rescue step deriving `RescueStep` has named fields, each an `#[input]`, a \
             `#[from_rescued_step]`, the reason or the error",
        ));
    }
    let requests = data
        .fields
        .iter()
        .map(request)
        .collect::<syn::Result<Vec<_>>>()?;
    let names = data.fields.iter().map(|field| &field.ident);
    let got = format_ident!("got");
    let declared = requests.iter().map(Request::declared);
    let values = requests.iter().map(|request| request.taken(&got));
    let ident = &input.ident;
    Ok(quote! {
        impl ::itinera::step::FromRescueRequests for #ident {
            fn needs() -> ::itinera::step::RescueNeeds {
                ::itinera::step::RescueNeeds::new() #(#declared)*
            }

            fn from_requests<__ItineraMode>(
                #got: &mut ::itinera::step::RescueResolved<'_, __ItineraMode>,
            ) -> ::core::result::Result<Self, ::itinera::error::Error> {
                ::core::result::Result::Ok(Self { #(#names: #values),* })
            }
        }
    })
}

/// The error, and a `FromRescueRequests` that is never called.
fn failed(error: syn::Error, input: &DeriveInput) -> TokenStream {
    let error = error.into_compile_error();
    let (generics, types, where_clause) = input.generics.split_for_impl();
    let ident = &input.ident;
    quote! {
        #error

        impl #generics ::itinera::step::FromRescueRequests for #ident #types #where_clause {
            fn needs() -> ::itinera::step::RescueNeeds {
                ::core::unreachable!()
            }

            fn from_requests<__ItineraMode>(
                _: &mut ::itinera::step::RescueResolved<'_, __ItineraMode>,
            ) -> ::core::result::Result<Self, ::itinera::error::Error> {
                ::core::unreachable!()
            }
        }
    }
}

/// What a field of a rescue step requests.
enum Request {
    /// Data by key, from where the source says.
    Datum(Datum, Source),
    /// What the failed attempt left, and whether the field is an `Option` of it. The field's
    /// type says where the request is written.
    Left(Left, bool, Type),
}

/// Where data requested by key comes from.
#[derive(Clone, Copy)]
enum Source {
    DataBag,
    RescuedStep,
}

impl Source {
    /// The method that requests data from it.
    fn method(self) -> &'static str {
        match self {
            Self::DataBag => "input",
            Self::RescuedStep => "from_rescued_step",
        }
    }
}

/// What a failed attempt left, besides its contributions.
#[derive(Clone, Copy)]
enum Left {
    Reason,
    Error,
}

impl Left {
    /// Its name, which is also the method that requests it.
    fn name(self) -> &'static str {
        match self {
            Self::Reason => "reason",
            Self::Error => "error",
        }
    }
}

/// What the field requests.
fn request(field: &Field) -> syn::Result<Request> {
    let mark = marker::at_most_one(
        marker::found(&field.attrs, &["input", "from_rescued_step"]),
        "this attribute is one too many: each field carries at most one of them",
    )?;
    match (mark, Handle::of(&field.ty)) {
        (Some(mark), _) if marker::name(mark) == "input" => keyed(mark, field, Source::DataBag),
        (Some(mark), _) => keyed(mark, field, Source::RescuedStep),
        (None, Some(Handle::Reason { optional })) => {
            Ok(Request::Left(Left::Reason, optional, field.ty.clone()))
        }
        (None, Some(Handle::Error { optional })) => {
            Ok(Request::Left(Left::Error, optional, field.ty.clone()))
        }
        (None, _) => Err(syn::Error::new_spanned(
            field,
            "every field of a rescue step deriving `RescueStep` is an `#[input]`, a \
             `#[from_rescued_step]`, the reason, as `Reason` or `Option<Reason>`, or the error, \
             as `Error` or `Option<Error>`",
        )),
    }
}

fn keyed(mark: &Attribute, field: &Field, source: Source) -> syn::Result<Request> {
    let datum = Arguments::of(mark)?.datum(field.ident.as_ref(), &field.ty)?;
    Ok(Request::Datum(datum, source))
}

impl Request {
    /// The call that declares the request on the rescue step's needs.
    fn declared(&self) -> TokenStream {
        match self {
            Self::Datum(datum, source) => datum.declared(source.method()),
            Self::Left(left, optional, _) => {
                let method = method(*left, *optional);
                quote!(.#method())
            }
        }
    }

    /// The expression that takes what was resolved for the request.
    fn taken(&self, got: &Ident) -> TokenStream {
        match self {
            Self::Datum(datum, source) => datum.taken(got, source.method()),
            Self::Left(left, optional, ty) => {
                let method = method(*left, *optional);
                quote_spanned!(ty.span()=> #got.#method()?)
            }
        }
    }
}

/// The method requesting what the attempt left, such as `optional_reason` into an `Option`.
fn method(left: Left, optional: bool) -> Ident {
    let name = left.name();
    if optional {
        format_ident!("optional_{name}")
    } else {
        format_ident!("{name}")
    }
}
