//! Which hook decided, and why a step will not be attempted again.

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
    /// The attempt reported a failure that is not retriable.
    #[display("failure")]
    Failure,
    /// The attempt ended in an abnormal termination, and the step does not allow retrying it.
    #[display("abnormal termination")]
    AbnormalTermination,
    /// The retry budget is spent.
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
