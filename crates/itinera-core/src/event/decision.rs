//! Which hook decided, why a step will not be attempted again, and why it is rescued.

use crate::policy::{PolicyName, StepHook};
use crate::step::Reason;

/// The policy and step hook whose returned lifecycle decided something.
///
/// Only step hooks return lifecycles, so only they decide. `H` is the set of hooks that can take
/// the decision: any step hook by default, or a narrower set such as [`GiveUpHook`].
///
/// # Examples
///
/// ```
/// use itinera::event::DecidingHook;
///
/// fn describe(hook: &DecidingHook) -> String {
///     format!("{}, {}", hook.policy, hook.hook)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct DecidingHook<H = StepHook> {
    /// The policy's name.
    pub policy: PolicyName,
    /// The hook.
    pub hook: H,
}

/// A step hook that gives a step up when it returns `FailWorkflow`.
///
/// It displays as the hook is named, for example `on step retry`.
///
/// # Examples
///
/// ```
/// use itinera::event::GiveUpHook;
///
/// assert_eq!(GiveUpHook::OnStepRetry.to_string(), "on step retry");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum GiveUpHook {
    /// Called before a step is attempted again.
    #[display("on step retry")]
    OnStepRetry,
    /// Called after an attempt ended in an abnormal termination.
    #[display("on step abnormal termination")]
    OnStepAbnormalTermination,
}

impl GiveUpHook {
    /// The step hook this is.
    pub(crate) fn step_hook(self) -> StepHook {
        match self {
            Self::OnStepRetry => StepHook::OnStepRetry,
            Self::OnStepAbnormalTermination => StepHook::OnStepAbnormalTermination,
        }
    }
}

/// Why a step will not be attempted again.
///
/// It displays as the cause is named, for example `retries exhausted`. Every cause but
/// `FailWorkflow` is decided by default.
///
/// # Examples
///
/// ```
/// use itinera::event::GiveUpCause;
///
/// assert_eq!(GiveUpCause::RetriesExhausted.to_string(), "retries exhausted");
/// ```
#[derive(Clone, Debug, derive_more::Display)]
#[non_exhaustive]
pub enum GiveUpCause {
    /// The attempt reported a failure, and the step has no retry budget.
    #[display("failure")]
    Failure,
    /// The attempt ended in an abnormal termination, and the step has no retry budget or does
    /// not retry abnormal terminations.
    #[display("abnormal termination")]
    AbnormalTermination,
    /// The step's retry budget, greater than 0, is spent.
    #[display("retries exhausted")]
    RetriesExhausted,
    /// A hook returned `FailWorkflow`.
    #[display("FailWorkflow")]
    #[non_exhaustive]
    FailWorkflow {
        /// The hook that returned it.
        decided_by: DecidingHook<GiveUpHook>,
        /// The reason it carried.
        reason: Reason,
    },
}

/// How the attempt of a step that is rescued failed.
///
/// It displays as the cause is named, for example `abnormal termination`.
///
/// # Examples
///
/// ```
/// use itinera::event::RescueCause;
///
/// assert_eq!(RescueCause::AbnormalTermination.to_string(), "abnormal termination");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[display(rename_all = "lowercase")]
#[non_exhaustive]
pub enum RescueCause {
    /// The attempt reported a failure.
    Failure,
    /// The attempt ended in an abnormal termination.
    #[display("abnormal termination")]
    AbnormalTermination,
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::give_up_on_step_retry(&GiveUpHook::OnStepRetry, "on step retry")]
    #[case::give_up_on_step_abnormal_termination(
        &GiveUpHook::OnStepAbnormalTermination,
        "on step abnormal termination"
    )]
    #[case::given_up_after_a_failure(&GiveUpCause::Failure, "failure")]
    #[case::given_up_after_an_abnormal_termination(
        &GiveUpCause::AbnormalTermination,
        "abnormal termination"
    )]
    #[case::given_up_with_retries_exhausted(&GiveUpCause::RetriesExhausted, "retries exhausted")]
    #[case::rescued_after_a_failure(&RescueCause::Failure, "failure")]
    #[case::rescued_after_an_abnormal_termination(
        &RescueCause::AbnormalTermination,
        "abnormal termination"
    )]
    fn names_display_as_the_specification_writes_them(
        #[case] name: &dyn fmt::Display,
        #[case] written: &str,
    ) {
        assert_eq!(name.to_string(), written);
    }

    #[test]
    fn a_step_given_up_by_fail_workflow_names_fail_workflow_as_its_cause() {
        let cause = GiveUpCause::FailWorkflow {
            decided_by: DecidingHook {
                policy: PolicyName::from("close"),
                hook: GiveUpHook::OnStepRetry,
            },
            reason: Reason::new("fraud"),
        };
        assert_eq!(cause.to_string(), "FailWorkflow");
    }
}
