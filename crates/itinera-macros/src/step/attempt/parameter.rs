//! A parameter of a step's or a rescue step's `run`, and the handle or service it takes.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Ident, PatType};

use super::MARKS;
use crate::marker;
use crate::step::service::Service;

/// A parameter of `run`: what it takes, where its mark is, and where its type is.
pub(super) struct Parameter {
    taken: Taken,
    pub(super) mark: Span,
    ty: Span,
}

/// What a parameter of `run` takes.
enum Taken {
    Handle(Handle),
    Service(Box<Service>),
}

/// A handle `run` takes.
#[derive(Clone, Copy, PartialEq)]
enum Handle {
    Contributor,
    Reporter,
}

/// Reads what the parameter of the `run` of this kind of step takes, and removes its attribute.
pub(super) fn parameter(parameter: &mut PatType, noun: &str) -> syn::Result<Parameter> {
    let mark = marker::single(
        marker::take(&mut parameter.attrs, &MARKS),
        &parameter.pat,
        &format!(
            "every parameter of a {noun}'s `run` after `self` is a `#[contributor]`, a \
             `#[reporter]` or a `#[service]`"
        ),
    )?;
    let taken = match marker::name(&mark).as_str() {
        "service" => Taken::Service(Box::new(Service::read(&mark, parameter)?)),
        "contributor" => Taken::Handle(Handle::Contributor),
        _ => Taken::Handle(Handle::Reporter),
    };
    if let Taken::Handle(_) = taken {
        mark.meta.require_path_only()?;
    }
    Ok(Parameter {
        taken,
        mark: mark.span(),
        ty: parameter.ty.span(),
    })
}

/// The first parameter taking a handle that an earlier one takes.
pub(super) fn repeated(parameters: &[Parameter]) -> Option<&Parameter> {
    parameters
        .iter()
        .enumerate()
        .find_map(|(index, parameter)| again(parameters, index, parameter))
}

fn again<'a>(
    parameters: &[Parameter],
    index: usize,
    parameter: &'a Parameter,
) -> Option<&'a Parameter> {
    let handle = parameter.handle()?;
    let earlier: Vec<Handle> = parameters
        .iter()
        .take(index)
        .filter_map(Parameter::handle)
        .collect();
    earlier.contains(&handle).then_some(parameter)
}

impl Parameter {
    fn handle(&self) -> Option<Handle> {
        match self.taken {
            Taken::Handle(handle) => Some(handle),
            Taken::Service(_) => None,
        }
    }

    /// The service the parameter takes, if it takes one.
    pub(super) fn service(&self) -> Option<&Service> {
        match &self.taken {
            Taken::Service(service) => Some(service),
            Taken::Handle(_) => None,
        }
    }

    /// The name of the attempt's field holding what the parameter at this index takes.
    pub(super) fn name(&self, index: usize) -> Ident {
        match &self.taken {
            Taken::Handle(handle) => handle.name(),
            Taken::Service(_) => format_ident!("service_{index}"),
        }
    }

    /// The attempt's field holding what the parameter at this index takes.
    pub(super) fn field(&self, index: usize, asynchronous: bool) -> TokenStream {
        let name = self.name(index);
        let ty = match &self.taken {
            Taken::Handle(handle) => handle.ty(asynchronous),
            Taken::Service(service) => service.borrowed(),
        };
        quote!(#name: #ty)
    }

    /// The call that declares the handle on the step's needs; a service is declared with the
    /// step's other services.
    pub(super) fn need(&self) -> Option<TokenStream> {
        let handle = self.handle()?;
        let name = handle.name();
        Some(quote!(.#name()))
    }

    /// The expression that takes what the parameter takes from what was resolved.
    pub(super) fn taken(&self, got: &Ident) -> TokenStream {
        match &self.taken {
            Taken::Handle(handle) => {
                let name = handle.name();
                quote!(#got.#name()?)
            }
            Taken::Service(service) => service.taken(got),
        }
    }

    /// What `run` is given for the parameter at this index, where the parameter's type is
    /// written, so that a handle of the wrong type is reported there.
    pub(super) fn argument(&self, index: usize) -> TokenStream {
        let name = Ident::new(&self.name(index).to_string(), self.ty);
        quote_spanned!(self.ty=> self.#name)
    }
}

impl Handle {
    fn name(self) -> Ident {
        match self {
            Self::Contributor => format_ident!("contributor"),
            Self::Reporter => format_ident!("reporter"),
        }
    }

    /// The type of the attempt's field holding the handle.
    fn ty(self, asynchronous: bool) -> TokenStream {
        match (self, asynchronous) {
            (Self::Contributor, _) => quote!(&'a mut ::itinera::journey::Contributor),
            (Self::Reporter, false) => quote!(::itinera::step::StepReporter<'a>),
            (Self::Reporter, true) => quote!(::itinera::step::AsyncStepReporter<'a>),
        }
    }
}
