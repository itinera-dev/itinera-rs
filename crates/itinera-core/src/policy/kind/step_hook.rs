//! The kinds of the four step hooks.

use super::{ErrorHookKind, FailureHookKind, HookKind, PolicyHookKind, StepHookKind, sealed};
use crate::policy::{RetryCause, StepFailureCause};
use crate::step::StepAttempt;

/// `on step success`, called after an attempt that succeeded.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, StepSuccess};
///
/// let needs = HookNeeds::<StepSuccess>::new().reporter();
/// ```
#[derive(Clone, Copy, Debug)]
pub enum StepSuccess {}

/// `on step failure`, called after a step was given up.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, StepFailure};
///
/// let needs = HookNeeds::<StepFailure>::new().reason();
/// ```
#[derive(Clone, Copy, Debug)]
pub enum StepFailure {}

/// `on step retry`, called before a step is attempted again.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, StepRetry};
///
/// let needs = HookNeeds::<StepRetry>::new().optional_error();
/// ```
#[derive(Clone, Copy, Debug)]
pub enum StepRetry {}

/// `on step abnormal termination`, called after an attempt ended in an abnormal termination.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, StepAbnormalTermination};
///
/// let needs = HookNeeds::<StepAbnormalTermination>::new().error();
/// ```
#[derive(Clone, Copy, Debug)]
pub enum StepAbnormalTermination {}

impl sealed::Sealed for StepSuccess {}
impl sealed::Sealed for StepFailure {}
impl sealed::Sealed for StepRetry {}
impl sealed::Sealed for StepAbnormalTermination {}

impl HookKind for StepSuccess {
    type Context = StepAttempt;
}

impl HookKind for StepFailure {
    type Context = (StepAttempt, StepFailureCause);
}

impl HookKind for StepRetry {
    type Context = (StepAttempt, RetryCause);
}

impl HookKind for StepAbnormalTermination {
    type Context = StepAttempt;
}

impl StepHookKind for StepSuccess {
    fn attempt(context: &StepAttempt) -> &StepAttempt {
        context
    }
}

impl StepHookKind for StepFailure {
    fn attempt((attempt, _): &(StepAttempt, StepFailureCause)) -> &StepAttempt {
        attempt
    }
}

impl StepHookKind for StepRetry {
    fn attempt((attempt, _): &(StepAttempt, RetryCause)) -> &StepAttempt {
        attempt
    }
}

impl StepHookKind for StepAbnormalTermination {
    fn attempt(context: &StepAttempt) -> &StepAttempt {
        context
    }
}

impl FailureHookKind for StepFailure {}
impl FailureHookKind for StepRetry {}

impl ErrorHookKind for StepFailure {}
impl ErrorHookKind for StepRetry {}
impl ErrorHookKind for StepAbnormalTermination {}

impl PolicyHookKind for StepSuccess {}
impl PolicyHookKind for StepFailure {}
impl PolicyHookKind for StepRetry {}
impl PolicyHookKind for StepAbnormalTermination {}
