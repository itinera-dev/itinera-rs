//! Who did what an event records: the hook that was called, who made an optional request, and
//! who made a contribution.

use crate::policy::{PolicyName, StepHook, WorkflowHook};
use crate::step::StepAttempt;
use crate::workflow::AdapterName;

/// A hook that was called: a step hook, with the step and attempt that triggered it, or a
/// workflow hook.
///
/// # Examples
///
/// ```
/// use itinera::event::HookSource;
/// use itinera::step::StepName;
///
/// fn triggered_by(source: &HookSource) -> Option<StepName> {
///     match source {
///         HookSource::Step { step, .. } => Some(step.step),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HookSource {
    /// A step hook.
    #[non_exhaustive]
    Step {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: StepHook,
        /// The step and attempt that triggered it.
        step: StepAttempt,
    },
    /// A workflow hook.
    #[non_exhaustive]
    Workflow {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: WorkflowHook,
    },
}

/// Who made an optional request that had no value: a step for one of its inputs, a hook, or an
/// input adapter resolving a step's input; with the step and attempt it was made for, except for
/// a workflow hook.
///
/// # Examples
///
/// ```
/// use itinera::event::RequestSource;
/// use itinera::workflow::AdapterName;
///
/// fn adapter(source: &RequestSource) -> Option<AdapterName> {
///     match source {
///         RequestSource::Adapter { adapter, .. } => Some(*adapter),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RequestSource {
    /// A step, for one of its inputs, during this attempt.
    Step(StepAttempt),
    /// A hook.
    Hook(HookSource),
    /// An input adapter, for an input of this step and attempt.
    #[non_exhaustive]
    Adapter {
        /// The adapter's name.
        adapter: AdapterName,
        /// The step and attempt whose input it was resolving.
        step: StepAttempt,
    },
}

/// Who made a contribution: a step, or a hook.
///
/// # Examples
///
/// ```
/// use itinera::event::Source;
///
/// fn from_a_hook(source: &Source) -> bool {
///     matches!(source, Source::Hook(_))
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Source {
    /// A step, during this attempt.
    Step(StepAttempt),
    /// A hook.
    Hook(HookSource),
}
