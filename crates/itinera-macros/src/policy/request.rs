//! What a hook's parameter requests, which its attribute says, or its type for a handle.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, Pat, PatType, Type};

use super::hook::HookName;
use crate::datum::{Arguments, Datum};
use crate::handle::Handle;
use crate::marker;

/// The attributes that mark a hook's parameters, which are not handles.
pub(super) const MARKS: [&str; 3] = ["from_step", "from_workflow", "role"];

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
        let mark = marker::at_most_one(
            marker::take(&mut parameter.attrs, &MARKS),
            "this attribute is one too many: each parameter carries at most one of them",
        )?;
        let ty = &*parameter.ty;
        let request = match mark {
            Some(mark) => Self::marked(&mark, &parameter.pat, ty)?,
            None => Self::handle(parameter)?,
        };
        let allowed = request.allowed();
        if !allowed.is_empty() && !allowed.contains(&hook) {
            return Err(syn::Error::new_spanned(
                ty,
                format!(
                    "{} is requested only by {}",
                    request.spoken(),
                    HookName::spoken_list(allowed)
                ),
            ));
        }
        Ok(request)
    }

    /// What a parameter carrying this attribute requests.
    fn marked(mark: &Attribute, pattern: &Pat, ty: &Type) -> syn::Result<Self> {
        match marker::name(mark).as_str() {
            "from_step" => Ok(Self::FromStep(
                Arguments::of(mark)?.datum(named(pattern), ty)?,
            )),
            "from_workflow" => Ok(Self::FromWorkflow(
                Arguments::of(mark)?.datum(named(pattern), ty)?,
            )),
            _ => {
                mark.meta.require_path_only()?;
                Ok(Self::Role(referent(ty)?.clone()))
            }
        }
    }

    /// The handle an unmarked parameter takes, as its type says.
    fn handle(parameter: &PatType) -> syn::Result<Self> {
        match Handle::of(&parameter.ty) {
            Some(Handle::Contributor) => Ok(Self::Contributor),
            Some(Handle::Reporter) => Ok(Self::Reporter),
            Some(Handle::JourneyId) => Ok(Self::JourneyId),
            Some(Handle::StepName) => Ok(Self::StepName),
            Some(Handle::Attempt) => Ok(Self::Attempt),
            Some(Handle::Reason { optional }) => Ok(Self::Reason { optional }),
            Some(Handle::Error { optional }) => Ok(Self::Error { optional }),
            Some(Handle::Cause) => Ok(Self::Cause),
            Some(Handle::DataBag) => Err(syn::Error::new_spanned(
                &parameter.ty,
                "the data bag is requested only by an input adapter",
            )),
            None => Err(syn::Error::new_spanned(
                &parameter.pat,
                "every parameter of a hook after `&self` is a handle, such as `&mut Contributor` \
                 or `Attempt`, or says what it requests, such as `#[from_step(\"receipt\")]` or \
                 `#[role]`",
            )),
        }
    }

    /// The hooks that may make the request, or none when every hook may.
    fn allowed(&self) -> &'static [HookName] {
        match self {
            Self::FromStep(_) | Self::StepName | Self::Attempt => &STEP_HOOKS,
            Self::Reason { .. } | Self::Cause => &FAILURE_HOOKS,
            Self::Error { .. } => &ERROR_HOOKS,
            Self::FromWorkflow(_)
            | Self::JourneyId
            | Self::Role(_)
            | Self::Contributor
            | Self::Reporter => &[],
        }
    }

    /// What the request is called where an error names it.
    fn spoken(&self) -> &'static str {
        match self {
            Self::FromStep(_) => "`#[from_step]`",
            Self::FromWorkflow(_) => "`#[from_workflow]`",
            Self::StepName => "the step's name",
            Self::Attempt => "the attempt",
            Self::Reason { .. } => "the reason",
            Self::Cause => "the cause",
            Self::Error { .. } => "the error",
            Self::JourneyId => "the journey ID",
            Self::Role(_) => "a role",
            Self::Contributor => "a contributor",
            Self::Reporter => "a reporter",
        }
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
