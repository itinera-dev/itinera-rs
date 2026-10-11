//! Who made a request whose data could not be resolved, as events tell it.

use crate::policy::{PolicyName, StepHook, WorkflowHook};
use crate::step::StepName;
use crate::workflow::AdapterName;

/// Who made a request whose data could not be resolved, and the step it was made for: a step for
/// one of its inputs, an input adapter resolving a step's input, a step hook, or a workflow hook,
/// which has no step.
///
/// # Examples
///
/// ```
/// use itinera::event::Requester;
/// use itinera::workflow::AdapterName;
///
/// fn adapter(requester: &Requester) -> Option<AdapterName> {
///     match requester {
///         Requester::Adapter { adapter, .. } => Some(*adapter),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Requester {
    /// A step, for one of its inputs.
    #[non_exhaustive]
    Step {
        /// The step's name.
        step: StepName,
    },
    /// An input adapter, for an input of a step.
    #[non_exhaustive]
    Adapter {
        /// The adapter's name.
        adapter: AdapterName,
        /// The name of the step whose input it was resolving.
        step: StepName,
    },
    /// A step hook.
    #[non_exhaustive]
    StepHook {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: StepHook,
        /// The name of the step it acts on.
        step: StepName,
    },
    /// A workflow hook.
    #[non_exhaustive]
    WorkflowHook {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: WorkflowHook,
    },
}
impl Requester {
    pub(super) fn step(&self) -> Option<StepName> {
        match self {
            Self::Step { step } | Self::Adapter { step, .. } | Self::StepHook { step, .. } => {
                Some(*step)
            }
            Self::WorkflowHook { .. } => None,
        }
    }
}

/// Who requested what a failed attempt left, the failure's reason or the error, and the step it
/// was for: a step hook, for the step it acts on, or a rescue step, for the step it rescues.
///
/// # Examples
///
/// ```
/// use itinera::event::FailureRequester;
/// use itinera::step::StepName;
///
/// fn rescue_step(requester: &FailureRequester) -> Option<StepName> {
///     match requester {
///         FailureRequester::RescueStep { step, .. } => Some(*step),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FailureRequester {
    /// A step hook.
    #[non_exhaustive]
    StepHook {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: StepHook,
        /// The name of the step it acts on.
        step: StepName,
    },
    /// A rescue step.
    #[non_exhaustive]
    RescueStep {
        /// The rescue step's name.
        step: StepName,
    },
}

impl FailureRequester {
    pub(super) fn step(&self) -> StepName {
        match self {
            Self::StepHook { step, .. } | Self::RescueStep { step } => *step,
        }
    }
}
