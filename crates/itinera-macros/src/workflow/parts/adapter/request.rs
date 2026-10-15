//! What a parameter of an input adapter requests, which its attribute says, or its type for a
//! handle.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, PatType};

use crate::datum::{Arguments, Datum};
use crate::handle::Handle;
use crate::marker;

/// The attributes that mark an input adapter's parameters, which are not handles.
pub(in crate::workflow) const MARKS: [&str; 2] = ["key", "from_workflow"];

/// What one parameter of an input adapter requests.
pub(super) enum Request {
    StepName,
    Key,
    FromWorkflow(Box<Datum>),
    JourneyId,
    DataBag,
}

impl Request {
    /// Reads what the parameter requests, and removes its attribute.
    pub(super) fn read(parameter: &mut PatType) -> syn::Result<Self> {
        let mark = marker::at_most_one(
            marker::take(&mut parameter.attrs, &MARKS),
            "this attribute is one too many: each parameter carries at most one of them",
        )?;
        let Some(mark) = mark else {
            return Self::handle(parameter);
        };
        if marker::name(&mark) == "key" {
            mark.meta.require_path_only()?;
            return Ok(Self::Key);
        }
        let name = match &*parameter.pat {
            syn::Pat::Ident(pattern) => Some(&pattern.ident),
            _ => None,
        };
        let datum = Arguments::of(&mark)?.datum(name, &parameter.ty)?;
        Ok(Self::FromWorkflow(Box::new(datum)))
    }

    /// The handle an unmarked parameter takes, as its type says.
    fn handle(parameter: &PatType) -> syn::Result<Self> {
        match Handle::of(&parameter.ty) {
            Some(Handle::StepName) => Ok(Self::StepName),
            Some(Handle::JourneyId) => Ok(Self::JourneyId),
            Some(Handle::DataBag) => Ok(Self::DataBag),
            _ => Err(syn::Error::new_spanned(
                &parameter.pat,
                "every parameter of an input adapter after `&self` is the step's name, the \
                 journey ID or the data bag, as `StepName`, `&JourneyId` or `&DataBag`, or says \
                 what it requests, such as `#[key]` or `#[from_workflow(\"price\")]`",
            )),
        }
    }

    /// The call that declares the request on the adapter's needs, if the needs declare it.
    pub(super) fn need(&self) -> Option<TokenStream> {
        match self {
            Self::FromWorkflow(datum) => Some(datum.declared("from_workflow")),
            Self::DataBag => Some(quote!(.data_bag())),
            Self::StepName | Self::Key | Self::JourneyId => None,
        }
    }

    pub(super) fn takes(&self) -> bool {
        self.need().is_some()
    }

    /// The expression that gives the parameter its value from what was resolved.
    pub(super) fn taken(&self, got: &Ident) -> TokenStream {
        match self {
            Self::FromWorkflow(datum) => datum.taken(got, "from_workflow"),
            Self::DataBag => quote!(#got.data_bag()?),
            Self::StepName => quote!(#got.step_name()),
            Self::Key => quote!(#got.key()),
            Self::JourneyId => quote!(#got.journey_id()),
        }
    }
}
