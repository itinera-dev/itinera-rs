//! What `on step failure`, `on step retry` and `on step abnormal termination` return.

use super::Lifecycle;
use crate::step::Reason;

/// What `on step failure`, `on step retry` and `on step abnormal termination` return: keep the
/// default, or fail the journey at once.
///
/// The default differs by hook: after `on step failure` the journey fails with the step's own
/// failure, and after the two others the step is tried again or given up. A policy declared with
/// `#[step_policy]` returns it with the shorthands `Self::keep_default()` and
/// `Self::fail_workflow(..)`.
///
/// # Examples
///
/// ```
/// use itinera::policy::OnFailure;
///
/// fn after_a_decline(declines: u32) -> OnFailure {
///     if declines > 3 {
///         OnFailure::fail_workflow("fraud-suspected")
///     } else {
///         OnFailure::KeepDefault
///     }
/// }
///
/// assert!(after_a_decline(1).keeps_default());
/// ```
#[derive(Clone, Debug, Default, derive_more::From)]
#[non_exhaustive]
pub enum OnFailure {
    /// No lifecycle: the hook's default happens.
    #[default]
    KeepDefault,
    /// The journey fails at once, with this reason.
    #[from]
    FailWorkflow(Reason),
}

impl OnFailure {
    /// The journey fails at once, for this reason. A text is the reason's code.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::OnFailure;
    ///
    /// let blocked = OnFailure::fail_workflow("card-blocked");
    /// assert_eq!(blocked.fails_workflow().map(|reason| reason.code()), Some("card-blocked"));
    /// ```
    pub fn fail_workflow(reason: impl Into<Reason>) -> Self {
        Self::FailWorkflow(reason.into())
    }

    /// Whether it keeps the default, deciding nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::OnFailure;
    ///
    /// assert!(OnFailure::KeepDefault.keeps_default());
    /// assert!(!OnFailure::fail_workflow("fraud").keeps_default());
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
    /// use itinera::policy::OnFailure;
    ///
    /// let blocked = OnFailure::fail_workflow("card-blocked");
    /// assert_eq!(blocked.fails_workflow().map(|reason| reason.code()), Some("card-blocked"));
    /// assert!(OnFailure::KeepDefault.fails_workflow().is_none());
    /// ```
    #[must_use]
    pub fn fails_workflow(&self) -> Option<&Reason> {
        match self {
            Self::FailWorkflow(reason) => Some(reason),
            Self::KeepDefault => None,
        }
    }

    /// The reason the journey fails with, `None` when it keeps the default.
    pub(crate) fn failing(self) -> Option<Reason> {
        match self {
            Self::KeepDefault => None,
            Self::FailWorkflow(reason) => Some(reason),
        }
    }

    /// The lifecycle it returns, `None` when it keeps the default.
    pub(crate) fn lifecycle(self) -> Option<Lifecycle> {
        self.failing().map(Lifecycle::FailWorkflow)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::keep_default(OnFailure::KeepDefault, true)]
    #[case::fail_workflow(OnFailure::fail_workflow("fraud"), false)]
    fn only_keep_default_keeps_the_default(#[case] returned: OnFailure, #[case] keeps: bool) {
        assert_eq!(returned.keeps_default(), keeps);
    }

    #[rstest]
    #[case::keep_default(OnFailure::KeepDefault, None)]
    #[case::fail_workflow(OnFailure::fail_workflow("fraud"), Some("fraud"))]
    fn failing_the_workflow_carries_its_reason(
        #[case] returned: OnFailure,
        #[case] code: Option<&str>,
    ) {
        assert_eq!(returned.fails_workflow().map(Reason::code), code);
    }

    #[rstest]
    #[case::keep_default(OnFailure::KeepDefault, None)]
    #[case::fail_workflow(OnFailure::fail_workflow("fraud"), Some("FailWorkflow"))]
    fn only_a_lifecycle_is_recorded(#[case] returned: OnFailure, #[case] recorded: Option<&str>) {
        let lifecycle = returned.lifecycle().map(|lifecycle| lifecycle.to_string());
        assert_eq!(lifecycle.as_deref(), recorded);
    }
}
