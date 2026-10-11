//! The name of a hook, and the names the generated code derives from it.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::policy::Kind;

/// A hook a policy may define. Its marker, such as `on_step_success`, is also the name of the
/// hook trait's method and of the descriptor's method that defines it.
#[derive(Clone, Copy, PartialEq)]
#[expect(
    clippy::enum_variant_names,
    reason = "the variants are the hooks' names, which all start with \"on\""
)]
pub(crate) enum HookName {
    OnStepSuccess,
    OnStepFailure,
    OnStepRetry,
    OnStepAbnormalTermination,
    OnWorkflowSuccess,
    OnWorkflowFailure,
}

impl HookName {
    pub(crate) const ALL: [Self; 6] = [
        Self::OnStepSuccess,
        Self::OnStepFailure,
        Self::OnStepRetry,
        Self::OnStepAbnormalTermination,
        Self::OnWorkflowSuccess,
        Self::OnWorkflowFailure,
    ];

    /// The hook a marker names, if it names one.
    pub(crate) fn of(marker: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|hook| hook.is_marked_by(marker))
    }

    fn is_marked_by(self, marker: &str) -> bool {
        self.marker() == marker
    }

    /// The attribute that marks the hook, without its `#`, such as `on_step_success`.
    pub(crate) const fn marker(self) -> &'static str {
        match self {
            Self::OnStepSuccess => "on_step_success",
            Self::OnStepFailure => "on_step_failure",
            Self::OnStepRetry => "on_step_retry",
            Self::OnStepAbnormalTermination => "on_step_abnormal_termination",
            Self::OnWorkflowSuccess => "on_workflow_success",
            Self::OnWorkflowFailure => "on_workflow_failure",
        }
    }

    /// Whether the hook belongs to step policies or to workflow policies.
    pub(crate) fn kind(self) -> Kind {
        match self {
            Self::OnStepSuccess
            | Self::OnStepFailure
            | Self::OnStepRetry
            | Self::OnStepAbnormalTermination => Kind::Step,
            Self::OnWorkflowSuccess | Self::OnWorkflowFailure => Kind::Workflow,
        }
    }

    /// The hook as the specification writes it, in backticks, such as `` `on step success` ``.
    pub(crate) fn spoken(self) -> String {
        format!("`{}`", self.marker().replace('_', " "))
    }

    /// Hooks as the specification writes them, such as `` `on step failure` and `on step retry` ``.
    pub(crate) fn spoken_list(hooks: &[Self]) -> String {
        let spoken: Vec<String> = hooks.iter().map(|hook| hook.spoken()).collect();
        match spoken.split_last() {
            Some((last, [])) => last.clone(),
            Some((last, others)) => format!("{} and {last}", others.join(", ")),
            None => String::new(),
        }
    }

    /// The hook trait's method, such as `on_step_success`.
    pub(crate) fn method(self) -> syn::Ident {
        format_ident!("{}", self.marker())
    }

    /// The name of the hook's kind, such as `StepSuccess`.
    fn kind_name(self) -> &'static str {
        match self {
            Self::OnStepSuccess => "StepSuccess",
            Self::OnStepFailure => "StepFailure",
            Self::OnStepRetry => "StepRetry",
            Self::OnStepAbnormalTermination => "StepAbnormalTermination",
            Self::OnWorkflowSuccess => "WorkflowSuccess",
            Self::OnWorkflowFailure => "WorkflowFailure",
        }
    }

    /// The hook's kind, such as `StepSuccess`.
    pub(crate) fn hook_kind(self) -> syn::Ident {
        format_ident!("{}", self.kind_name())
    }

    /// The hook's name in upper camel case, such as `OnStepSuccess`, which names its trait and its
    /// variant of `StepHook` or `WorkflowHook`.
    pub(crate) fn variant(self) -> syn::Ident {
        format_ident!("On{}", self.kind_name())
    }

    /// What the hook returns when it succeeds.
    pub(crate) fn returned(self) -> TokenStream {
        match self {
            Self::OnStepSuccess => quote!(::core::option::Option<::itinera::policy::OnSuccess>),
            Self::OnStepFailure | Self::OnStepRetry | Self::OnStepAbnormalTermination => {
                quote!(::core::option::Option<::itinera::policy::FailWorkflow>)
            }
            Self::OnWorkflowSuccess | Self::OnWorkflowFailure => quote!(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::one(&[HookName::OnStepSuccess], "`on step success`")]
    #[case::two(
        &[HookName::OnStepFailure, HookName::OnStepRetry],
        "`on step failure` and `on step retry`"
    )]
    #[case::three(
        &[HookName::OnStepSuccess, HookName::OnStepFailure, HookName::OnStepRetry],
        "`on step success`, `on step failure` and `on step retry`"
    )]
    fn hooks_are_listed_as_a_sentence(#[case] hooks: &[HookName], #[case] spoken: &str) {
        assert_eq!(HookName::spoken_list(hooks), spoken);
    }

    #[test]
    fn every_hook_is_named_by_its_own_marker() {
        for hook in HookName::ALL {
            assert!(HookName::of(hook.marker()) == Some(hook));
        }
    }
}
