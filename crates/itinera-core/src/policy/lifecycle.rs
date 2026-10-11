//! What a hook returns: the lifecycle that decides what happens next.

use crate::step::Reason;

/// A lifecycle a hook returned, which decides what happens next.
///
/// It displays as the lifecycle is named, for example `FailWorkflow`.
///
/// # Examples
///
/// ```
/// use itinera::policy::Lifecycle;
///
/// fn ends_the_journey(lifecycle: &Lifecycle) -> bool {
///     matches!(lifecycle, Lifecycle::FinishWorkflow | Lifecycle::FailWorkflow(_))
/// }
///
/// assert_eq!(Lifecycle::FinishWorkflow.to_string(), "FinishWorkflow");
/// ```
#[derive(Clone, Debug, derive_more::Display)]
#[non_exhaustive]
pub enum Lifecycle {
    /// The journey succeeds at once.
    FinishWorkflow,
    /// The journey fails at once, with this reason.
    #[display("FailWorkflow")]
    FailWorkflow(Reason),
}

/// What `on step success` may return: a lifecycle that ends the journey at once.
///
/// Returning `None` instead goes on to the next step, or succeeds the journey after the last.
///
/// # Examples
///
/// ```
/// use itinera::policy::OnSuccess;
/// use itinera::step::Reason;
///
/// fn ends_without_the_steps_left(returned: &OnSuccess) -> bool {
///     matches!(returned, OnSuccess::FinishWorkflow)
/// }
///
/// assert!(ends_without_the_steps_left(&OnSuccess::FinishWorkflow));
/// assert!(!ends_without_the_steps_left(&OnSuccess::FailWorkflow(Reason::new("refused"))));
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum OnSuccess {
    /// The journey succeeds at once, and the steps left are not run.
    FinishWorkflow,
    /// The journey fails at once, with this reason. The step stays succeeded.
    FailWorkflow(Reason),
}

/// The lifecycle `on step failure`, `on step retry` and `on step abnormal termination` may
/// return: the journey fails at once, with this reason.
///
/// Returning `None` instead keeps the default: the journey fails with the step's own failure
/// after `on step failure`, and the step is tried again or given up after the two others.
///
/// # Examples
///
/// ```
/// use itinera::policy::FailWorkflow;
/// use itinera::step::Reason;
///
/// let fail = FailWorkflow::from(Reason::new("fraud").with_message("the card is blocked"));
/// let reason: Reason = fail.into();
/// assert_eq!(reason.code(), "fraud");
/// ```
#[derive(Clone, Debug, derive_more::From, derive_more::Into)]
pub struct FailWorkflow(Reason);

impl From<OnSuccess> for Lifecycle {
    fn from(returned: OnSuccess) -> Self {
        match returned {
            OnSuccess::FinishWorkflow => Self::FinishWorkflow,
            OnSuccess::FailWorkflow(reason) => Self::FailWorkflow(reason),
        }
    }
}

impl From<FailWorkflow> for Lifecycle {
    fn from(returned: FailWorkflow) -> Self {
        Self::FailWorkflow(returned.into())
    }
}
