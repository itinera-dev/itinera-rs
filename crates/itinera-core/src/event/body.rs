//! What an event says, one variant per kind of event.

use super::{
    EventKind, GiveUpCause, HookSource, JourneyAbort, JourneyFailure, RequestSource, RescueCause,
    Source,
};
use crate::policy::{Lifecycle, PolicyName, RetryCause};
use crate::service::{Implementation, ProviderName};
use crate::step::{Reason, StepAttempt, StepName};
use crate::value::AnyValue;
use crate::workflow::AdapterName;

/// What an event says: one variant per kind of event.
///
/// # Examples
///
/// ```
/// use itinera::event::EventBody;
///
/// fn is_decision(body: &EventBody) -> bool {
///     matches!(
///         body,
///         EventBody::StepRetrying { .. }
///             | EventBody::StepGivenUp { .. }
///             | EventBody::StepRescuing { .. }
///             | EventBody::StepRescued { .. }
///             | EventBody::JourneySucceeded { .. }
///             | EventBody::JourneyFailed { .. }
///     )
/// }
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum EventBody {
    /// The journey started.
    #[non_exhaustive]
    JourneyStarted {
        /// The keys of the initial data, never their values.
        initial_keys: Vec<String>,
    },
    /// An attempt of a step started.
    #[non_exhaustive]
    AttemptStarted {
        /// The step and attempt.
        step: StepAttempt,
    },
    /// An input adapter supplied a value for a step's input.
    #[non_exhaustive]
    InputAdapterSupplied {
        /// The step and attempt.
        step: StepAttempt,
        /// The input's key.
        key: String,
        /// The adapter's name.
        adapter: AdapterName,
    },
    /// An input adapter failed for a step's input.
    #[non_exhaustive]
    InputAdapterFailed {
        /// The step and attempt.
        step: StepAttempt,
        /// The input's key.
        key: String,
        /// The adapter's name.
        adapter: AdapterName,
    },
    /// An optional request had no value.
    #[non_exhaustive]
    OptionalInputAbsent {
        /// The key that was requested.
        key: String,
        /// Who requested it.
        requester: RequestSource,
    },
    /// A service provider made an instance of a step's service.
    #[non_exhaustive]
    ServiceProvided {
        /// The step and attempt being built.
        step: StepAttempt,
        /// The service's key.
        key: String,
        /// The provider's name.
        provider: ProviderName,
        /// The implementation the instance is.
        implementation: Implementation,
        /// The data the provider supplied with the instance, if any.
        data: Option<AnyValue>,
    },
    /// A service provider failed for a step's service.
    #[non_exhaustive]
    ServiceProviderFailed {
        /// The step and attempt being built.
        step: StepAttempt,
        /// The service's key.
        key: String,
        /// The provider's name.
        provider: ProviderName,
    },
    /// An attempt reported Success.
    #[non_exhaustive]
    StepSucceeded {
        /// The step and attempt.
        step: StepAttempt,
    },
    /// An attempt reported Failure.
    #[non_exhaustive]
    StepFailed {
        /// The step and attempt.
        step: StepAttempt,
        /// Why the step failed.
        reason: Reason,
    },
    /// An attempt reported Skipped.
    #[non_exhaustive]
    StepSkipped {
        /// The step and attempt.
        step: StepAttempt,
        /// Why the step was skipped, if it said.
        reason: Option<Reason>,
    },
    /// An error escaped a running step.
    #[non_exhaustive]
    StepAbnormalTermination {
        /// The step and attempt.
        step: StepAttempt,
        /// The error's message.
        message: String,
    },
    /// A hook returned without failing.
    #[non_exhaustive]
    HookCalled {
        /// The hook, and the step and attempt that triggered a step hook.
        hook: HookSource,
        /// The lifecycle it returned, if any.
        lifecycle: Option<Lifecycle>,
    },
    /// A contribution reached the data bag.
    #[non_exhaustive]
    ContributionCommitted {
        /// The contribution's key.
        key: String,
        /// Who contributed it.
        source: Source,
    },
    /// A skipped step's contributions were discarded.
    #[non_exhaustive]
    ContributionsDiscarded {
        /// The step and attempt.
        step: StepAttempt,
    },
    /// A committed key replaced an earlier value.
    #[non_exhaustive]
    DataOverwritten {
        /// The key.
        key: String,
        /// Who contributed the new value.
        source: Source,
    },
    /// A step, hook or service provider set an entry of the journey's global metadata, after the
    /// reporters received it. The value is never carried: reporters already have it.
    #[non_exhaustive]
    GlobalMetadataSet {
        /// The entry's key.
        key: String,
        /// Who set it.
        source: Source,
    },
    /// The journey was aborted. This is always its last event.
    #[non_exhaustive]
    JourneyAborted {
        /// Why it was aborted, and what that reason carries.
        abort: JourneyAbort,
    },
    /// The executor decided to attempt a step again.
    #[non_exhaustive]
    StepRetrying {
        /// The step and the attempt that failed.
        step: StepAttempt,
        /// Why it will be attempted again. The executor's own rule always decides it.
        cause: RetryCause,
    },
    /// The executor decided not to attempt a step again: it ends as failed.
    #[non_exhaustive]
    StepGivenUp {
        /// The step and its last attempt.
        step: StepAttempt,
        /// Why it will not be attempted again, and who decided it.
        cause: GiveUpCause,
    },
    /// The executor decided to rescue a step: its rescue step runs, then the step is attempted
    /// again.
    #[non_exhaustive]
    StepRescuing {
        /// The step and the attempt that failed.
        step: StepAttempt,
        /// How that attempt failed. The executor's own rule always decides it.
        cause: RescueCause,
        /// The name of the step's rescue step.
        rescue: StepName,
    },
    /// A step's rescue step succeeded: the executor decided to attempt the step again. The
    /// executor's own rule always decides it.
    #[non_exhaustive]
    StepRescued {
        /// The step and the attempt that failed.
        step: StepAttempt,
        /// The name of the step's rescue step.
        rescue: StepName,
    },
    /// The executor decided that the journey succeeds.
    #[non_exhaustive]
    JourneySucceeded {
        /// The policy whose `on step success` returned `FinishWorkflow`, or `None` when no
        /// step was left and the journey succeeded by default.
        decided_by: Option<PolicyName>,
    },
    /// The executor decided that the journey fails.
    #[non_exhaustive]
    JourneyFailed {
        /// The name of the step that failed, or whose hook returned `FailWorkflow`.
        step: StepName,
        /// Why the journey failed, and who decided it.
        failure: JourneyFailure,
    },
    /// A step reported information.
    #[non_exhaustive]
    StepInfo {
        /// The step and attempt.
        step: StepAttempt,
        /// The step's message.
        message: String,
        /// The step's data, if any.
        data: Option<AnyValue>,
    },
    /// A step reported a warning.
    #[non_exhaustive]
    StepWarning {
        /// The step and attempt.
        step: StepAttempt,
        /// The step's message.
        message: String,
        /// The step's data, if any.
        data: Option<AnyValue>,
    },
    /// A step reported an error, which does not change its outcome.
    #[non_exhaustive]
    StepError {
        /// The step and attempt.
        step: StepAttempt,
        /// The step's message.
        message: String,
        /// The step's data, if any.
        data: Option<AnyValue>,
    },
    /// A hook reported information.
    #[non_exhaustive]
    JourneyInfo {
        /// The hook, and the step and attempt that triggered a step hook or that a service
        /// provider was building.
        hook: HookSource,
        /// The hook's message.
        message: String,
        /// The hook's data, if any.
        data: Option<AnyValue>,
    },
    /// A hook reported a warning.
    #[non_exhaustive]
    JourneyWarning {
        /// The hook, and the step and attempt that triggered a step hook or that a service
        /// provider was building.
        hook: HookSource,
        /// The hook's message.
        message: String,
        /// The hook's data, if any.
        data: Option<AnyValue>,
    },
    /// A hook reported an error, which does not change what it returns.
    #[non_exhaustive]
    JourneyError {
        /// The hook, and the step and attempt that triggered a step hook or that a service
        /// provider was building.
        hook: HookSource,
        /// The hook's message.
        message: String,
        /// The hook's data, if any.
        data: Option<AnyValue>,
    },
}
impl EventBody {
    /// The kind of event, which displays in snake_case, for example `step_failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::{EventBody, EventKind};
    ///
    /// fn is_emitted_by_a_step(body: &EventBody) -> bool {
    ///     matches!(
    ///         body.kind(),
    ///         EventKind::StepInfo | EventKind::StepWarning | EventKind::StepError
    ///     )
    /// }
    /// ```
    pub fn kind(&self) -> EventKind {
        match self {
            Self::JourneyStarted { .. } => EventKind::JourneyStarted,
            Self::AttemptStarted { .. } => EventKind::AttemptStarted,
            Self::InputAdapterSupplied { .. } => EventKind::InputAdapterSupplied,
            Self::InputAdapterFailed { .. } => EventKind::InputAdapterFailed,
            Self::OptionalInputAbsent { .. } => EventKind::OptionalInputAbsent,
            Self::ServiceProvided { .. } => EventKind::ServiceProvided,
            Self::ServiceProviderFailed { .. } => EventKind::ServiceProviderFailed,
            Self::StepSucceeded { .. } => EventKind::StepSucceeded,
            Self::StepFailed { .. } => EventKind::StepFailed,
            Self::StepSkipped { .. } => EventKind::StepSkipped,
            Self::StepAbnormalTermination { .. } => EventKind::StepAbnormalTermination,
            Self::HookCalled { .. } => EventKind::HookCalled,
            Self::ContributionCommitted { .. } => EventKind::ContributionCommitted,
            Self::ContributionsDiscarded { .. } => EventKind::ContributionsDiscarded,
            Self::DataOverwritten { .. } => EventKind::DataOverwritten,
            Self::GlobalMetadataSet { .. } => EventKind::GlobalMetadataSet,
            Self::JourneyAborted { .. } => EventKind::JourneyAborted,
            Self::StepRetrying { .. } => EventKind::StepRetrying,
            Self::StepGivenUp { .. } => EventKind::StepGivenUp,
            Self::StepRescuing { .. } => EventKind::StepRescuing,
            Self::StepRescued { .. } => EventKind::StepRescued,
            Self::JourneySucceeded { .. } => EventKind::JourneySucceeded,
            Self::JourneyFailed { .. } => EventKind::JourneyFailed,
            Self::StepInfo { .. } => EventKind::StepInfo,
            Self::StepWarning { .. } => EventKind::StepWarning,
            Self::StepError { .. } => EventKind::StepError,
            Self::JourneyInfo { .. } => EventKind::JourneyInfo,
            Self::JourneyWarning { .. } => EventKind::JourneyWarning,
            Self::JourneyError { .. } => EventKind::JourneyError,
        }
    }
}

#[cfg(test)]
mod tests;
