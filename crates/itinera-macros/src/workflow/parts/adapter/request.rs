//! What a parameter of an input adapter requests, which its attribute says.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, PatType};

use crate::datum::{Arguments, Datum};
use crate::marker;

/// The attributes that mark an input adapter's parameters.
pub(in crate::workflow) const MARKS: [&str; 5] = [
    "step_name",
    "key",
    "from_workflow",
    "journey_id",
    "data_bag",
];

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
        let mark = marker::single(
            marker::take(&mut parameter.attrs, &MARKS),
            &parameter.pat,
            "every parameter of an input adapter after `&self` says what it requests, such as \
             `#[key]` or `#[from_workflow(\"price\")]`",
        )?;
        let request = match marker::name(&mark).as_str() {
            "from_workflow" => {
                let name = match &*parameter.pat {
                    syn::Pat::Ident(pattern) => Some(&pattern.ident),
                    _ => None,
                };
                let datum = Arguments::of(&mark)?.datum(name, &parameter.ty)?;
                return Ok(Self::FromWorkflow(Box::new(datum)));
            }
            "step_name" => Self::StepName,
            "key" => Self::Key,
            "journey_id" => Self::JourneyId,
            _ => Self::DataBag,
        };
        mark.meta.require_path_only()?;
        Ok(request)
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
