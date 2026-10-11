//! The kind of policy a macro describes, and the names that kind uses.

use quote::format_ident;

/// The kind of policy a macro describes.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    Step,
    Workflow,
}

impl Kind {
    /// The macro's attribute, such as `#[step_policy]`.
    pub(super) fn attribute(self) -> &'static str {
        match self {
            Self::Step => "#[step_policy]",
            Self::Workflow => "#[workflow_policy]",
        }
    }

    /// The trait through which a workflow attaches the policy, and its descriptor and hook types.
    pub(super) fn names(self) -> (syn::Ident, syn::Ident, syn::Ident) {
        match self {
            Self::Step => (
                format_ident!("StepPolicy"),
                format_ident!("StepPolicyDescriptor"),
                format_ident!("StepHook"),
            ),
            Self::Workflow => (
                format_ident!("WorkflowPolicy"),
                format_ident!("WorkflowPolicyDescriptor"),
                format_ident!("WorkflowHook"),
            ),
        }
    }
}
