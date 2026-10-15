//! What a parameter of a service provider requests, which its attribute says, or its type for a
//! handle.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::spanned::Spanned as _;
use syn::{Ident, Pat, PatType};

use crate::datum::{Arguments, Datum};
use crate::handle::Handle;
use crate::marker;

/// The attribute that marks a service provider's parameters which are not handles.
pub(in crate::workflow) const MARKS: [&str; 1] = ["from_workflow"];

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
        let mark = marker::at_most_one(
            marker::take(&mut parameter.attrs, &MARKS),
            "this attribute is one too many: each parameter carries at most one of them",
        )?;
        let request = match mark {
            Some(mark) => {
                let named = match &*parameter.pat {
                    Pat::Ident(pattern) => Some(&pattern.ident),
                    _ => None,
                };
                let datum = Arguments::of(&mark)?.datum(named, &parameter.ty)?;
                Request::FromWorkflow(Box::new(datum))
            }
            None => Request::handle(parameter, per_attempt)?,
        };
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

impl Request {
    /// The handle an unmarked parameter takes, as its type says, for a provider of attempt scope
    /// or of journey scope.
    fn handle(parameter: &PatType, per_attempt: bool) -> syn::Result<Self> {
        match (Handle::of(&parameter.ty), per_attempt) {
            (Some(Handle::JourneyId), _) => Ok(Self::JourneyId),
            (Some(Handle::Reporter), _) => Ok(Self::Reporter),
            (Some(Handle::StepName), true) => Ok(Self::StepName),
            (Some(Handle::Attempt), true) => Ok(Self::Attempt),
            (Some(Handle::StepName), false) => Err(journey_scoped(parameter, "the step's name")),
            (Some(Handle::Attempt), false) => Err(journey_scoped(parameter, "the attempt")),
            _ => Err(syn::Error::new_spanned(
                &parameter.pat,
                "every parameter of a service provider after `&self` is the journey ID, as \
                 `&JourneyId`, a reporter, as `HookReporter<'_>`, or, for a provider of attempt \
                 scope, the step's name or the attempt, as `StepName` or `Attempt`, or says what \
                 it requests, such as `#[from_workflow(\"gateway\")]`",
            )),
        }
    }
}

/// The error refusing what only a provider of attempt scope requests.
fn journey_scoped(parameter: &PatType, requested: &str) -> syn::Error {
    syn::Error::new_spanned(
        &parameter.ty,
        format!(
            "{requested} is requested only by a provider of attempt scope, marked \
             `#[provider(per_attempt)]`: one of journey scope serves whichever step needs its \
             service first"
        ),
    )
}
