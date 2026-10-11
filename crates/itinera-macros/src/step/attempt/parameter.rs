//! A parameter of a step's `run`, and the handle it takes.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::PatType;
use syn::spanned::Spanned as _;

use crate::marker;

/// A parameter of `run`: the handle it takes, where its mark is, and where its type is.
pub(super) struct Parameter {
    handle: Handle,
    pub(super) mark: Span,
    ty: Span,
}

/// A handle `run` takes.
#[derive(Clone, Copy, PartialEq)]
enum Handle {
    Contributor,
    Reporter,
}

/// Reads which handle the parameter of the `run` of this kind of step takes, and removes its
/// attribute.
pub(super) fn parameter(parameter: &mut PatType, noun: &str) -> syn::Result<Parameter> {
    let mark = marker::single(
        marker::take(&mut parameter.attrs, &["contributor", "reporter"]),
        &parameter.pat,
        &format!(
            "every parameter of a {noun}'s `run` after `self` is a `#[contributor]` or a \
             `#[reporter]`"
        ),
    )?;
    mark.meta.require_path_only()?;
    let handle = match marker::name(&mark).as_str() {
        "contributor" => Handle::Contributor,
        _ => Handle::Reporter,
    };
    Ok(Parameter {
        handle,
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
    let earlier: Vec<Handle> = parameters
        .iter()
        .take(index)
        .map(Parameter::handle)
        .collect();
    earlier.contains(&parameter.handle).then_some(parameter)
}

impl Parameter {
    fn handle(&self) -> Handle {
        self.handle
    }

    pub(super) fn name(&self) -> syn::Ident {
        self.handle.name()
    }

    /// The attempt's field holding the handle.
    pub(super) fn field(&self, asynchronous: bool) -> TokenStream {
        self.handle.field(asynchronous)
    }

    /// The handle as `run` is given it, where the parameter's type is written, so that a handle
    /// of the wrong type is reported there.
    pub(super) fn argument(&self) -> TokenStream {
        let name = syn::Ident::new(&self.name().to_string(), self.ty);
        quote_spanned!(self.ty=> self.#name)
    }
}

impl Handle {
    fn name(&self) -> syn::Ident {
        match self {
            Self::Contributor => format_ident!("contributor"),
            Self::Reporter => format_ident!("reporter"),
        }
    }

    /// The attempt's field holding the handle.
    pub(super) fn field(&self, asynchronous: bool) -> TokenStream {
        let name = self.name();
        let ty = match (self, asynchronous) {
            (Self::Contributor, _) => quote!(::itinera::journey::Contributor<'a>),
            (Self::Reporter, false) => quote!(::itinera::step::StepReporter<'a>),
            (Self::Reporter, true) => quote!(::itinera::step::AsyncStepReporter<'a>),
        };
        quote!(#name: #ty)
    }
}
