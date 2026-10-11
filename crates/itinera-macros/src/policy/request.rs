//! What a hook's parameter requests, which its attribute says.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Pat, PatType, Type};

use super::hook::HookName;
use crate::datum::{Arguments, Datum, option_of};
use crate::marker;

/// The attributes that mark a hook's parameters.
pub(super) const MARKS: [&str; 11] = [
    "from_step",
    "from_workflow",
    "step_name",
    "attempt",
    "reason",
    "cause",
    "error",
    "journey_id",
    "role",
    "contributor",
    "reporter",
];

/// What one parameter of a hook requests.
pub(super) enum Request {
    FromStep(Datum),
    FromWorkflow(Datum),
    StepName,
    Attempt,
    Reason {
        optional: bool,
    },
    Cause,
    Error {
        optional: bool,
    },
    JourneyId,
    /// The role, such as `dyn Notifier` for a parameter of type `&dyn Notifier`.
    Role(Type),
    Contributor,
    Reporter,
}

const STEP_HOOKS: [HookName; 4] = [
    HookName::OnStepSuccess,
    HookName::OnStepFailure,
    HookName::OnStepRetry,
    HookName::OnStepAbnormalTermination,
];

const FAILURE_HOOKS: [HookName; 2] = [HookName::OnStepFailure, HookName::OnStepRetry];

const ERROR_HOOKS: [HookName; 3] = [
    HookName::OnStepFailure,
    HookName::OnStepRetry,
    HookName::OnStepAbnormalTermination,
];

impl Request {
    /// Reads what the parameter requests, and removes its attribute.
    pub(super) fn read(parameter: &mut PatType, hook: HookName) -> syn::Result<Self> {
        let mark = marker::single(
            marker::take(&mut parameter.attrs, &MARKS),
            &parameter.pat,
            "every parameter of a hook after `&self` says what it requests, such as \
             `#[from_step(\"receipt\")]` or `#[reporter]`",
        )?;
        let name = marker::name(&mark);
        let allowed: &[HookName] = match name.as_str() {
            "from_step" | "step_name" | "attempt" => &STEP_HOOKS,
            "reason" | "cause" => &FAILURE_HOOKS,
            "error" => &ERROR_HOOKS,
            _ => &[],
        };
        if !allowed.is_empty() && !allowed.contains(&hook) {
            return Err(syn::Error::new_spanned(
                &mark,
                format!(
                    "`#[{name}]` is requested only by {}",
                    HookName::spoken_list(allowed)
                ),
            ));
        }
        let ty = &*parameter.ty;
        let request = match name.as_str() {
            "from_step" => Self::FromStep(Arguments::of(&mark)?.datum(named(&parameter.pat), ty)?),
            "from_workflow" => {
                Self::FromWorkflow(Arguments::of(&mark)?.datum(named(&parameter.pat), ty)?)
            }
            "reason" => Self::Reason {
                optional: option_of(ty).is_some(),
            },
            "error" => Self::Error {
                optional: option_of(ty).is_some(),
            },
            "role" => Self::Role(referent(ty)?.clone()),
            "step_name" => Self::StepName,
            "attempt" => Self::Attempt,
            "cause" => Self::Cause,
            "journey_id" => Self::JourneyId,
            "contributor" => Self::Contributor,
            _ => Self::Reporter,
        };
        if !matches!(request, Self::FromStep(_) | Self::FromWorkflow(_)) {
            mark.meta.require_path_only()?;
        }
        Ok(request)
    }

    /// The call that declares the request on the hook's needs, if it is one the needs declare.
    pub(super) fn need(&self) -> Option<TokenStream> {
        match self {
            Self::FromStep(datum) => Some(datum.declared("from_step")),
            Self::FromWorkflow(datum) => Some(datum.declared("from_workflow")),
            Self::Reason { optional: false } => Some(quote!(.reason())),
            Self::Reason { optional: true } => Some(quote!(.optional_reason())),
            Self::Error { optional: false } => Some(quote!(.error())),
            Self::Error { optional: true } => Some(quote!(.optional_error())),
            Self::Contributor => Some(quote!(.contributor())),
            Self::Reporter => Some(quote!(.reporter())),
            Self::StepName | Self::Attempt | Self::Cause | Self::JourneyId | Self::Role(_) => None,
        }
    }

    /// Whether the request is taken out of what was resolved, which needs it mutably.
    pub(super) fn takes(&self) -> bool {
        self.need().is_some()
    }

    /// The expression that gives the parameter its value from what was resolved.
    pub(super) fn taken(&self, got: &Ident) -> TokenStream {
        match self {
            Self::FromStep(datum) => datum.taken(got, "from_step"),
            Self::FromWorkflow(datum) => datum.taken(got, "from_workflow"),
            Self::Reason { optional: false } => quote!(#got.reason()?),
            Self::Reason { optional: true } => quote!(#got.optional_reason()?),
            Self::Error { optional: false } => quote!(#got.error()?),
            Self::Error { optional: true } => quote!(#got.optional_error()?),
            Self::Contributor => quote!(#got.contributor()?),
            Self::Reporter => quote!(#got.reporter()?),
            Self::StepName => quote!(#got.step_name()),
            Self::Attempt => quote!(#got.attempt()),
            Self::Cause => quote!(#got.cause()),
            Self::JourneyId => quote!(#got.journey_id()),
            Self::Role(role) => quote!(#got.role::<#role>()),
        }
    }

    /// The role the parameter requests, if it requests one.
    pub(super) fn role(&self) -> Option<&Type> {
        match self {
            Self::Role(role) => Some(role),
            _ => None,
        }
    }
}

/// The parameter's name, when its pattern is one.
fn named(pattern: &Pat) -> Option<&Ident> {
    match pattern {
        Pat::Ident(pattern) => Some(&pattern.ident),
        _ => None,
    }
}

/// The type a reference refers to, such as `dyn Notifier` for `&dyn Notifier`.
fn referent(ty: &Type) -> syn::Result<&Type> {
    match ty {
        Type::Reference(reference) if reference.mutability.is_none() => Ok(&reference.elem),
        _ => Err(syn::Error::new_spanned(
            ty,
            "a role is taken by shared reference, such as `&dyn Notifier`",
        )),
    }
}
