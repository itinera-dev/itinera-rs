//! Who made a request whose data could not be resolved, as the result tells it.

use crate::policy::{PolicyName, StepHook, WorkflowHook};
#[cfg(feature = "unstable")]
use crate::service::ProviderName;
use crate::workflow::AdapterName;

/// Who requested what a failed attempt left, the failure's reason or the error: a step hook, for
/// the step it acts on, or a rescue step, for the step it rescues.
///
/// It is the result's counterpart of
/// [`event::FailureRequester`](crate::event::FailureRequester), without the step's name, since
/// the result names no step.
///
/// # Examples
///
/// ```
/// use itinera::journey::FailureRequester;
///
/// fn by_a_rescue_step(requester: &FailureRequester) -> bool {
///     matches!(requester, FailureRequester::RescueStep)
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
    },
    /// A rescue step.
    RescueStep,
}

/// Who made a request whose data could not be resolved: a step for one of its inputs, an input
/// adapter resolving a step's input, a step hook, a workflow hook, or, with the `unstable`
/// feature, a service provider supplying a step's service.
///
/// It is the result's counterpart of [`event::Requester`](crate::event::Requester), without the
/// step's name, since the result names no step.
///
/// # Examples
///
/// ```
/// use itinera::journey::Requester;
/// use itinera::policy::PolicyName;
///
/// fn policy(requester: &Requester) -> Option<PolicyName> {
///     match requester {
///         Requester::StepHook { policy, .. } | Requester::WorkflowHook { policy, .. } => {
///             Some(*policy)
///         }
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Requester {
    /// A step, for one of its inputs.
    Step,
    /// An input adapter, for an input of a step.
    #[non_exhaustive]
    Adapter {
        /// The adapter's name.
        adapter: AdapterName,
    },
    /// A step hook.
    #[non_exhaustive]
    StepHook {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: StepHook,
    },
    /// A workflow hook.
    #[non_exhaustive]
    WorkflowHook {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: WorkflowHook,
    },
    /// A service provider, for a service of a step.
    #[cfg(feature = "unstable")]
    #[non_exhaustive]
    Provider {
        /// The provider's name.
        provider: ProviderName,
    },
}
