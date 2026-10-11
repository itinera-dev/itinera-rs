//! `#[derive(RescueStep)]`: a rescue step built from its fields, each a request: an input, data
//! from the step it rescues, the failure's reason or the error.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Attribute, Data, DeriveInput, Field, Fields, Ident, Type};

use crate::datum::{Arguments, Datum, option_of};
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
             `#[from_rescued_step]`, a `#[reason]` or an `#[error]`",
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
    /// What the failed attempt left, required unless the field is an `Option`. The field's type
    /// says where the request is written.
    Left(Left, Type),
}

/// Where data requested by key comes from.
#[derive(Clone, Copy)]
enum Source {
    DataBag,
    RescuedStep,
}

impl Source {
    /// The prefix of the methods that request data from it.
    fn prefix(self) -> &'static str {
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
    let mark = marker::single(
        marker::found(
            &field.attrs,
            &["input", "from_rescued_step", "reason", "error"],
        ),
        field,
        "every field of a rescue step deriving `RescueStep` is an `#[input]`, a \
         `#[from_rescued_step]`, a `#[reason]` or an `#[error]`",
    )?;
    match marker::name(mark).as_str() {
        "input" => keyed(mark, field, Source::DataBag),
        "from_rescued_step" => keyed(mark, field, Source::RescuedStep),
        "reason" => unkeyed(mark, field, Left::Reason),
        _ => unkeyed(mark, field, Left::Error),
    }
}

fn keyed(mark: &Attribute, field: &Field, source: Source) -> syn::Result<Request> {
    let datum = Arguments::of(mark)?.datum(field.ident.as_ref(), &field.ty)?;
    Ok(Request::Datum(datum, source))
}

/// The request of what the attempt left, whose attribute takes no arguments.
fn unkeyed(mark: &Attribute, field: &Field, left: Left) -> syn::Result<Request> {
    mark.meta
        .require_path_only()
        .map_err(|_| arguments_refused(mark, left))?;
    Ok(Request::Left(left, field.ty.clone()))
}

fn arguments_refused(mark: &Attribute, left: Left) -> syn::Error {
    let name = left.name();
    syn::Error::new_spanned(
        &mark.meta,
        format!(
            "`#[{name}]` takes no arguments: a failed attempt leaves at most one {name}, under \
             no key"
        ),
    )
}

impl Request {
    /// The call that declares the request on the rescue step's needs.
    fn declared(&self) -> TokenStream {
        match self {
            Self::Datum(datum, source) => datum.declared(source.prefix()),
            Self::Left(left, ty) => {
                let method = method(*left, ty);
                quote!(.#method())
            }
        }
    }

    /// The expression that takes what was resolved for the request.
    fn taken(&self, got: &Ident) -> TokenStream {
        match self {
            Self::Datum(datum, source) => datum.taken(got, source.prefix()),
            Self::Left(left, ty) => {
                let method = method(*left, ty);
                quote_spanned!(ty.span()=> #got.#method()?)
            }
        }
    }
}

/// The method requesting what the attempt left into a field of this type, such as
/// `optional_reason` for an `Option`.
fn method(left: Left, ty: &Type) -> Ident {
    let name = left.name();
    match option_of(ty) {
        Some(_) => format_ident!("optional_{name}"),
        None => format_ident!("{name}"),
    }
}
