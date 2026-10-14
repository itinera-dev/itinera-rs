//! Reading what a step hook returned.

use super::{FailWorkflow, OnSuccess};
use crate::policy::kind::sealed::Sealed;
use crate::step::Reason;

/// Reads what a step hook returned: a lifecycle, or nothing.
///
/// A test that calls a step hook itself reads its answer with these methods, rather than matching it.
/// Only what step hooks return implements it: `Option<OnSuccess>` and `Option<FailWorkflow>`.
///
/// # Examples
///
/// ```
/// use itinera::policy::{FailWorkflow, OnSuccess, ReturnedLifecycle};
///
/// let after_success = Some(OnSuccess::FinishWorkflow);
/// assert!(after_success.finishes_workflow());
///
/// let after_failure: Option<FailWorkflow> = None;
/// assert!(after_failure.keeps_default());
/// ```
pub trait ReturnedLifecycle: Sealed {
    /// Whether the hook returned nothing, so the executor does what it does by default.
    ///
    /// That depends on the hook: after `on step retry` the step is tried again, but after
    /// `on step failure` the journey fails with the step's own failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{FailWorkflow, ReturnedLifecycle};
    /// use itinera::step::Reason;
    ///
    /// let nothing: Option<FailWorkflow> = None;
    /// assert!(nothing.keeps_default());
    /// assert!(!Some(FailWorkflow::from(Reason::new("fraud"))).keeps_default());
    /// ```
    #[must_use]
    fn keeps_default(&self) -> bool;

    /// The reason the journey fails with, when the hook returned `FailWorkflow`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{OnSuccess, ReturnedLifecycle};
    /// use itinera::step::Reason;
    ///
    /// let refused = Some(OnSuccess::FailWorkflow(Reason::new("refused")));
    /// assert_eq!(refused.fails_workflow().map(Reason::code), Some("refused"));
    /// assert!(Some(OnSuccess::FinishWorkflow).fails_workflow().is_none());
    /// ```
    #[must_use]
    fn fails_workflow(&self) -> Option<&Reason>;

    /// Whether the hook returned `FinishWorkflow`, which only `on step success` may return.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{OnSuccess, ReturnedLifecycle};
    ///
    /// assert!(Some(OnSuccess::FinishWorkflow).finishes_workflow());
    /// assert!(!None::<OnSuccess>.finishes_workflow());
    /// ```
    #[must_use]
    fn finishes_workflow(&self) -> bool;
}

impl Sealed for Option<OnSuccess> {}

impl ReturnedLifecycle for Option<OnSuccess> {
    fn keeps_default(&self) -> bool {
        self.is_none()
    }

    fn fails_workflow(&self) -> Option<&Reason> {
        match self {
            Some(OnSuccess::FailWorkflow(reason)) => Some(reason),
            Some(OnSuccess::FinishWorkflow) | None => None,
        }
    }

    fn finishes_workflow(&self) -> bool {
        matches!(self, Some(OnSuccess::FinishWorkflow))
    }
}

impl Sealed for Option<FailWorkflow> {}

impl ReturnedLifecycle for Option<FailWorkflow> {
    fn keeps_default(&self) -> bool {
        self.is_none()
    }

    fn fails_workflow(&self) -> Option<&Reason> {
        self.as_ref().map(FailWorkflow::reason)
    }

    fn finishes_workflow(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::nothing_after_success(&None::<OnSuccess>, true)]
    #[case::nothing_after_another_step_hook(&None::<FailWorkflow>, true)]
    #[case::finish_workflow(&Some(OnSuccess::FinishWorkflow), false)]
    #[case::fail_workflow_after_success(&Some(OnSuccess::FailWorkflow(Reason::new("fraud"))), false)]
    #[case::fail_workflow_after_another_step_hook(&Some(FailWorkflow::from(Reason::new("fraud"))), false)]
    fn only_returning_nothing_keeps_the_default(
        #[case] returned: &dyn ReturnedLifecycle,
        #[case] keeps: bool,
    ) {
        assert_eq!(returned.keeps_default(), keeps);
    }

    #[rstest]
    #[case::nothing_after_success(&None::<OnSuccess>, None)]
    #[case::nothing_after_another_step_hook(&None::<FailWorkflow>, None)]
    #[case::finish_workflow(&Some(OnSuccess::FinishWorkflow), None)]
    #[case::fail_workflow_after_success(
        &Some(OnSuccess::FailWorkflow(Reason::new("fraud"))),
        Some("fraud")
    )]
    #[case::fail_workflow_after_another_step_hook(&Some(FailWorkflow::from(Reason::new("fraud"))), Some("fraud"))]
    fn failing_the_workflow_carries_its_reason(
        #[case] returned: &dyn ReturnedLifecycle,
        #[case] code: Option<&str>,
    ) {
        assert_eq!(returned.fails_workflow().map(Reason::code), code);
    }

    #[rstest]
    #[case::nothing_after_success(&None::<OnSuccess>, false)]
    #[case::nothing_after_another_step_hook(&None::<FailWorkflow>, false)]
    #[case::finish_workflow(&Some(OnSuccess::FinishWorkflow), true)]
    #[case::fail_workflow_after_success(&Some(OnSuccess::FailWorkflow(Reason::new("fraud"))), false)]
    #[case::fail_workflow_after_another_step_hook(&Some(FailWorkflow::from(Reason::new("fraud"))), false)]
    fn only_finish_workflow_finishes_the_workflow(
        #[case] returned: &dyn ReturnedLifecycle,
        #[case] finishes: bool,
    ) {
        assert_eq!(returned.finishes_workflow(), finishes);
    }
}
