//! A step's own `new`, whose parameters are its inputs and the services it requests, and which
//! may refuse to build it.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{FnArg, Ident, ImplItemFn, Pat, ReturnType, Type};

use super::service::{self, Service};
use crate::block;
use crate::datum::{Arguments, Datum};
use crate::marker;

/// What `new` takes, in order, whether it may fail, and where its return type is written.
pub(super) struct Constructor {
    parameters: Vec<Parameter>,
    fallible: bool,
    output: Span,
}

/// What a parameter of `new` takes: an input, or a service.
enum Parameter {
    Input(Datum),
    Service(Service),
}

impl Constructor {
    /// Reads what `new` takes, and removes its parameters' attributes.
    pub(super) fn read(new: &mut ImplItemFn) -> syn::Result<Self> {
        let parameters: Vec<Parameter> = new
            .sig
            .inputs
            .iter_mut()
            .map(parameter)
            .collect::<syn::Result<_>>()?;
        let constructor = Self {
            parameters,
            fallible: block::returns_result(&new.sig.output),
            output: returned(&new.sig.output),
        };
        match service::repeated(&constructor.services().collect::<Vec<_>>()) {
            Some(error) => Err(error),
            None => Ok(constructor),
        }
    }

    /// The services `new` takes, in order.
    pub(super) fn services(&self) -> impl Iterator<Item = &Service> {
        self.parameters.iter().filter_map(Parameter::service)
    }

    /// The step's `FromInputs`, which calls `new`.
    pub(super) fn expand(&self, step: &Type) -> TokenStream {
        let got = format_ident!("got");
        let declared = self.parameters.iter().map(Parameter::declared);
        let taken = self
            .parameters
            .iter()
            .map(|parameter| parameter.taken(&got));
        let new = quote_spanned!(self.output=> <#step>::new(#(#taken),*));
        let built = if self.fallible {
            quote_spanned!(self.output=> {
                let step = #new?;
                ::core::result::Result::Ok(step)
            })
        } else {
            quote_spanned!(self.output=> ::core::result::Result::Ok(#new))
        };
        quote! {
            impl ::itinera::step::FromInputs for #step {
                fn needs() -> ::itinera::step::StepNeeds {
                    ::itinera::step::StepNeeds::new() #(#declared)*
                }

                fn from_inputs<__ItineraMode>(
                    #got: &mut ::itinera::step::Resolved<'_, __ItineraMode>,
                ) -> ::core::result::Result<Self, ::itinera::error::Error> {
                    #built
                }
            }
        }
    }
}

/// Where `new`'s return type is written, so that a step `new` does not build is reported there.
fn returned(output: &ReturnType) -> Span {
    match output {
        ReturnType::Type(_, ty) => ty.span(),
        ReturnType::Default => Span::call_site(),
    }
}

/// Reads what a parameter of `new` takes, and removes its attribute.
fn parameter(argument: &mut FnArg) -> syn::Result<Parameter> {
    let FnArg::Typed(parameter) = argument else {
        return Err(syn::Error::new_spanned(
            argument,
            "a step's `new` builds the step from its inputs alone, without `self`",
        ));
    };
    let mark = marker::single(
        marker::take(&mut parameter.attrs, &["input", "service"]),
        &parameter.pat,
        "every parameter of a step's `new` is an `#[input]` or, with the `unstable` feature, a \
         `#[service]`",
    )?;
    if marker::name(&mark) == "service" {
        return Service::read(&mark, parameter).map(Parameter::Service);
    }
    let name = match &*parameter.pat {
        Pat::Ident(pattern) => Some(&pattern.ident),
        _ => None,
    };
    Arguments::of(&mark)?
        .datum(name, &parameter.ty)
        .map(Parameter::Input)
}

impl Parameter {
    fn service(&self) -> Option<&Service> {
        match self {
            Self::Service(service) => Some(service),
            Self::Input(_) => None,
        }
    }

    /// The call that declares what the parameter takes on the step's needs.
    fn declared(&self) -> TokenStream {
        match self {
            Self::Input(input) => input.declared("input"),
            Self::Service(service) => service.declared(),
        }
    }

    /// The expression that takes what the parameter takes from what was resolved.
    fn taken(&self, got: &Ident) -> TokenStream {
        match self {
            Self::Input(input) => input.taken(got, "input"),
            Self::Service(service) => service.taken(got),
        }
    }
}
