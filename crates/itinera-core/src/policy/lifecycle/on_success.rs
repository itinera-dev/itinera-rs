//! What `on step success` returns.

use super::Lifecycle;
use crate::step::Reason;

/// What `on step success` returns: keep the default, or a lifecycle that ends the journey at once.
///
/// A policy declared with `#[step_policy]` returns it with the shorthands `Self::keep_default()`,
/// `Self::finish_workflow()` and `Self::fail_workflow(..)`.
///
/// # Examples
///
/// ```
/// use itinera::policy::OnSuccess;
///
/// fn after_a_charge(charged: i64) -> OnSuccess {
///     if charged > 10_000 {
///         OnSuccess::fail_workflow("needs-approval")
///     } else {
///         OnSuccess::KeepDefault
///     }
/// }
///
/// assert!(after_a_charge(40).keeps_default());
/// ```
#[derive(Clone, Debug, Default, derive_more::From)]
#[non_exhaustive]
pub enum OnSuccess {
    /// No lifecycle: the journey goes on to the next step, or succeeds after the last.
    #[default]
    KeepDefault,
    /// The journey succeeds at once, and the steps left are not run.
    FinishWorkflow,
    /// The journey fails at once, with this reason. The step stays succeeded.
    #[from]
    FailWorkflow(Reason),
}

impl OnSuccess {
    /// The journey fails at once, for this reason. A text is the reason's code.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::OnSuccess;
    ///
    /// let refused = OnSuccess::fail_workflow("fraud-suspected");
    /// assert_eq!(refused.fails_workflow().map(|reason| reason.code()), Some("fraud-suspected"));
    /// ```
    pub fn fail_workflow(reason: impl Into<Reason>) -> Self {
        Self::FailWorkflow(reason.into())
    }

    /// Whether it keeps the default, deciding nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::OnSuccess;
    ///
    /// assert!(OnSuccess::KeepDefault.keeps_default());
    /// assert!(!OnSuccess::FinishWorkflow.keeps_default());
    /// ```
    #[must_use]
    pub fn keeps_default(&self) -> bool {
        matches!(self, Self::KeepDefault)
    }

    /// The reason it fails the workflow with, if it returns `FailWorkflow`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::OnSuccess;
    ///
    /// let refused = OnSuccess::fail_workflow("fraud");
    /// assert_eq!(refused.fails_workflow().map(|reason| reason.code()), Some("fraud"));
    /// assert!(OnSuccess::FinishWorkflow.fails_workflow().is_none());
    /// ```
    #[must_use]
    pub fn fails_workflow(&self) -> Option<&Reason> {
        match self {
            Self::FailWorkflow(reason) => Some(reason),
            Self::KeepDefault | Self::FinishWorkflow => None,
        }
    }

    /// Whether it finishes the workflow at once.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::OnSuccess;
    ///
    /// assert!(OnSuccess::FinishWorkflow.finishes_workflow());
    /// assert!(!OnSuccess::KeepDefault.finishes_workflow());
    /// ```
    #[must_use]
    pub fn finishes_workflow(&self) -> bool {
        matches!(self, Self::FinishWorkflow)
    }

    /// The lifecycle it returns, `None` when it keeps the default.
    pub(crate) fn lifecycle(self) -> Option<Lifecycle> {
        match self {
            Self::KeepDefault => None,
            Self::FinishWorkflow => Some(Lifecycle::FinishWorkflow),
            Self::FailWorkflow(reason) => Some(Lifecycle::FailWorkflow(reason)),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::keep_default(OnSuccess::KeepDefault, true)]
    #[case::finish_workflow(OnSuccess::FinishWorkflow, false)]
    #[case::fail_workflow(OnSuccess::fail_workflow("fraud"), false)]
    fn only_keep_default_keeps_the_default(#[case] returned: OnSuccess, #[case] keeps: bool) {
        assert_eq!(returned.keeps_default(), keeps);
    }

    #[rstest]
    #[case::keep_default(OnSuccess::KeepDefault, None)]
    #[case::finish_workflow(OnSuccess::FinishWorkflow, None)]
    #[case::fail_workflow(OnSuccess::fail_workflow("fraud"), Some("fraud"))]
    fn failing_the_workflow_carries_its_reason(
        #[case] returned: OnSuccess,
        #[case] code: Option<&str>,
    ) {
        assert_eq!(returned.fails_workflow().map(Reason::code), code);
    }

    #[rstest]
    #[case::keep_default(OnSuccess::KeepDefault, false)]
    #[case::finish_workflow(OnSuccess::FinishWorkflow, true)]
    #[case::fail_workflow(OnSuccess::fail_workflow("fraud"), false)]
    fn only_finish_workflow_finishes_the_workflow(
        #[case] returned: OnSuccess,
        #[case] finishes: bool,
    ) {
        assert_eq!(returned.finishes_workflow(), finishes);
    }

    #[rstest]
    #[case::keep_default(OnSuccess::KeepDefault, None)]
    #[case::finish_workflow(OnSuccess::FinishWorkflow, Some("FinishWorkflow"))]
    #[case::fail_workflow(OnSuccess::fail_workflow("fraud"), Some("FailWorkflow"))]
    fn only_a_lifecycle_is_recorded(#[case] returned: OnSuccess, #[case] recorded: Option<&str>) {
        let lifecycle = returned.lifecycle().map(|lifecycle| lifecycle.to_string());
        assert_eq!(lifecycle.as_deref(), recorded);
    }
}
