//! What a parameter of a service provider requests, which its attribute says.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::spanned::Spanned as _;
use syn::{Ident, Pat, PatType};

use crate::datum::{Arguments, Datum};
use crate::marker;

/// The attributes that mark a service provider's parameters.
pub(in crate::workflow) const MARKS: [&str; 5] = [
    "journey_id",
    "from_workflow",
    "reporter",
    "step_name",
    "attempt",
];

/// What one parameter of a service provider requests, and where its type is written, so that a
/// request of another type is reported there.
pub(super) struct Parameter {
    request: Request,
    ty: Span,
}

enum Request {
    JourneyId,
    FromWorkflow(Box<Datum>),
    Reporter,
    StepName,
    Attempt,
}

impl Parameter {
    /// Reads what the parameter requests, for a provider of attempt scope or of journey scope,
    /// and removes its attribute.
    pub(super) fn read(parameter: &mut PatType, per_attempt: bool) -> syn::Result<Self> {
        let mark = marker::single(
            marker::take(&mut parameter.attrs, &MARKS),
            &parameter.pat,
            "every parameter of a service provider after `&self` says what it requests, such as \
             `#[journey_id]` or `#[from_workflow(\"gateway\")]`",
        )?;
        let name = marker::name(&mark);
        if !per_attempt && matches!(name.as_str(), "step_name" | "attempt") {
            return Err(syn::Error::new_spanned(
                &mark,
                format!(
                    "`#[{name}]` is requested only by a provider of attempt scope, marked \
                     `#[provider(per_attempt)]`: one of journey scope serves whichever step needs \
                     its service first"
                ),
            ));
        }
        let request = match name.as_str() {
            "from_workflow" => {
                let named = match &*parameter.pat {
                    Pat::Ident(pattern) => Some(&pattern.ident),
                    _ => None,
                };
                let datum = Arguments::of(&mark)?.datum(named, &parameter.ty)?;
                Request::FromWorkflow(Box::new(datum))
            }
            "journey_id" => Request::JourneyId,
            "reporter" => Request::Reporter,
            "step_name" => Request::StepName,
            _ => Request::Attempt,
        };
        if !matches!(request, Request::FromWorkflow(_)) {
            mark.meta.require_path_only()?;
        }
        Ok(Self {
            request,
            ty: parameter.ty.span(),
        })
    }

    /// The call that declares the request on the provider's needs, if the needs declare it.
    pub(super) fn need(&self) -> Option<TokenStream> {
        match &self.request {
            Request::FromWorkflow(datum) => Some(datum.declared("from_workflow")),
            Request::Reporter => Some(quote!(.reporter())),
            Request::JourneyId | Request::StepName | Request::Attempt => None,
        }
    }

    pub(super) fn is_reporter(&self) -> bool {
        matches!(self.request, Request::Reporter)
    }

    /// Whether the request is taken out of what was resolved, which needs it mutably.
    pub(super) fn takes(&self) -> bool {
        self.need().is_some()
    }

    /// The expression that gives the parameter its value from what was resolved.
    pub(super) fn taken(&self, got: &Ident) -> TokenStream {
        match &self.request {
            Request::FromWorkflow(datum) => datum.taken(got, "from_workflow"),
            Request::Reporter => quote!(#got.reporter()?),
            Request::JourneyId => quote!(#got.journey_id()),
            Request::StepName => quote!(#got.step_name()),
            Request::Attempt => quote!(#got.attempt()),
        }
    }

    /// The variable holding the request at this index, named where the parameter's type is
    /// written.
    pub(super) fn argument(&self, index: usize) -> Ident {
        format_ident!("request_{index}", span = self.ty)
    }
}
